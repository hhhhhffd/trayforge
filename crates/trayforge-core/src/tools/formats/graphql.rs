use super::indent_option;
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "graphql-formatter",
            name: "GraphQL Formatter",
            category: Category::Formats,
            description: "Format and beautify GraphQL queries and mutations",
            keywords: vec!["graphql", "gql", "query", "format", "beautify"],
            icon: "workflow",
            input_placeholder: "Paste GraphQL query here...",
            output_placeholder: "Formatted GraphQL will appear here",
            primary_action: "Format",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                indent_option(),
                ToolOption {
                    id: "sortFields",
                    label: "Sort fields alphabetically",
                    help: Some("Sort fields within selections"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(process),
    }
}

struct Node {
    head: String,
    children: Option<Vec<Node>>,
}

/// Layout rule: every whitespace-separated token outside
/// parentheses gets its own line, so `query Q {` becomes `query` / `Q {`.
/// Tokens that obviously belong together (`... on T`, `alias: field`) are kept joined.
fn process(req: &Request) -> ToolResult {
    let chars: Vec<char> = req.input.chars().collect();
    let mut i = 0;
    let mut nodes = parse_block(&chars, &mut i, 0)?;
    if req.options.bool("sortFields") {
        sort_selections(&mut nodes, true);
    }
    let mut lines = Vec::new();
    print(&nodes, 0, req.options.i64("indent") as usize, &mut lines);
    Ok(ToolOutput::text(lines.join("\n")))
}

fn push_token(nodes: &mut Vec<Node>, token: &mut String) {
    if token.is_empty() {
        return;
    }
    match nodes.last_mut() {
        Some(last)
            if last.children.is_none()
                && (matches!(last.head.as_str(), "..." | "... on") || last.head.ends_with(':')) =>
        {
            last.head.push(' ');
            last.head.push_str(token);
        }
        _ => nodes.push(Node {
            head: std::mem::take(token),
            children: None,
        }),
    }
    token.clear();
}

/// Bounds the recursion of parsing, printing and dropping the tree.
const MAX_DEPTH: usize = 256;
/// Printing stops indenting deeper levels so output stays linear in input size.
const MAX_INDENT_DEPTH: usize = 32;

/// `depth` is the number of enclosing `{ }` blocks; 0 is the document level.
fn parse_block(chars: &[char], i: &mut usize, depth: usize) -> Result<Vec<Node>, ToolError> {
    let top = depth == 0;
    let mut nodes = Vec::new();
    let mut token = String::new();
    let mut parens = 0usize;
    while *i < chars.len() {
        let c = chars[*i];
        if c == '"' {
            // String literal, possibly a """block string""", copied verbatim.
            let quote = if chars[*i..].starts_with(&['"', '"', '"']) {
                3
            } else {
                1
            };
            token.extend(std::iter::repeat_n('"', quote));
            *i += quote;
            while *i < chars.len() {
                let s = chars[*i];
                token.push(s);
                *i += 1;
                if s == '\\' && quote == 1 {
                    token.extend(chars.get(*i));
                    *i += 1;
                } else if s == '"' && (quote == 1 || chars[*i - 1..].starts_with(&['"', '"', '"']))
                {
                    if quote == 3 {
                        token.push_str("\"\"");
                        *i += 2;
                    }
                    break;
                }
            }
            continue;
        }
        *i += 1;
        if parens > 0 {
            parens += usize::from(c == '(');
            parens -= usize::from(c == ')');
            token.push(c);
            continue;
        }
        match c {
            '(' => {
                parens = 1;
                token.push(c);
            }
            '#' => {
                push_token(&mut nodes, &mut token);
                let end = chars[*i..]
                    .iter()
                    .position(|c| *c == '\n')
                    .map_or(chars.len(), |p| *i + p);
                let comment: String = std::iter::once('#')
                    .chain(chars[*i..end].iter().copied())
                    .collect();
                nodes.push(Node {
                    head: comment.trim_end().to_string(),
                    children: None,
                });
                *i = end;
            }
            '{' => {
                if depth >= MAX_DEPTH {
                    return Err(ToolError::InvalidInput(format!(
                        "Selection sets are nested deeper than {MAX_DEPTH} levels"
                    )));
                }
                let children = parse_block(chars, i, depth + 1)?;
                if token.is_empty() {
                    match nodes.last_mut() {
                        Some(last) if last.children.is_none() && !last.head.starts_with('#') => {
                            last.children = Some(children);
                        }
                        _ => nodes.push(Node {
                            head: String::new(),
                            children: Some(children),
                        }),
                    }
                } else {
                    nodes.push(Node {
                        head: std::mem::take(&mut token),
                        children: Some(children),
                    });
                }
            }
            '}' => {
                if top {
                    return Err(ToolError::InvalidInput("Unexpected '}'".into()));
                }
                push_token(&mut nodes, &mut token);
                return Ok(nodes);
            }
            c if c.is_whitespace() || c == ',' => push_token(&mut nodes, &mut token),
            c => token.push(c),
        }
    }
    if !top {
        return Err(ToolError::InvalidInput("Missing closing '}'".into()));
    }
    push_token(&mut nodes, &mut token);
    Ok(nodes)
}

/// Sorts the fields inside selection sets; top-level definitions keep their order.
fn sort_selections(nodes: &mut [Node], top: bool) {
    if !top {
        nodes.sort_by(|a, b| a.head.cmp(&b.head));
    }
    for node in nodes {
        if let Some(children) = &mut node.children {
            sort_selections(children, false);
        }
    }
}

fn print(nodes: &[Node], depth: usize, indent: usize, out: &mut Vec<String>) {
    let pad = " ".repeat(depth.min(MAX_INDENT_DEPTH) * indent);
    for node in nodes {
        match &node.children {
            Some(children) => {
                let head = if node.head.is_empty() {
                    "{".to_string()
                } else {
                    format!("{} {{", node.head)
                };
                out.push(format!("{pad}{head}"));
                print(children, depth + 1, indent, out);
                out.push(format!("{pad}}}"));
            }
            None => out.push(format!("{pad}{}", node.head)),
        }
    }
}
