use std::fmt::Write;

use super::{picker, stepper, toggle};
use crate::{Category, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOutput, ToolResult};

const CASE_FORMATS: [&str; 12] = [
    "camelCase",
    "PascalCase",
    "snake_case",
    "SCREAMING_SNAKE_CASE",
    "kebab-case",
    "Train-Case",
    "dot.case",
    "path/case",
    "Title Case",
    "Sentence case",
    "UPPERCASE",
    "lowercase",
];

pub fn string_case_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "string-case",
            name: "String Case",
            category: Category::Text,
            description: "Convert text between camelCase, snake_case, kebab-case, and more",
            keywords: vec![
                "case", "camel", "snake", "kebab", "pascal", "title", "upper",
            ],
            icon: "case-sensitive",
            input_placeholder: "Enter text to convert...",
            output_placeholder: "Converted text will appear here",
            primary_action: "Convert",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "format",
                    "Convert to",
                    "Target case format",
                    CASE_FORMATS.to_vec(),
                ),
                toggle(
                    "preserveAcronyms",
                    "Preserve acronyms",
                    "Keep acronyms like 'HTTP' intact",
                    false,
                ),
            ],
        },
        process: Some(string_case_process),
    }
}

fn string_case_process(req: &Request) -> ToolResult {
    let format = req.options.str("format");
    let preserve = req.options.bool("preserveAcronyms");
    let converted = req
        .input
        .lines()
        .map(|line| convert_case(line, format, preserve))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(ToolOutput::text(converted))
}

/// Splits on non-alphanumerics, lower→upper transitions and the end of an
/// acronym ("HTTPServer" → "HTTP", "Server").
fn split_words(line: &str) -> Vec<String> {
    let chars: Vec<char> = line.chars().collect();
    let mut words: Vec<String> = Vec::new();
    let mut current = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if !c.is_alphanumeric() {
            if !current.is_empty() {
                words.push(std::mem::take(&mut current));
            }
            continue;
        }
        if let Some(prev) = current.chars().last() {
            let after_lower = c.is_uppercase() && (prev.is_lowercase() || prev.is_ascii_digit());
            let acronym_end = c.is_uppercase()
                && prev.is_uppercase()
                && chars.get(i + 1).is_some_and(|n| n.is_lowercase());
            if after_lower || acronym_end {
                words.push(std::mem::take(&mut current));
            }
        }
        current.push(c);
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

fn capitalize(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => first
            .to_uppercase()
            .chain(chars.flat_map(char::to_lowercase))
            .collect(),
        None => String::new(),
    }
}

fn convert_case(line: &str, format: &str, preserve: bool) -> String {
    match format {
        "UPPERCASE" => return line.to_uppercase(),
        "lowercase" => return line.to_lowercase(),
        _ => {}
    }
    let words = split_words(line);
    let cap = |w: &String| {
        if preserve && w.chars().count() > 1 && w.chars().all(|c| !c.is_lowercase()) {
            w.clone()
        } else {
            capitalize(w)
        }
    };
    let lower = |w: &String| w.to_lowercase();
    match format {
        "camelCase" => words
            .iter()
            .enumerate()
            .map(|(i, w)| {
                if i == 0 && !preserve {
                    lower(w)
                } else {
                    cap(w)
                }
            })
            .collect(),
        "PascalCase" => words.iter().map(cap).collect(),
        "snake_case" => words.iter().map(lower).collect::<Vec<_>>().join("_"),
        "SCREAMING_SNAKE_CASE" => words
            .iter()
            .map(|w| w.to_uppercase())
            .collect::<Vec<_>>()
            .join("_"),
        "kebab-case" => words.iter().map(lower).collect::<Vec<_>>().join("-"),
        "Train-Case" => words.iter().map(cap).collect::<Vec<_>>().join("-"),
        "dot.case" => words.iter().map(lower).collect::<Vec<_>>().join("."),
        "path/case" => words.iter().map(lower).collect::<Vec<_>>().join("/"),
        "Title Case" => words.iter().map(cap).collect::<Vec<_>>().join(" "),
        _ => words
            .iter()
            .enumerate()
            .map(|(i, w)| if i == 0 { cap(w) } else { lower(w) })
            .collect::<Vec<_>>()
            .join(" "),
    }
}

const ESCAPE_FORMATS: [&str; 9] = [
    "JavaScript/JSON",
    "Python",
    "Java",
    "C/C++",
    "Swift",
    "SQL",
    "Regex",
    "HTML",
    "URL",
];

pub fn string_escape_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "string-escape",
            name: "String Escape",
            category: Category::Text,
            description: "Escape and unescape strings for various languages",
            keywords: vec!["escape", "unescape", "quote", "json", "string"],
            icon: "quote",
            input_placeholder: "Enter text to escape or escaped text to unescape...",
            output_placeholder: "Processed string will appear here",
            primary_action: "Escape",
            secondary_action: Some("Unescape"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![picker(
                "format",
                "Format",
                "Escape format for target language",
                ESCAPE_FORMATS.to_vec(),
            )],
        },
        process: Some(string_escape_process),
    }
}

fn string_escape_process(req: &Request) -> ToolResult {
    let format = req.options.str("format");
    let input = req.input;
    let output = match (format, req.reverse) {
        ("SQL", false) => input.replace('\'', "''"),
        ("SQL", true) => input.replace("''", "'"),
        ("Regex", false) => fancy_regex::escape(input).into_owned(),
        ("Regex", true) => unescape_backslashes(input, format)?,
        ("HTML", false) => escape_html(input),
        ("HTML", true) => unescape_html(input),
        ("URL", false) => percent_encode(input),
        ("URL", true) => percent_decode(input)?,
        (_, false) => escape_backslashes(input, format),
        (_, true) => unescape_backslashes(input, format)?,
    };
    Ok(ToolOutput::text(output))
}

fn escape_backslashes(input: &str, format: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\'' if format == "Python" => out.push_str("\\'"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\0' if matches!(format, "C/C++" | "Swift") => out.push_str("\\0"),
            '\u{8}' if format != "Swift" => out.push_str("\\b"),
            '\u{c}' if format != "Swift" => out.push_str("\\f"),
            c if c.is_control() => {
                let code = c as u32;
                let _ = match format {
                    "Swift" => write!(out, "\\u{{{code:X}}}"),
                    "Python" | "C/C++" if code <= 0xFF => write!(out, "\\x{code:02x}"),
                    _ => write!(out, "\\u{code:04x}"),
                };
            }
            c => out.push(c),
        }
    }
    out
}

fn unescape_backslashes(input: &str, format: &str) -> Result<String, ToolError> {
    let invalid = || ToolError::InvalidInput("Invalid escape sequence".into());
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        let Some(e) = chars.next() else {
            out.push('\\');
            break;
        };
        // Regex escapes only punctuation; letters keep their backslash.
        if format == "Regex" {
            if e.is_alphanumeric() {
                out.push('\\');
            }
            out.push(e);
            continue;
        }
        match e {
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            '0' => out.push('\0'),
            'x' => {
                let hex: String = (0..2).filter_map(|_| chars.next()).collect();
                let code = u32::from_str_radix(&hex, 16).map_err(|_| invalid())?;
                out.push(char::from_u32(code).ok_or_else(invalid)?);
            }
            'u' => {
                let read_hex = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        let hex: String = chars.by_ref().take_while(|c| *c != '}').collect();
                        u32::from_str_radix(&hex, 16).map_err(|_| invalid())
                    } else {
                        let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                        u32::from_str_radix(&hex, 16).map_err(|_| invalid())
                    }
                };
                let mut code = read_hex(&mut chars)?;
                if (0xD800..0xDC00).contains(&code) {
                    // High surrogate: must be followed by `\uXXXX` low surrogate.
                    if chars.next() != Some('\\') || chars.next() != Some('u') {
                        return Err(invalid());
                    }
                    let low = read_hex(&mut chars)?;
                    if !(0xDC00..0xE000).contains(&low) {
                        return Err(invalid());
                    }
                    code = 0x10000 + ((code - 0xD800) << 10) + (low - 0xDC00);
                }
                out.push(char::from_u32(code).ok_or_else(invalid)?);
            }
            '\\' | '"' | '\'' | '/' => out.push(e),
            other => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    Ok(out)
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

fn unescape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let decoded = rest.find(';').filter(|end| *end <= 10).and_then(|end| {
            let name = &rest[1..end];
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => '\u{a0}',
                _ => {
                    let num = name.strip_prefix('#')?;
                    let code = match num.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                        None => num.parse().ok()?,
                    };
                    char::from_u32(code)?
                }
            };
            Some((c, end))
        });
        match decoded {
            Some((c, end)) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

fn percent_decode(input: &str) -> Result<String, ToolError> {
    let invalid = || ToolError::InvalidInput("Invalid percent-encoded string".into());
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = input
                .get(i + 1..i + 3)
                .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                .ok_or_else(invalid)?;
            out.push(u8::from_str_radix(hex, 16).map_err(|_| invalid())?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| invalid())
}

const STOP_WORDS: [&str; 20] = [
    "a", "an", "the", "and", "or", "but", "is", "are", "was", "were", "of", "to", "in", "on", "at",
    "for", "with", "by", "as", "it",
];

pub fn slug_generator_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "slug-generator",
            name: "Slug Generator",
            category: Category::Text,
            description: "Generate URL-friendly slugs from text",
            keywords: vec!["slug", "url", "permalink", "seo"],
            icon: "link",
            input_placeholder: "Enter title or text to convert to slug...",
            output_placeholder: "URL slug will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "separator",
                    "Separator",
                    "Character to separate words",
                    vec!["-", "_", "."],
                ),
                toggle("lowercase", "Lowercase", "Convert to lowercase", true),
                toggle(
                    "transliterate",
                    "Transliterate accents",
                    "Convert é→e, ñ→n, etc.",
                    true,
                ),
                stepper(
                    "maxLength",
                    "Max length",
                    "Maximum slug length",
                    (10, 200, 80),
                ),
                toggle(
                    "removeStopWords",
                    "Remove stop words",
                    "Remove common words (a, the, is, etc.)",
                    false,
                ),
            ],
        },
        process: Some(slug_process),
    }
}

fn slug_process(req: &Request) -> ToolResult {
    let separator = req.options.str("separator");
    let max_length = req.options.i64("maxLength").max(1) as usize;
    let mut text = if req.options.bool("transliterate") {
        deunicode::deunicode(req.input)
    } else {
        req.input.to_string()
    };
    if req.options.bool("lowercase") {
        text = text.to_lowercase();
    }
    // Apostrophes join letters ("don't" → "dont") instead of splitting words.
    text.retain(|c| c != '\'' && c != '\u{2019}');
    let remove_stop_words = req.options.bool("removeStopWords");
    let words: Vec<&str> = text
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| !w.is_empty())
        .filter(|w| !remove_stop_words || !STOP_WORDS.contains(&w.to_lowercase().as_str()))
        .collect();
    let joined = words.join(separator);
    let slug: String = joined.chars().take(max_length).collect();
    let slug = slug.trim_end_matches(separator).to_string();
    if slug.is_empty() {
        return Err(ToolError::ProcessingFailed(
            "Could not generate a slug from the input".into(),
        ));
    }
    let slug_words = slug.split(separator).filter(|w| !w.is_empty()).count();
    Ok(ToolOutput::multiple(vec![
        OutputItem::new("Slug", slug.clone()),
        OutputItem::new("Length", format!("{} characters", slug.chars().count())),
        OutputItem::new("URL preview", format!("https://example.com/{slug}")),
        OutputItem::new(
            "Words",
            format!("{} → {}", req.input.split_whitespace().count(), slug_words),
        ),
    ]))
}
