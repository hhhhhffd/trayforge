use quick_xml::escape::{escape, unescape};
use quick_xml::events::Event;
use quick_xml::Reader;
use serde_json::{Map, Value};

use super::{indent_option, parse_json, scalar_text, write_json, Layout};
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn formatter_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "xml-formatter",
            name: "XML Formatter",
            category: Category::Formats,
            description: "Format, validate, and beautify XML documents",
            keywords: vec!["xml", "format", "beautify", "minify", "validate"],
            icon: "file-code",
            input_placeholder: "Paste XML here...",
            output_placeholder: "Formatted XML will appear here",
            primary_action: "Format",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "mode",
                    label: "Mode",
                    help: Some("Pretty print or minify XML"),
                    kind: OptionKind::Segmented {
                        choices: vec!["Pretty", "Minify"],
                        default: 0,
                    },
                },
                indent_option(),
                ToolOption {
                    id: "preserveCdata",
                    label: "Preserve CDATA sections",
                    help: Some("Keep CDATA sections intact"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(formatter_process),
    }
}

pub fn xml_json_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "xml-json",
            name: "XML ↔ JSON",
            category: Category::Formats,
            description: "Convert between XML and JSON formats",
            keywords: vec!["xml", "json", "convert"],
            icon: "arrow-left-right",
            input_placeholder: "Paste XML or JSON here...",
            output_placeholder: "Converted output will appear here",
            primary_action: "To JSON",
            secondary_action: Some("To XML"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                ToolOption {
                    id: "attributePrefix",
                    label: "Attribute prefix",
                    help: Some("Prefix for XML attributes in JSON"),
                    // "@" is the default prefix.
                    kind: OptionKind::Picker {
                        choices: vec!["@", "_", "$", "attr_"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "textKey",
                    label: "Text content key",
                    help: Some("Key name for text content"),
                    kind: OptionKind::Picker {
                        choices: vec!["#text", "$value", "_text", "value"],
                        default: 0,
                    },
                },
            ],
        },
        process: Some(xml_json_process),
    }
}

enum Node {
    Element(Element),
    /// Text with XML escapes still in place.
    Text(String),
    CData(String),
    Comment(String),
    Declaration(String),
    ProcessingInstruction(String),
    DocType(String),
}

struct Element {
    name: String,
    /// Tag content as written: name plus attributes.
    head: String,
    attributes: Vec<(String, String)>,
    /// Written as `<a/>` rather than `<a></a>`.
    self_closing: bool,
    children: Vec<Node>,
}

/// Bounds the recursion of printing and of dropping the parsed tree.
const MAX_DEPTH: usize = 256;
/// Pretty printing stops indenting deeper levels so output stays linear in input size.
const MAX_INDENT_DEPTH: usize = 32;

fn xml_error(e: impl std::fmt::Display) -> ToolError {
    ToolError::InvalidInput(format!("XML: {e}"))
}

fn lossy(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}

fn element_from(
    e: &quick_xml::events::BytesStart,
    self_closing: bool,
) -> Result<Element, ToolError> {
    let mut attributes = Vec::new();
    for attr in e.attributes() {
        let attr = attr.map_err(xml_error)?;
        let value = attr.unescape_value().map_err(xml_error)?;
        attributes.push((lossy(attr.key.as_ref()), value.into_owned()));
    }
    Ok(Element {
        name: lossy(e.name().as_ref()),
        head: lossy(e),
        attributes,
        self_closing,
        children: Vec::new(),
    })
}

fn parse_xml(input: &str) -> Result<Vec<Node>, ToolError> {
    let mut reader = Reader::from_str(input);
    reader.config_mut().trim_text(true);
    let mut stack: Vec<Element> = Vec::new();
    let mut top: Vec<Node> = Vec::new();
    loop {
        let node = match reader.read_event().map_err(xml_error)? {
            Event::Eof => break,
            Event::Start(e) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(xml_error(format!(
                        "elements are nested deeper than {MAX_DEPTH} levels"
                    )));
                }
                stack.push(element_from(&e, false)?);
                continue;
            }
            Event::End(_) => {
                let done = stack
                    .pop()
                    .ok_or_else(|| xml_error("unexpected closing tag"))?;
                Node::Element(done)
            }
            Event::Empty(e) => Node::Element(element_from(&e, true)?),
            Event::Text(t) => Node::Text(lossy(&t)),
            Event::CData(c) => Node::CData(lossy(&c)),
            Event::Comment(c) => Node::Comment(lossy(&c)),
            Event::Decl(d) => Node::Declaration(lossy(&d)),
            Event::PI(p) => Node::ProcessingInstruction(lossy(&p)),
            Event::DocType(d) => Node::DocType(lossy(&d)),
        };
        match stack.last_mut() {
            Some(parent) => parent.children.push(node),
            None => top.push(node),
        }
    }
    if let Some(open) = stack.last() {
        return Err(xml_error(format!("unclosed element <{}>", open.name)));
    }
    Ok(top)
}

fn formatter_process(req: &Request) -> ToolResult {
    let nodes = parse_xml(req.input)?;
    if !nodes.iter().any(|n| matches!(n, Node::Element(_))) {
        return Err(xml_error("No root element found"));
    }
    let indent = (req.options.str("mode") == "Pretty").then(|| req.options.i64("indent") as usize);
    Ok(ToolOutput::text(print(
        &nodes,
        indent,
        req.options.bool("preserveCdata"),
    )))
}

/// Pretty (`Some(indent)`) puts each node on its own line, except elements that
/// contain only text, which stay on one line. Minify drops all layout whitespace.
fn print(nodes: &[Node], indent: Option<usize>, keep_cdata: bool) -> String {
    let mut lines = Vec::new();
    for node in nodes {
        node_lines(node, 0, keep_cdata, &mut lines);
    }
    match indent {
        Some(n) => lines
            .iter()
            .map(|(depth, text)| {
                format!("{}{text}", " ".repeat((*depth).min(MAX_INDENT_DEPTH) * n))
            })
            .collect::<Vec<_>>()
            .join("\n"),
        None => lines.into_iter().map(|(_, text)| text).collect(),
    }
}

fn inline_text(node: &Node, keep_cdata: bool) -> Option<String> {
    match node {
        Node::Text(t) => Some(t.clone()),
        Node::CData(c) if keep_cdata => Some(format!("<![CDATA[{c}]]>")),
        Node::CData(c) => Some(escape(c).into_owned()),
        _ => None,
    }
}

fn node_lines(node: &Node, depth: usize, keep_cdata: bool, out: &mut Vec<(usize, String)>) {
    match node {
        Node::Element(e) => {
            let texts: Option<Vec<String>> = e
                .children
                .iter()
                .map(|c| inline_text(c, keep_cdata))
                .collect();
            if e.children.is_empty() {
                let line = if e.self_closing {
                    format!("<{}/>", e.head)
                } else {
                    format!("<{}></{}>", e.head, e.name)
                };
                out.push((depth, line));
            } else if let Some(texts) = texts {
                out.push((
                    depth,
                    format!("<{}>{}</{}>", e.head, texts.concat(), e.name),
                ));
            } else {
                out.push((depth, format!("<{}>", e.head)));
                for child in &e.children {
                    node_lines(child, depth + 1, keep_cdata, out);
                }
                out.push((depth, format!("</{}>", e.name)));
            }
        }
        Node::Comment(c) => out.push((depth, format!("<!--{c}-->"))),
        Node::Declaration(d) => out.push((depth, format!("<?{d}?>"))),
        Node::ProcessingInstruction(p) => out.push((depth, format!("<?{p}?>"))),
        Node::DocType(d) => out.push((depth, format!("<!DOCTYPE {d}>"))),
        text => out.extend(inline_text(text, keep_cdata).map(|t| (depth, t))),
    }
}

fn xml_json_process(req: &Request) -> ToolResult {
    let indent = req.options.i64("indent") as usize;
    let prefix = req.options.str("attributePrefix");
    let text_key = req.options.str("textKey");
    if req.reverse {
        let value = parse_json(req.input)?;
        let root = match &value {
            Value::Object(map) if map.len() == 1 => {
                let (name, inner) = map.iter().next().expect("one entry");
                element_nodes(name, inner, prefix, text_key)?
            }
            other => element_nodes("root", other, prefix, text_key)?,
        };
        return Ok(ToolOutput::text(print(&root, Some(indent), true)));
    }

    let nodes = parse_xml(req.input)?;
    let root = nodes
        .iter()
        .find_map(|n| match n {
            Node::Element(e) => Some(e),
            _ => None,
        })
        .ok_or_else(|| ToolError::InvalidInput("No root element found".into()))?;
    let mut doc = Map::new();
    doc.insert(root.name.clone(), element_json(root, prefix, text_key)?);
    Ok(ToolOutput::text(write_json(
        &Value::Object(doc),
        Layout::Pretty(indent),
        false,
        false,
    )))
}

fn element_json(e: &Element, prefix: &str, text_key: &str) -> Result<Value, ToolError> {
    let mut map = Map::new();
    for (key, value) in &e.attributes {
        map.insert(format!("{prefix}{key}"), Value::String(value.clone()));
    }
    let mut text = String::new();
    for child in &e.children {
        match child {
            Node::Element(c) => {
                let value = element_json(c, prefix, text_key)?;
                match map.get_mut(&c.name) {
                    Some(Value::Array(items)) => items.push(value),
                    Some(existing) => *existing = Value::Array(vec![existing.take(), value]),
                    None => {
                        map.insert(c.name.clone(), value);
                    }
                }
            }
            Node::Text(t) => text.push_str(&unescape(t).map_err(xml_error)?),
            Node::CData(c) => text.push_str(c),
            _ => {}
        }
    }
    if map.is_empty() {
        return Ok(Value::String(text));
    }
    if !text.is_empty() {
        map.insert(text_key.to_string(), Value::String(text));
    }
    Ok(Value::Object(map))
}

fn is_xml_name(name: &str) -> bool {
    name.starts_with(|c: char| c.is_alphabetic() || c == '_')
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | ':'))
}

/// Builds the XML nodes for `name: value`; arrays become repeated elements.
fn element_nodes(
    name: &str,
    value: &Value,
    prefix: &str,
    text_key: &str,
) -> Result<Vec<Node>, ToolError> {
    if !is_xml_name(name) {
        return Err(ToolError::InvalidInput(format!(
            "Invalid XML element name '{name}'"
        )));
    }
    if let Value::Array(items) = value {
        let mut nodes = Vec::new();
        for item in items {
            nodes.extend(element_nodes(name, item, prefix, text_key)?);
        }
        return Ok(nodes);
    }
    let mut head = name.to_string();
    let mut children = Vec::new();
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if let Some(attr) = key.strip_prefix(prefix).filter(|a| !a.is_empty()) {
                    if !is_xml_name(attr) {
                        return Err(ToolError::InvalidInput(format!(
                            "Invalid XML attribute name '{attr}'"
                        )));
                    }
                    head.push_str(&format!(" {attr}=\"{}\"", escape(scalar_text(inner))));
                } else if key == text_key {
                    children.push(Node::Text(escape(scalar_text(inner)).into_owned()));
                } else {
                    children.extend(element_nodes(key, inner, prefix, text_key)?);
                }
            }
        }
        Value::Null => {}
        scalar => children.push(Node::Text(escape(scalar_text(scalar)).into_owned())),
    }
    Ok(vec![Node::Element(Element {
        name: name.to_string(),
        head,
        attributes: Vec::new(),
        self_closing: children.is_empty(),
        children,
    })])
}
