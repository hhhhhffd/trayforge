use serde::Deserialize;
use serde_json::Value;

use super::{indent_option, parse_json, write_json, write_string, Layout};
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn json_yaml_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "json-yaml",
            name: "JSON ↔ YAML",
            category: Category::Formats,
            description: "Convert between JSON and YAML formats",
            keywords: vec!["json", "yaml", "yml", "convert"],
            icon: "arrow-left-right",
            input_placeholder: "Paste JSON or YAML here...",
            output_placeholder: "Converted output will appear here",
            primary_action: "To YAML",
            secondary_action: Some("To JSON"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                ToolOption {
                    id: "sortKeys",
                    label: "Sort keys",
                    help: Some("Sort keys alphabetically"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(json_yaml_process),
    }
}

pub fn formatter_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "yaml-formatter",
            name: "YAML Formatter",
            category: Category::Formats,
            description: "Format, validate, and beautify YAML documents",
            keywords: vec!["yaml", "yml", "format", "beautify", "validate"],
            icon: "file-code",
            input_placeholder: "Paste YAML here...",
            output_placeholder: "Formatted YAML will appear here",
            primary_action: "Format",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                ToolOption {
                    id: "sortKeys",
                    label: "Sort keys",
                    help: Some("Sort object keys alphabetically"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "documentStart",
                    label: "Add document start (---)",
                    help: Some("Add YAML document start marker"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(formatter_process),
    }
}

fn json_yaml_process(req: &Request) -> ToolResult {
    let indent = req.options.i64("indent") as usize;
    let sort = req.options.bool("sortKeys");
    if req.reverse {
        let yaml_error =
            |e: serde_yaml::Error| ToolError::InvalidInput(format!("Invalid YAML: {e}"));
        // JSON cannot hold .inf/.nan and serde_json would turn them into null.
        let parsed: serde_yaml::Value = serde_yaml::from_str(req.input).map_err(yaml_error)?;
        if has_non_finite_float(&parsed) {
            return Err(ToolError::InvalidInput(
                "YAML contains .inf or .nan, which JSON cannot represent".into(),
            ));
        }
        let value: Value = serde_yaml::from_str(req.input).map_err(yaml_error)?;
        if value.is_null() {
            return Err(ToolError::ProcessingFailed(
                "YAML parsed to nil (document may be empty)".into(),
            ));
        }
        return Ok(ToolOutput::text(write_json(
            &value,
            Layout::Pretty(indent),
            sort,
            false,
        )));
    }
    // The whole document is emitted one level deep, preceded by a
    // blank line, with list items indented under their key.
    let style = Style {
        indent,
        sort,
        root_col: indent,
        seq_indented: true,
        single_quotes: false,
        strict_chars: true,
    };
    Ok(ToolOutput::text(emit(&parse_json(req.input)?, &style)))
}

fn has_non_finite_float(v: &serde_yaml::Value) -> bool {
    use serde_yaml::Value::{Mapping, Number, Sequence, Tagged};
    match v {
        Number(n) => n.as_f64().is_some_and(|f| !f.is_finite()),
        Sequence(items) => items.iter().any(has_non_finite_float),
        Mapping(map) => map
            .iter()
            .any(|(k, v)| has_non_finite_float(k) || has_non_finite_float(v)),
        Tagged(tagged) => has_non_finite_float(&tagged.value),
        _ => false,
    }
}

fn formatter_process(req: &Request) -> ToolResult {
    let style = Style {
        indent: req.options.i64("indent") as usize,
        sort: req.options.bool("sortKeys"),
        root_col: 0,
        seq_indented: false,
        single_quotes: true,
        strict_chars: false,
    };
    let documents = serde_yaml::Deserializer::from_str(req.input)
        .map(|doc| {
            Value::deserialize(doc)
                .map_err(|e| ToolError::InvalidInput(format!("Invalid YAML: {e}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = String::new();
    for (i, doc) in documents.iter().enumerate() {
        if i > 0 || req.options.bool("documentStart") {
            out.push_str("---\n");
        }
        out.push_str(&emit(doc, &style));
    }
    Ok(ToolOutput::text(out))
}

struct Style {
    indent: usize,
    sort: bool,
    /// Column of the top-level block; a non-zero value also adds a leading blank line.
    root_col: usize,
    /// Sequences under a mapping key are indented one level (otherwise flush with the key).
    seq_indented: bool,
    single_quotes: bool,
    /// Also quote strings containing any YAML indicator character.
    strict_chars: bool,
}

fn emit(v: &Value, st: &Style) -> String {
    let mut out = String::new();
    if is_block(v) {
        if st.root_col > 0 {
            out.push('\n');
        }
        block(v, st.root_col, st, &mut out);
    } else {
        out.push_str(&inline(v, st));
        out.push('\n');
    }
    out
}

fn is_block(v: &Value) -> bool {
    match v {
        Value::Array(a) => !a.is_empty(),
        Value::Object(m) => !m.is_empty(),
        _ => false,
    }
}

fn block(v: &Value, col: usize, st: &Style, out: &mut String) {
    let pad = " ".repeat(col);
    match v {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            if st.sort {
                entries.sort_by(|a, b| a.0.cmp(b.0));
            }
            for (key, val) in entries {
                out.push_str(&pad);
                out.push_str(&yaml_string(key, st));
                if is_block(val) {
                    out.push_str(":\n");
                    let nested = match val {
                        Value::Array(_) if !st.seq_indented => col,
                        _ => col + st.indent,
                    };
                    block(val, nested, st, out);
                } else {
                    out.push_str(": ");
                    out.push_str(&inline(val, st));
                    out.push('\n');
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                out.push_str(&pad);
                out.push_str("- ");
                if is_block(item) {
                    // The first line of the nested block continues the dash line.
                    let mut nested = String::new();
                    block(item, col + 2, st, &mut nested);
                    out.push_str(&nested[col + 2..]);
                } else {
                    out.push_str(&inline(item, st));
                    out.push('\n');
                }
            }
        }
        scalar => {
            out.push_str(&pad);
            out.push_str(&inline(scalar, st));
            out.push('\n');
        }
    }
}

fn inline(v: &Value, st: &Style) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => super::number_text(n),
        Value::String(s) => yaml_string(s, st),
        Value::Array(_) => "[]".into(),
        Value::Object(_) => "{}".into(),
    }
}

fn yaml_string(s: &str, st: &Style) -> String {
    if !needs_quotes(s, st.strict_chars) {
        return s.to_string();
    }
    if st.single_quotes && !s.chars().any(char::is_control) {
        return format!("'{}'", s.replace('\'', "''"));
    }
    let mut quoted = String::new();
    write_string(s, false, &mut quoted);
    quoted
}

/// True when a plain scalar would be misread: empty, padded, containing control
/// characters or indicators, or resolving to another type (null, bool, number, date).
fn needs_quotes(s: &str, strict: bool) -> bool {
    let lower = s.to_ascii_lowercase();
    s.is_empty()
        || s.trim() != s
        || s.chars().any(char::is_control)
        || s.starts_with([
            '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|',
        ])
        || s.starts_with(['>', '\'', '"', '%', '@', '`'])
        || s.contains(": ")
        || s.contains(" #")
        || s.ends_with(':')
        || (strict && s.contains([':', '!', '#', '&', '*', '?', '|', '>', '\'', '{', '[', ',']))
        || (strict && s.contains(['}', ']', '@', '`']))
        || matches!(
            lower.as_str(),
            "null"
                | "~"
                | "true"
                | "false"
                | "yes"
                | "no"
                | "on"
                | "off"
                | "y"
                | "n"
                | ".inf"
                | "+.inf"
                | "-.inf"
                | ".nan"
        )
        || s.parse::<f64>().is_ok()
        || (s.starts_with(|c: char| c.is_ascii_digit())
            && s.chars()
                .all(|c| c.is_ascii_hexdigit() || "_.:-+xXoO".contains(c)))
}
