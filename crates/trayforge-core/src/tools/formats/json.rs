use serde_json::Value;

use super::{indent_option, parse_json, scalar_text, write_json, Layout};
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn formatter_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "json-formatter",
            name: "JSON Formatter",
            category: Category::Formats,
            description: "Format, validate, and minify JSON with syntax highlighting",
            keywords: vec!["json", "format", "pretty", "minify", "validate", "jsonc"],
            icon: "braces",
            input_placeholder: "Paste JSON here...",
            output_placeholder: "Formatted JSON will appear here",
            primary_action: "Format",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "mode",
                    label: "Mode",
                    help: Some("Pretty print or minify JSON"),
                    kind: OptionKind::Picker {
                        choices: vec!["Pretty", "Minify", "Compact"],
                        default: 0,
                    },
                },
                indent_option(),
                ToolOption {
                    id: "sortKeys",
                    label: "Sort keys alphabetically",
                    help: Some("Sort object keys in alphabetical order"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "escapeUnicode",
                    label: "Escape Unicode",
                    help: Some("Escape non-ASCII characters"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "stripComments",
                    label: "Strip comments (JSONC)",
                    help: Some("Remove // and /* */ comments"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(format_process),
    }
}

fn format_process(req: &Request) -> ToolResult {
    let value = if req.options.bool("stripComments") {
        parse_json(&strip_comments(req.input))?
    } else {
        parse_json(req.input)?
    };
    let layout = match req.options.str("mode") {
        "Minify" => Layout::Minify,
        "Compact" => Layout::Compact,
        _ => Layout::Pretty(req.options.i64("indent") as usize),
    };
    Ok(ToolOutput::text(write_json(
        &value,
        layout,
        req.options.bool("sortKeys"),
        req.options.bool("escapeUnicode"),
    )))
}

/// Removes `//` and `/* */` comments outside string literals.
fn strip_comments(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' => {
                out.push(c);
                while let Some(s) = chars.next() {
                    out.push(s);
                    if s == '\\' {
                        out.extend(chars.next());
                    } else if s == '"' {
                        break;
                    }
                }
            }
            '/' if chars.peek() == Some(&'/') => {
                // Keep the newline so line numbers in errors stay meaningful.
                while chars.next_if(|n| *n != '\n').is_some() {}
            }
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                let mut prev = '\0';
                for n in chars.by_ref() {
                    if prev == '*' && n == '/' {
                        break;
                    }
                    if n == '\n' {
                        out.push('\n');
                    }
                    prev = n;
                }
            }
            c => out.push(c),
        }
    }
    out
}

pub fn path_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "json-path",
            name: "JSON Path Query",
            category: Category::Formats,
            description: "Query JSON data using JSONPath expressions",
            keywords: vec!["json", "jsonpath", "query", "path", "filter"],
            icon: "search",
            input_placeholder: "Paste JSON here...",
            output_placeholder: "Query results will appear here",
            primary_action: "Query",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "path",
                    label: "JSONPath Expression",
                    help: Some("JSONPath expression (e.g., $.key, $[0], $.items[*].name)"),
                    kind: OptionKind::TextField {
                        placeholder: "$.store.books[0].title",
                        default: "$",
                    },
                },
                ToolOption {
                    id: "pretty",
                    label: "Pretty print output",
                    help: Some("Format output JSON with indentation"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "extract",
                    label: "Extract primitive values",
                    help: Some("Extract raw values for strings/numbers instead of JSON"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(path_process),
    }
}

enum Selector {
    Key(String),
    Index(i64),
    Wildcard,
    Slice(Option<i64>, Option<i64>),
}

struct Step {
    recursive: bool,
    selector: Selector,
}

fn path_process(req: &Request) -> ToolResult {
    let root: Value = serde_json::from_str(req.input)
        .map_err(|_| ToolError::InvalidInput("Invalid JSON input".into()))?;
    let steps = parse_path(req.options.str("path").trim())
        .map_err(|m| ToolError::InvalidInput(format!("Invalid JSONPath expression: {m}")))?;
    let multi = steps
        .iter()
        .any(|s| s.recursive || !matches!(s.selector, Selector::Key(_) | Selector::Index(_)));
    let matches = evaluate(&root, &steps, multi)
        .map_err(|m| ToolError::ProcessingFailed(format!("Failed to execute query: {m}")))?;
    if matches.is_empty() {
        return Err(ToolError::ProcessingFailed("No matches found".into()));
    }

    if req.options.bool("extract") {
        let lines: Vec<String> = matches
            .iter()
            .map(|v| match v {
                Value::Array(_) | Value::Object(_) => write_json(v, Layout::Minify, false, false),
                Value::Null => "null".into(),
                other => scalar_text(other),
            })
            .collect();
        return Ok(ToolOutput::text(lines.join("\n")));
    }
    let layout = if req.options.bool("pretty") {
        Layout::Pretty(2)
    } else {
        Layout::Minify
    };
    let result = if multi {
        Value::Array(matches.into_iter().cloned().collect())
    } else {
        matches[0].clone()
    };
    Ok(ToolOutput::text(write_json(&result, layout, false, false)))
}

fn parse_path(path: &str) -> Result<Vec<Step>, String> {
    let rest = path.strip_prefix('$').ok_or("JSONPath must start with $")?;
    let chars: Vec<char> = rest.chars().collect();
    let mut steps = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let recursive = chars[i] == '.' && chars.get(i + 1) == Some(&'.');
        match chars[i] {
            '.' => {
                i += if recursive { 2 } else { 1 };
                if chars.get(i) == Some(&'[') {
                    continue; // `..[` is handled by the bracket branch below
                }
                let start = i;
                while i < chars.len() && chars[i] != '.' && chars[i] != '[' {
                    i += 1;
                }
                let name: String = chars[start..i].iter().collect();
                let selector = match name.as_str() {
                    "" => {
                        return Err(match chars.get(start) {
                            Some(c) => format!("Unexpected character: {c}"),
                            None => "Missing key name".into(),
                        })
                    }
                    "*" => Selector::Wildcard,
                    _ => Selector::Key(name),
                };
                steps.push(Step {
                    recursive,
                    selector,
                });
            }
            '[' => {
                let close = chars[i..]
                    .iter()
                    .position(|c| *c == ']')
                    .ok_or("Missing closing bracket")?;
                let inner: String = chars[i + 1..i + close].iter().collect();
                steps.push(Step {
                    recursive: i >= 2 && chars[i - 1] == '.' && chars[i - 2] == '.',
                    selector: parse_bracket(inner.trim())?,
                });
                i += close + 1;
            }
            c => return Err(format!("Unexpected character: {c}")),
        }
    }
    Ok(steps)
}

fn parse_bracket(inner: &str) -> Result<Selector, String> {
    let invalid = || format!("Invalid bracket content: [{inner}]");
    if inner == "*" {
        return Ok(Selector::Wildcard);
    }
    if let Ok(n) = inner.parse::<i64>() {
        return Ok(Selector::Index(n));
    }
    for quote in ['\'', '"'] {
        if let Some(key) = inner
            .strip_prefix(quote)
            .and_then(|s| s.strip_suffix(quote))
        {
            return Ok(Selector::Key(key.to_string()));
        }
    }
    if let Some((from, to)) = inner.split_once(':') {
        let bound = |s: &str| -> Result<Option<i64>, String> {
            match s.trim() {
                "" => Ok(None),
                n => n.parse().map(Some).map_err(|_| invalid()),
            }
        };
        return Ok(Selector::Slice(bound(from)?, bound(to)?));
    }
    Err(invalid())
}

/// Upper bound on nodes held at any step; `$..*..*..*` otherwise grows without limit.
const MAX_MATCHES: usize = 100_000;

fn too_many_matches() -> String {
    format!("Too many matches (more than {MAX_MATCHES}); narrow the path")
}

fn evaluate<'a>(root: &'a Value, steps: &[Step], multi: bool) -> Result<Vec<&'a Value>, String> {
    let mut nodes = vec![root];
    for step in steps {
        let mut candidates = Vec::new();
        for node in nodes {
            candidates.push(node);
            if step.recursive {
                collect_descendants(node, &mut candidates)?;
            }
        }
        let mut next = Vec::new();
        for candidate in candidates {
            select(candidate, &step.selector, multi, &mut next)?;
            if next.len() > MAX_MATCHES {
                return Err(too_many_matches());
            }
        }
        nodes = next;
    }
    Ok(nodes)
}

fn collect_descendants<'a>(v: &'a Value, out: &mut Vec<&'a Value>) -> Result<(), String> {
    let children: Vec<&Value> = match v {
        Value::Array(items) => items.iter().collect(),
        Value::Object(map) => map.values().collect(),
        _ => return Ok(()),
    };
    for child in children {
        out.push(child);
        if out.len() > MAX_MATCHES {
            return Err(too_many_matches());
        }
        collect_descendants(child, out)?;
    }
    Ok(())
}

/// Strict (errors on a miss) for plain paths; lenient (skips) for multi-match paths.
fn select<'a>(
    node: &'a Value,
    selector: &Selector,
    lenient: bool,
    out: &mut Vec<&'a Value>,
) -> Result<(), String> {
    match (selector, node) {
        (Selector::Key(key), Value::Object(map)) => match map.get(key) {
            Some(v) => out.push(v),
            None if lenient => {}
            None => return Err(format!("Key '{key}' not found in object")),
        },
        (Selector::Key(key), _) if !lenient => {
            return Err(format!("Key '{key}' not found in object"))
        }
        (Selector::Index(n), Value::Array(items)) => {
            let idx = if *n < 0 { *n + items.len() as i64 } else { *n };
            match usize::try_from(idx).ok().and_then(|i| items.get(i)) {
                Some(v) => out.push(v),
                None if lenient => {}
                None => {
                    return Err(format!(
                        "Index {n} out of bounds (array has {} elements)",
                        items.len()
                    ))
                }
            }
        }
        (Selector::Index(n), _) if !lenient => {
            return Err(format!("Index {n} out of bounds (value is not an array)"))
        }
        (Selector::Wildcard, Value::Array(items)) => out.extend(items),
        (Selector::Wildcard, Value::Object(map)) => out.extend(map.values()),
        (Selector::Slice(from, to), Value::Array(items)) => {
            let len = items.len() as i64;
            let norm = |b: i64| if b < 0 { (b + len).max(0) } else { b.min(len) };
            let (start, end) = (from.map_or(0, norm), to.map_or(len, norm));
            if start < end {
                out.extend(&items[start as usize..end as usize]);
            }
        }
        _ => {}
    }
    Ok(())
}
