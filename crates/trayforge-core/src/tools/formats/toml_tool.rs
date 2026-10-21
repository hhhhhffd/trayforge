use toml::{Table, Value};

use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "toml-parser",
            name: "TOML Parser",
            category: Category::Formats,
            description: "Parse, validate, and format TOML files",
            keywords: vec!["toml", "parse", "format", "validate", "config"],
            icon: "file-cog",
            input_placeholder: "Paste TOML here...",
            output_placeholder: "Parsed/formatted TOML will appear here",
            primary_action: "Format",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "sortKeys",
                    label: "Sort keys",
                    help: Some("Sort keys alphabetically"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "inlineTables",
                    label: "Inline simple tables",
                    help: Some("Use inline format for simple tables"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(process),
    }
}

fn process(req: &Request) -> ToolResult {
    let mut table: Table = req.input.parse().map_err(|e: toml::de::Error| {
        let line = e
            .span()
            .map_or(0, |s| req.input[..s.start].matches('\n').count() + 1);
        ToolError::InvalidInput(format!("{} (line {line})", e.message()))
    })?;
    if table.is_empty() {
        return Err(ToolError::InvalidInput(
            "(no valid key-value pairs found)".into(),
        ));
    }
    if req.options.bool("sortKeys") {
        table = sorted(table);
    }
    let mut sections = Vec::new();
    emit(
        &table,
        &[],
        None,
        req.options.bool("inlineTables"),
        &mut sections,
    );
    Ok(ToolOutput::text(format!("{}\n", sections.join("\n\n"))))
}

fn sorted(table: Table) -> Table {
    let mut entries: Vec<_> = table.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
        .into_iter()
        .map(|(k, v)| {
            let v = match v {
                Value::Table(t) => Value::Table(sorted(t)),
                Value::Array(items) => Value::Array(
                    items
                        .into_iter()
                        .map(|i| match i {
                            Value::Table(t) => Value::Table(sorted(t)),
                            other => other,
                        })
                        .collect(),
                ),
                other => other,
            };
            (k, v)
        })
        .collect()
}

fn key_text(key: &str) -> String {
    if !key.is_empty()
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        key.to_string()
    } else {
        Value::String(key.to_string()).to_string()
    }
}

/// Only plain values inside: no sub-tables and no arrays of tables.
fn is_simple(table: &Table) -> bool {
    table.values().all(|v| match v {
        Value::Table(_) => false,
        Value::Array(items) => !items.iter().any(Value::is_table),
        _ => true,
    })
}

fn table_array(v: &Value) -> Option<&[Value]> {
    match v {
        Value::Array(items) if !items.is_empty() && items.iter().all(Value::is_table) => {
            Some(items)
        }
        _ => None,
    }
}

/// Appends one section per table (`[a.b]`, `[[a.b]]`) in document order.
fn emit(
    table: &Table,
    path: &[String],
    header: Option<String>,
    inline: bool,
    sections: &mut Vec<String>,
) {
    let mut lines = Vec::new();
    let mut subtables = Vec::new();
    for (key, value) in table {
        let inlined = |t: &Table| inline && is_simple(t);
        match value {
            Value::Table(sub) if !inlined(sub) => subtables.push((key, value)),
            Value::Array(_)
                if table_array(value).is_some_and(|items| {
                    !items.iter().all(|i| i.as_table().is_some_and(inlined))
                }) =>
            {
                subtables.push((key, value));
            }
            other => lines.push(format!("{} = {other}", key_text(key))),
        }
    }

    let is_array_item = header.as_deref().is_some_and(|h| h.starts_with("[["));
    if let Some(header) = header {
        if !lines.is_empty() || is_array_item || subtables.is_empty() {
            sections.push(
                std::iter::once(header)
                    .chain(lines)
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
    } else if !lines.is_empty() {
        sections.push(lines.join("\n"));
    }

    for (key, value) in subtables {
        let mut sub_path = path.to_vec();
        sub_path.push(key_text(key));
        let dotted = sub_path.join(".");
        match value {
            Value::Table(sub) => emit(
                sub,
                &sub_path,
                Some(format!("[{dotted}]")),
                inline,
                sections,
            ),
            other => {
                for item in table_array(other).unwrap_or_default() {
                    if let Some(sub) = item.as_table() {
                        emit(
                            sub,
                            &sub_path,
                            Some(format!("[[{dotted}]]")),
                            inline,
                            sections,
                        );
                    }
                }
            }
        }
    }
}
