use std::collections::BTreeMap;

use serde_json::{Map, Number, Value};

use super::{parse_json, scalar_text, write_json, Layout};
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn csv_to_json_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "csv-to-json",
            name: "CSV → JSON",
            category: Category::Formats,
            description: "Convert CSV data to JSON format",
            keywords: vec!["csv", "json", "convert", "table", "spreadsheet"],
            icon: "table",
            input_placeholder: "Paste CSV data here...",
            output_placeholder: "JSON output will appear here",
            primary_action: "Convert",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "delimiter",
                    label: "Delimiter",
                    help: Some("Field separator character"),
                    kind: OptionKind::Picker {
                        choices: vec![
                            "Auto-detect",
                            "Comma (,)",
                            "Semicolon (;)",
                            "Tab",
                            "Pipe (|)",
                        ],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "hasHeaders",
                    label: "First row is headers",
                    help: Some("Use first row as property names"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "parseNumbers",
                    label: "Parse numbers",
                    help: Some("Convert numeric strings to numbers"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "trim",
                    label: "Trim whitespace",
                    help: Some("Remove leading/trailing spaces"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "format",
                    label: "Output Format",
                    help: Some("JSON structure format"),
                    kind: OptionKind::Segmented {
                        choices: vec!["Array of Objects", "Array of Arrays"],
                        default: 0,
                    },
                },
            ],
        },
        process: Some(csv_to_json_process),
    }
}

pub fn json_to_csv_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "json-to-csv",
            name: "JSON → CSV",
            category: Category::Formats,
            description: "Convert JSON arrays to CSV format",
            keywords: vec!["json", "csv", "convert", "table", "flatten"],
            icon: "table",
            input_placeholder: "Paste JSON array here...",
            output_placeholder: "CSV output will appear here",
            primary_action: "Convert",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "delimiter",
                    label: "Delimiter",
                    help: Some("Field separator character"),
                    kind: OptionKind::Picker {
                        choices: vec!["Comma (,)", "Semicolon (;)", "Tab", "Pipe (|)"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "includeHeaders",
                    label: "Include headers",
                    help: Some("Add column headers row"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "flatten",
                    label: "Flatten nested objects",
                    help: Some("How deep to flatten nested objects"),
                    kind: OptionKind::Picker {
                        choices: vec!["None", "1 level", "2 levels", "All"],
                        default: 1,
                    },
                },
                ToolOption {
                    id: "arrays",
                    label: "Array values",
                    help: Some("How to handle array values"),
                    // "Join with ;" is the default.
                    kind: OptionKind::Picker {
                        choices: vec!["Join with ;", "JSON string", "Index columns"],
                        default: 0,
                    },
                },
            ],
        },
        process: Some(json_to_csv_process),
    }
}

fn delimiter_byte(choice: &str) -> u8 {
    match choice {
        "Semicolon (;)" => b';',
        "Tab" => b'\t',
        "Pipe (|)" => b'|',
        _ => b',',
    }
}

/// Picks the candidate that occurs most often (outside quotes) in the first line.
fn detect_delimiter(input: &str) -> u8 {
    let line = input.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let mut in_quotes = false;
    let mut counts = [0usize; 4];
    for c in line.chars() {
        match c {
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => counts[0] += 1,
            ';' if !in_quotes => counts[1] += 1,
            '\t' if !in_quotes => counts[2] += 1,
            '|' if !in_quotes => counts[3] += 1,
            _ => {}
        }
    }
    let best = (0..4).max_by_key(|i| (counts[*i], 3 - i)).unwrap_or(0);
    b",;\t|"[if counts[best] == 0 { 0 } else { best }]
}

/// `-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?`: leading zeros stay text (ZIP codes, ids).
fn parse_number(s: &str) -> Option<Number> {
    let b = s.as_bytes();
    let digits = |from: usize| b[from..].iter().take_while(|c| c.is_ascii_digit()).count();
    let mut i = usize::from(b.first() == Some(&b'-'));
    let int_len = digits(i);
    if int_len == 0 || (int_len > 1 && b[i] == b'0') {
        return None;
    }
    i += int_len;
    if b.get(i) == Some(&b'.') {
        let frac = digits(i + 1);
        if frac == 0 {
            return None;
        }
        i += 1 + frac;
    }
    if matches!(b.get(i), Some(b'e' | b'E')) {
        let sign = usize::from(matches!(b.get(i + 1), Some(b'+' | b'-')));
        let exp = digits(i + 1 + sign);
        if exp == 0 {
            return None;
        }
        i += 1 + sign + exp;
    }
    if i != b.len() {
        return None;
    }
    s.parse::<i64>()
        .map(Number::from)
        .ok()
        .or_else(|| s.parse::<f64>().ok().and_then(Number::from_f64))
}

fn cell_value(cell: &str, parse: bool) -> Value {
    if parse {
        match cell {
            "true" => return Value::Bool(true),
            "false" => return Value::Bool(false),
            _ => {}
        }
        if let Some(n) = parse_number(cell) {
            return Value::Number(n);
        }
    }
    Value::String(cell.to_string())
}

fn csv_to_json_process(req: &Request) -> ToolResult {
    let delimiter = match req.options.str("delimiter") {
        "Auto-detect" => detect_delimiter(req.input),
        choice => delimiter_byte(choice),
    };
    let trim = if req.options.bool("trim") {
        csv::Trim::All
    } else {
        csv::Trim::None
    };
    let rows = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .trim(trim)
        .from_reader(req.input.as_bytes())
        .records()
        .map(|r| r.map_err(|e| ToolError::InvalidInput(format!("Invalid CSV: {e}"))))
        .collect::<Result<Vec<_>, _>>()?;
    if rows.is_empty() {
        return Err(ToolError::InvalidInput("No data found in CSV".into()));
    }
    let parse = req.options.bool("parseNumbers");

    let result = if req.options.str("format") == "Array of Arrays" {
        rows.iter()
            .map(|row| Value::Array(row.iter().map(|c| cell_value(c, parse)).collect()))
            .collect()
    } else {
        let (headers, data): (Vec<String>, &[csv::StringRecord]) = if req.options.bool("hasHeaders")
        {
            (rows[0].iter().map(str::to_string).collect(), &rows[1..])
        } else {
            (Vec::new(), &rows[..])
        };
        data.iter()
            .map(|row| {
                // Keys are sorted.
                let mut fields = BTreeMap::new();
                for (i, cell) in row.iter().enumerate() {
                    let key = match headers.get(i) {
                        Some(h) if !h.is_empty() => h.clone(),
                        _ => format!("column{}", i + 1),
                    };
                    fields.insert(key, cell_value(cell, parse));
                }
                for (i, h) in headers.iter().enumerate().skip(row.len()) {
                    let key = if h.is_empty() {
                        format!("column{}", i + 1)
                    } else {
                        h.clone()
                    };
                    fields
                        .entry(key)
                        .or_insert_with(|| Value::String(String::new()));
                }
                Value::Object(fields.into_iter().collect::<Map<_, _>>())
            })
            .collect()
    };
    Ok(ToolOutput::text(write_json(
        &Value::Array(result),
        Layout::Pretty(2),
        false,
        false,
    )))
}

#[derive(Clone, Copy)]
enum ArrayMode {
    Join,
    Json,
    Index,
}

struct Flatten {
    /// Remaining object levels to expand; `None` expands everything.
    depth: Option<usize>,
    arrays: ArrayMode,
}

impl Flatten {
    fn insert(
        &self,
        key: String,
        value: &Value,
        depth: Option<usize>,
        out: &mut BTreeMap<String, String>,
    ) {
        match value {
            Value::Object(map) if depth != Some(0) && !map.is_empty() => {
                for (k, v) in map {
                    self.insert(format!("{key}.{k}"), v, depth.map(|d| d - 1), out);
                }
            }
            Value::Array(items) => match self.arrays {
                ArrayMode::Join => {
                    let parts: Vec<String> = items.iter().map(cell_text).collect();
                    out.insert(key, parts.join(";"));
                }
                ArrayMode::Json => {
                    out.insert(key, cell_text(value));
                }
                ArrayMode::Index => {
                    for (i, v) in items.iter().enumerate() {
                        self.insert(format!("{key}.{i}"), v, depth, out);
                    }
                }
            },
            other => {
                out.insert(key, cell_text(other));
            }
        }
    }
}

/// Primitives as plain text, containers as minified JSON.
fn cell_text(v: &Value) -> String {
    match v {
        Value::Object(_) | Value::Array(_) => write_json(v, Layout::Minify, false, false),
        other => scalar_text(other),
    }
}

fn json_to_csv_process(req: &Request) -> ToolResult {
    let input = parse_json(req.input)?;
    let items = match input {
        Value::Array(items) => items,
        object @ Value::Object(_) => vec![object],
        _ => {
            return Err(ToolError::InvalidInput(
                "Expected JSON array or object".into(),
            ))
        }
    };
    let flatten = Flatten {
        depth: match req.options.str("flatten") {
            "None" => Some(0),
            "1 level" => Some(1),
            "2 levels" => Some(2),
            _ => None,
        },
        arrays: match req.options.str("arrays") {
            "JSON string" => ArrayMode::Json,
            "Index columns" => ArrayMode::Index,
            _ => ArrayMode::Join,
        },
    };

    let rows: Vec<BTreeMap<String, String>> = items
        .iter()
        .map(|item| {
            let mut row = BTreeMap::new();
            match item {
                Value::Object(map) => {
                    for (k, v) in map {
                        flatten.insert(k.clone(), v, flatten.depth, &mut row);
                    }
                }
                other => {
                    row.insert("value".to_string(), cell_text(other));
                }
            }
            row
        })
        .collect();
    let columns: Vec<&String> = rows
        .iter()
        .flat_map(BTreeMap::keys)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();

    let mut writer = csv::WriterBuilder::new()
        .delimiter(delimiter_byte(req.options.str("delimiter")))
        .from_writer(Vec::new());
    let write_err = |e: csv::Error| ToolError::ProcessingFailed(format!("CSV output failed: {e}"));
    if req.options.bool("includeHeaders") {
        writer.write_record(&columns).map_err(write_err)?;
    }
    for row in &rows {
        let cells = columns
            .iter()
            .map(|c| row.get(*c).map_or("", String::as_str));
        writer.write_record(cells).map_err(write_err)?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| ToolError::ProcessingFailed(format!("CSV output failed: {e}")))?;
    String::from_utf8(bytes)
        .map(ToolOutput::text)
        .map_err(|_| ToolError::ProcessingFailed("Could not convert to string".into()))
}
