use super::indent_option;
use crate::{Category, OptionKind, Request, Tool, ToolInfo, ToolOption, ToolOutput, ToolResult};

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "sql-formatter",
            name: "SQL Formatter",
            category: Category::Formats,
            description: "Format and beautify SQL queries",
            keywords: vec!["sql", "format", "beautify", "query", "database"],
            icon: "database",
            input_placeholder: "Paste SQL query here...",
            output_placeholder: "Formatted SQL will appear here",
            primary_action: "Format",
            secondary_action: None,
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "keywordCase",
                    label: "Keyword case",
                    help: Some("Case for SQL keywords"),
                    // "UPPERCASE" is the default.
                    kind: OptionKind::Picker {
                        choices: vec!["UPPERCASE", "lowercase", "Preserve"],
                        default: 0,
                    },
                },
                indent_option(),
                ToolOption {
                    id: "newlineBeforeAnd",
                    label: "New line before AND/OR",
                    help: Some("Place AND/OR on new lines"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "commaFirst",
                    label: "Comma at start of line",
                    help: Some("Place commas at beginning of lines"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(process),
    }
}

/// Clauses that start a new line, longest sequences first.
const CLAUSES: &[&[&str]] = &[
    &["LEFT", "OUTER", "JOIN"],
    &["RIGHT", "OUTER", "JOIN"],
    &["FULL", "OUTER", "JOIN"],
    &["GROUP", "BY"],
    &["ORDER", "BY"],
    &["INSERT", "INTO"],
    &["DELETE", "FROM"],
    &["UNION", "ALL"],
    &["INNER", "JOIN"],
    &["LEFT", "JOIN"],
    &["RIGHT", "JOIN"],
    &["FULL", "JOIN"],
    &["CROSS", "JOIN"],
    &["SELECT"],
    &["FROM"],
    &["WHERE"],
    &["HAVING"],
    &["LIMIT"],
    &["OFFSET"],
    &["UNION"],
    &["INTERSECT"],
    &["EXCEPT"],
    &["VALUES"],
    &["UPDATE"],
    &["SET"],
    &["JOIN"],
];

const KEYWORDS: &[&str] = &[
    "SELECT",
    "FROM",
    "WHERE",
    "GROUP",
    "ORDER",
    "BY",
    "HAVING",
    "LIMIT",
    "OFFSET",
    "UNION",
    "ALL",
    "INTERSECT",
    "EXCEPT",
    "INSERT",
    "INTO",
    "VALUES",
    "UPDATE",
    "SET",
    "DELETE",
    "JOIN",
    "INNER",
    "LEFT",
    "RIGHT",
    "FULL",
    "CROSS",
    "OUTER",
    "AND",
    "OR",
    "NOT",
    "AS",
    "ON",
    "IN",
    "IS",
    "NULL",
    "LIKE",
    "ILIKE",
    "BETWEEN",
    "DESC",
    "ASC",
    "DISTINCT",
    "CASE",
    "WHEN",
    "THEN",
    "ELSE",
    "END",
    "EXISTS",
    "ANY",
    "CREATE",
    "TABLE",
    "ALTER",
    "DROP",
    "INDEX",
    "PRIMARY",
    "KEY",
    "DEFAULT",
    "UNIQUE",
    "USING",
    "WITH",
    "OVER",
    "PARTITION",
    "TRUE",
    "FALSE",
];

enum Token {
    Word {
        text: String,
        opens: usize,
        closes: usize,
    },
    Comma,
    Comment(String),
}

fn tokenize(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == ',' {
            tokens.push(Token::Comma);
            i += 1;
        } else if c == '-' && chars.get(i + 1) == Some(&'-') {
            let end = chars[i..]
                .iter()
                .position(|c| *c == '\n')
                .map_or(chars.len(), |p| i + p);
            tokens.push(Token::Comment(
                chars[i..end].iter().collect::<String>().trim_end().into(),
            ));
            i = end;
        } else if c == '/' && chars.get(i + 1) == Some(&'*') {
            let end = (i + 2..chars.len().saturating_sub(1))
                .find(|&p| chars[p] == '*' && chars[p + 1] == '/')
                .map_or(chars.len(), |p| p + 2);
            tokens.push(Token::Comment(chars[i..end].iter().collect()));
            i = end;
        } else {
            let (mut text, mut opens, mut closes) = (String::new(), 0, 0);
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != ',' {
                let c = chars[i];
                if matches!(c, '\'' | '"' | '`') {
                    // Quoted section, kept verbatim; a doubled quote stays inside.
                    text.push(c);
                    i += 1;
                    while i < chars.len() {
                        let q = chars[i];
                        text.push(q);
                        i += 1;
                        if q == c {
                            if chars.get(i) == Some(&c) {
                                text.push(c);
                                i += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    continue;
                }
                opens += usize::from(c == '(');
                closes += usize::from(c == ')');
                text.push(c);
                i += 1;
            }
            tokens.push(Token::Word {
                text,
                opens,
                closes,
            });
        }
    }
    tokens
}

/// The bare word without surrounding parentheses and a trailing semicolon.
fn core(word: &str) -> &str {
    word.trim_start_matches('(').trim_end_matches([')', ';'])
}

fn apply_case(word: &str, case: &str) -> String {
    let bare = core(word);
    if case == "Preserve" || !KEYWORDS.contains(&bare.to_uppercase().as_str()) {
        return word.to_string();
    }
    let start = word.len() - word.trim_start_matches('(').len();
    let cased = if case == "lowercase" {
        bare.to_lowercase()
    } else {
        bare.to_uppercase()
    };
    format!("{}{cased}{}", &word[..start], &word[start + bare.len()..])
}

struct Printer {
    out: String,
    line_start: bool,
}

impl Printer {
    fn word(&mut self, text: &str) {
        if !self.line_start {
            self.out.push(' ');
        }
        self.out.push_str(text);
        self.line_start = false;
    }

    fn newline(&mut self, indent: usize) {
        self.out.push('\n');
        self.out.push_str(&" ".repeat(indent));
        self.line_start = true;
    }
}

fn process(req: &Request) -> ToolResult {
    let case = req.options.str("keywordCase");
    let indent = req.options.i64("indent") as usize;
    let and_or_newline = req.options.bool("newlineBeforeAnd");
    let comma_first = req.options.bool("commaFirst");

    let tokens = tokenize(req.input);
    let upper_core = |t: Option<&Token>| match t {
        Some(Token::Word { text, .. }) => Some(core(text).to_uppercase()),
        _ => None,
    };
    let mut printer = Printer {
        out: String::new(),
        line_start: true,
    };
    let (mut depth, mut in_between) = (0usize, false);
    let mut i = 0;
    while i < tokens.len() {
        match &tokens[i] {
            Token::Comma => {
                if depth > 0 {
                    printer.out.push(',');
                } else if comma_first {
                    printer.newline(0);
                    printer.out.push(',');
                } else {
                    printer.out.push(',');
                    printer.newline(0);
                }
                printer.line_start = depth == 0 && !comma_first;
                i += 1;
            }
            Token::Comment(text) => {
                printer.word(text);
                printer.newline(0);
                i += 1;
            }
            Token::Word {
                text,
                opens,
                closes,
            } => {
                let upper = core(text).to_uppercase();
                let clause = (depth == 0 && !text.starts_with('('))
                    .then(|| {
                        CLAUSES.iter().find(|seq| {
                            seq.iter().enumerate().all(|(k, kw)| {
                                let word = if k == 0 {
                                    Some(upper.clone())
                                } else {
                                    upper_core(tokens.get(i + k))
                                };
                                word.as_deref() == Some(*kw)
                            })
                        })
                    })
                    .flatten();
                let consumed = if let Some(seq) = clause {
                    if !printer.out.is_empty() && !printer.line_start {
                        printer.newline(0);
                    }
                    in_between = false;
                    seq.len()
                } else {
                    if depth == 0 && upper == "BETWEEN" {
                        in_between = true;
                    }
                    if depth == 0 && and_or_newline && matches!(upper.as_str(), "AND" | "OR") {
                        if upper == "AND" && in_between {
                            in_between = false;
                        } else {
                            printer.newline(indent);
                        }
                    }
                    1
                };
                for k in 0..consumed {
                    if let Some(Token::Word { text, .. }) = tokens.get(i + k) {
                        printer.word(&apply_case(text, case));
                    }
                }
                // Parentheses of the consumed words (only the first can carry any).
                depth = (depth + opens).saturating_sub(*closes);
                i += consumed;
                if depth == 0 && text.ends_with(';') && i < tokens.len() {
                    printer.out.push('\n');
                    printer.newline(0);
                }
            }
        }
    }
    Ok(ToolOutput::text(printer.out.trim_end().to_string()))
}
