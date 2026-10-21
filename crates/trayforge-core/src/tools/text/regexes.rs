use std::fmt::Write;

use fancy_regex::Regex;

use super::{picker, text_field, toggle};
use crate::{Category, Request, Tool, ToolError, ToolInfo, ToolOutput, ToolResult};

pub fn regex_tester_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "regex-tester",
            name: "Regex Tester",
            category: Category::Text,
            description: "Test and debug regular expressions with match highlighting",
            keywords: vec!["regex", "regexp", "pattern", "match", "replace"],
            icon: "regex",
            input_placeholder: "Enter pattern on first line, then test string(s)...\n\nExample:\n\\d+\nThere are 42 apples and 7 oranges",
            output_placeholder: "Match results will appear here",
            primary_action: "Test",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                toggle("caseInsensitive", "Case insensitive (i)", "Ignore case when matching", false),
                toggle("multiline", "Multiline (m)", "^ and $ match line start/end", false),
                toggle(
                    "dotMatchesNewlines",
                    "Dot matches newlines (s)",
                    "'.' matches newline characters",
                    false,
                ),
                toggle("showGroups", "Show capture groups", "Display captured groups", true),
                text_field(
                    "replacement",
                    "Replacement string",
                    "Use $1, $2, etc. for groups",
                    "Empty = match only",
                    "",
                ),
            ],
        },
        process: Some(regex_tester_process),
    }
}

fn compile(pattern: &str, flags: &str) -> Result<Regex, ToolError> {
    let full = if flags.is_empty() {
        pattern.to_string()
    } else {
        format!("(?{flags}){pattern}")
    };
    Regex::new(&full)
        .map_err(|e| ToolError::InvalidInput(format!("Invalid regular expression: {e}")))
}

/// Lists matches with character offsets (and capture groups); `limit` caps the count.
fn describe_matches(
    re: &Regex,
    text: &str,
    show_groups: bool,
    limit: usize,
) -> Result<String, ToolError> {
    // Matches arrive in increasing order, so count characters incrementally.
    let (mut seen_bytes, mut seen_chars) = (0, 0);
    let mut char_offset = |byte: usize| {
        if byte < seen_bytes {
            (seen_bytes, seen_chars) = (0, 0);
        }
        seen_chars += text[seen_bytes..byte].chars().count();
        seen_bytes = byte;
        seen_chars
    };
    let mut out = String::new();
    let mut count = 0;
    for captures in re.captures_iter(text).take(limit) {
        let captures = captures.map_err(|e| ToolError::ProcessingFailed(e.to_string()))?;
        let whole = captures.get(0).expect("group 0 always participates");
        count += 1;
        let _ = writeln!(
            out,
            "Match {count}: \"{}\" (position {}-{})",
            whole.as_str(),
            char_offset(whole.start()),
            char_offset(whole.end())
        );
        if show_groups {
            for i in 1..captures.len() {
                match captures.get(i) {
                    Some(g) => {
                        let _ = writeln!(out, "  Group {i}: \"{}\"", g.as_str());
                    }
                    None => {
                        let _ = writeln!(out, "  Group {i}: (no match)");
                    }
                }
            }
        }
    }
    if count == 0 {
        return Ok("No matches found".into());
    }
    Ok(out.trim_end().to_string())
}

fn regex_tester_process(req: &Request) -> ToolResult {
    let (pattern, text) = req
        .input
        .split_once('\n')
        .map_or((req.input, ""), |(p, t)| (p.trim_end_matches('\r'), t));
    if pattern.trim().is_empty() {
        return Err(ToolError::InvalidInput(
            "First line must be the regex pattern".into(),
        ));
    }
    if text.trim().is_empty() {
        return Err(ToolError::InvalidInput(
            "Please provide test string after the pattern".into(),
        ));
    }
    let mut flags = String::new();
    for (id, flag) in [
        ("caseInsensitive", 'i'),
        ("multiline", 'm'),
        ("dotMatchesNewlines", 's'),
    ] {
        if req.options.bool(id) {
            flags.push(flag);
        }
    }
    let re = compile(pattern, &flags)?;
    let mut out = describe_matches(&re, text, req.options.bool("showGroups"), usize::MAX)?;
    let replacement = req.options.str("replacement");
    if !replacement.is_empty() {
        let replaced = re
            .try_replacen(text, 0, replacement)
            .map_err(|e| ToolError::ProcessingFailed(e.to_string()))?;
        let _ = write!(out, "\n\nReplacement result:\n{replaced}");
    }
    Ok(ToolOutput::text(out))
}

const CATEGORIES: [&str; 7] = [
    "All Categories",
    "Email & Communication",
    "Web & URLs",
    "Numbers",
    "Dates & Times",
    "Identifiers",
    "Text",
];

/// (label as "Category: Name", pattern, description)
const LIBRARY: [(&str, &str, &str); 21] = [
    (
        "Email & Communication: Email Address",
        r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}",
        "Matches an email address",
    ),
    (
        "Email & Communication: Phone Number",
        r"\+?\d{1,3}[\s.-]?\(?\d{1,4}\)?[\s.-]?\d{1,4}[\s.-]?\d{1,9}",
        "Matches an international phone number",
    ),
    (
        "Email & Communication: Hashtag",
        r"#\w+",
        "Matches a #hashtag",
    ),
    (
        "Email & Communication: Mention",
        r"@\w+",
        "Matches an @mention",
    ),
    (
        "Web & URLs: URL",
        r"https?://[^\s/$.?#][^\s]*",
        "Matches an http or https URL",
    ),
    (
        "Web & URLs: Domain Name",
        r"(?:[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)+[a-zA-Z]{2,}",
        "Matches a domain name",
    ),
    (
        "Web & URLs: IPv4 Address",
        r"\b(?:(?:25[0-5]|2[0-4]\d|1?\d?\d)\.){3}(?:25[0-5]|2[0-4]\d|1?\d?\d)\b",
        "Matches an IPv4 address",
    ),
    (
        "Web & URLs: Hex Color",
        r"#(?:[0-9a-fA-F]{3}){1,2}\b",
        "Matches a #RGB or #RRGGBB color",
    ),
    ("Numbers: Integer", r"-?\d+", "Matches a signed integer"),
    (
        "Numbers: Decimal",
        r"-?\d+\.\d+",
        "Matches a signed decimal number",
    ),
    (
        "Numbers: Percentage",
        r"\d+(?:\.\d+)?%",
        "Matches a percentage",
    ),
    (
        "Numbers: Currency (USD)",
        r"\$\d{1,3}(?:,\d{3})*(?:\.\d{2})?",
        "Matches a US dollar amount",
    ),
    (
        "Dates & Times: Date (YYYY-MM-DD)",
        r"\d{4}-(?:0[1-9]|1[0-2])-(?:0[1-9]|[12]\d|3[01])",
        "Matches an ISO calendar date",
    ),
    (
        "Dates & Times: Time (24h)",
        r"(?:[01]\d|2[0-3]):[0-5]\d(?::[0-5]\d)?",
        "Matches a 24-hour time",
    ),
    (
        "Dates & Times: ISO 8601 Timestamp",
        r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})",
        "Matches an ISO 8601 date-time",
    ),
    (
        "Identifiers: UUID",
        r"[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}",
        "Matches a UUID",
    ),
    (
        "Identifiers: Semantic Version",
        r"\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?",
        "Matches a semver version",
    ),
    (
        "Identifiers: Slug",
        r"[a-z0-9]+(?:-[a-z0-9]+)*",
        "Matches a lowercase URL slug",
    ),
    (
        "Text: HTML Tag",
        r"</?[a-zA-Z][^>]*>",
        "Matches an HTML opening or closing tag",
    ),
    (
        "Text: Duplicate Words",
        r"\b(\w+)\s+\1\b",
        "Matches a repeated word",
    ),
    (
        "Text: Empty Lines",
        r"(?m)^\s*$",
        "Matches empty or blank lines",
    ),
];

pub fn regex_library_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "regex-library",
            name: "Regex Library",
            category: Category::Text,
            description: "Browse and copy common regex patterns",
            keywords: vec!["regex", "pattern", "library", "email", "url", "uuid"],
            icon: "library",
            input_placeholder: "Enter text to test regex patterns against (optional)...",
            output_placeholder: "Selected regex pattern and matches will appear here",
            primary_action: "Get Pattern",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                picker(
                    "category",
                    "Category",
                    "Filter patterns by category",
                    CATEGORIES.to_vec(),
                ),
                picker(
                    "pattern",
                    "Pattern",
                    "Select a regex pattern",
                    LIBRARY.iter().map(|e| e.0).collect(),
                ),
                toggle(
                    "showAllMatches",
                    "Show all matches",
                    "Display all matches when testing against input",
                    true,
                ),
                toggle(
                    "caseInsensitive",
                    "Case insensitive",
                    "Apply case-insensitive matching",
                    false,
                ),
            ],
        },
        process: Some(regex_library_process),
    }
}

fn regex_library_process(req: &Request) -> ToolResult {
    let category = req.options.str("category");
    let selected = req.options.str("pattern");
    let in_category = |label: &str| {
        category == "All Categories" || label.split_once(": ").is_some_and(|(c, _)| c == category)
    };
    // The pattern list cannot shrink with the category filter, so a pattern
    // outside the chosen category falls back to that category's first entry.
    let (label, pattern, description) = LIBRARY
        .iter()
        .find(|e| e.0 == selected && in_category(e.0))
        .or_else(|| LIBRARY.iter().find(|e| in_category(e.0)))
        .ok_or_else(|| ToolError::InvalidInput("No pattern selected".into()))?;

    let case_insensitive = req.options.bool("caseInsensitive");
    let mut out = format!("{label}\n{description}\n\nPattern:\n{pattern}");
    if !req.input.trim().is_empty() {
        let re = compile(pattern, if case_insensitive { "i" } else { "" })?;
        let limit = if req.options.bool("showAllMatches") {
            usize::MAX
        } else {
            1
        };
        let results = describe_matches(&re, req.input, true, limit)?;
        let _ = write!(out, "\n\n--- Test Results ---\n{results}");
    }
    let flag = if case_insensitive { "i" } else { "" };
    let _ = write!(
        out,
        "\n\n--- Copy Formats ---\nJavaScript: /{pattern}/{flag}\nPython: re.compile(r\"{pattern}\"{})\nSwift: try NSRegularExpression(pattern: #\"{pattern}\"#{})",
        if case_insensitive { ", re.IGNORECASE" } else { "" },
        if case_insensitive { ", options: .caseInsensitive" } else { "" },
    );
    Ok(ToolOutput::text(out))
}
