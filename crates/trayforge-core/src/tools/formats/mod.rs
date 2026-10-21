//! Formats category: JSON, YAML, XML, CSV, SQL, GraphQL and TOML tools.

mod graphql;
mod json;
mod sql;
mod tabular;
mod toml_tool;
mod xml;
mod yaml;

use serde_json::Value;

use crate::{OptionKind, Tool, ToolError, ToolOption};

pub fn tools() -> Vec<Tool> {
    vec![
        json::formatter_tool(),
        json::path_tool(),
        yaml::json_yaml_tool(),
        yaml::formatter_tool(),
        xml::formatter_tool(),
        xml::xml_json_tool(),
        tabular::csv_to_json_tool(),
        tabular::json_to_csv_tool(),
        sql::tool(),
        graphql::tool(),
        toml_tool::tool(),
    ]
}

fn indent_option() -> ToolOption {
    ToolOption {
        id: "indent",
        label: "Indent Spaces",
        help: Some("Number of spaces for indentation"),
        kind: OptionKind::Stepper {
            min: 1,
            max: 8,
            default: 2,
        },
    }
}

/// Parses JSON and reports failures with a line/column location.
fn parse_json(input: &str) -> Result<Value, ToolError> {
    serde_json::from_str(input).map_err(|e| {
        let place = format!(
            "around line {}, column {}. Check your input and try again",
            e.line(),
            e.column().saturating_sub(1)
        );
        ToolError::InvalidInput(if e.is_eof() {
            format!("Unexpected end of JSON input {place}")
        } else {
            format!("Invalid value {place}")
        })
    })
}

#[derive(Clone, Copy)]
enum Layout {
    /// Foundation style: `"key" : value`, one item per line.
    Pretty(usize),
    Minify,
    /// Single line with spaces after `:` and `,`.
    Compact,
}

/// Serializes JSON with a ` : ` separator, whole floats
/// without a fraction, slashes unescaped, keys in document order unless sorted.
fn write_json(value: &Value, layout: Layout, sort_keys: bool, escape_unicode: bool) -> String {
    let mut out = String::new();
    write_value(value, layout, sort_keys, escape_unicode, 0, &mut out);
    out
}

fn write_value(v: &Value, layout: Layout, sort: bool, esc: bool, depth: usize, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number_text(n)),
        Value::String(s) => write_string(s, esc, out),
        Value::Array(items) if items.is_empty() => out.push_str("[]"),
        Value::Object(map) if map.is_empty() => out.push_str("{}"),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                separator(layout, i == 0, depth + 1, out);
                write_value(item, layout, sort, esc, depth + 1, out);
            }
            close(layout, ']', depth, out);
        }
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            if sort {
                entries.sort_by(|a, b| a.0.cmp(b.0));
            }
            out.push('{');
            for (i, (key, item)) in entries.into_iter().enumerate() {
                separator(layout, i == 0, depth + 1, out);
                write_string(key, esc, out);
                out.push_str(match layout {
                    Layout::Pretty(_) => " : ",
                    Layout::Minify => ":",
                    Layout::Compact => ": ",
                });
                write_value(item, layout, sort, esc, depth + 1, out);
            }
            close(layout, '}', depth, out);
        }
    }
}

fn separator(layout: Layout, first: bool, depth: usize, out: &mut String) {
    if !first {
        out.push(',');
    }
    match layout {
        Layout::Pretty(n) => {
            out.push('\n');
            out.push_str(&" ".repeat(n * depth));
        }
        Layout::Compact if !first => out.push(' '),
        _ => {}
    }
}

fn close(layout: Layout, bracket: char, depth: usize, out: &mut String) {
    if let Layout::Pretty(n) = layout {
        out.push('\n');
        out.push_str(&" ".repeat(n * depth));
    }
    out.push(bracket);
}

fn number_text(n: &serde_json::Number) -> String {
    match n.as_f64() {
        Some(f) if n.is_f64() && f.fract() == 0.0 && f.abs() < 1e15 => format!("{}", f as i64),
        _ => n.to_string(),
    }
}

fn write_string(s: &str, escape_unicode: bool, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 || (escape_unicode && !c.is_ascii()) => {
                for unit in c.encode_utf16(&mut [0; 2]) {
                    out.push_str(&format!("\\u{unit:04x}"));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Renders a value for plain-text outputs (CSV cells, XML text).
fn scalar_text(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::String(s) => s.clone(),
        Value::Number(n) => number_text(n),
        other => write_json(other, Layout::Minify, false, false),
    }
}
