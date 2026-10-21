use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
use std::fmt::Write;

use super::{picker, stepper, text_field, toggle};
use crate::{Category, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOutput, ToolResult};

pub fn string_diff_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "string-diff",
            name: "String Diff",
            category: Category::Text,
            description: "Compare two strings and show differences",
            keywords: vec!["diff", "compare", "difference", "text"],
            icon: "diff",
            input_placeholder:
                "Enter two texts separated by a line containing only '---'\n\nText 1\n---\nText 2",
            output_placeholder: "Differences will appear here",
            primary_action: "Compare",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "level",
                    "Comparison level",
                    "Level of detail for comparison",
                    vec!["Lines", "Words", "Characters"],
                ),
                toggle(
                    "ignoreWhitespace",
                    "Ignore whitespace",
                    "Ignore whitespace differences",
                    false,
                ),
                toggle(
                    "ignoreCase",
                    "Ignore case",
                    "Case-insensitive comparison",
                    false,
                ),
                toggle(
                    "unified",
                    "Show unified diff",
                    "Show in unified diff format",
                    true,
                ),
            ],
        },
        process: Some(string_diff_process),
    }
}

enum Edit {
    Equal(usize),
    Delete(usize),
    Insert(usize),
}

/// Longest-common-subsequence diff over comparison keys.
fn diff_edits(a: &[String], b: &[String]) -> Result<Vec<Edit>, ToolError> {
    let (n, m) = (a.len(), b.len());
    if n.saturating_mul(m) > 25_000_000 {
        return Err(ToolError::ProcessingFailed(
            "Texts are too large to compare".into(),
        ));
    }
    let width = m + 1;
    let mut lcs = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i * width + j] = if a[i] == b[j] {
                lcs[(i + 1) * width + j + 1] + 1
            } else {
                lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut edits = Vec::new();
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            edits.push(Edit::Equal(i));
            i += 1;
            j += 1;
        } else if j == m || (i < n && lcs[(i + 1) * width + j] >= lcs[i * width + j + 1]) {
            edits.push(Edit::Delete(i));
            i += 1;
        } else {
            edits.push(Edit::Insert(j));
            j += 1;
        }
    }
    Ok(edits)
}

fn string_diff_process(req: &Request) -> ToolResult {
    let all_lines: Vec<&str> = req.input.split('\n').collect();
    let separator = all_lines
        .iter()
        .position(|l| l.trim() == "---")
        .ok_or_else(|| {
            ToolError::InvalidInput(
                "Please separate two texts with a line containing only '---'".into(),
            )
        })?;
    let first = all_lines[..separator].join("\n");
    let second = all_lines[separator + 1..].join("\n");

    let level = req.options.str("level");
    let ignore_whitespace = req.options.bool("ignoreWhitespace");
    let ignore_case = req.options.bool("ignoreCase");
    let tokenize = |text: &str| -> Vec<String> {
        match level {
            "Words" => text.split_whitespace().map(str::to_string).collect(),
            "Characters" => text
                .chars()
                .filter(|c| !(ignore_whitespace && c.is_whitespace()))
                .map(String::from)
                .collect(),
            _ if text.is_empty() => Vec::new(),
            _ => text.split('\n').map(str::to_string).collect(),
        }
    };
    let key = |token: &str| -> String {
        let mut k = token.to_string();
        if ignore_whitespace && level == "Lines" {
            k = k.split_whitespace().collect::<Vec<_>>().join(" ");
        }
        if ignore_case {
            k = k.to_lowercase();
        }
        k
    };
    let (a, b) = (tokenize(&first), tokenize(&second));
    let edits = diff_edits(
        &a.iter().map(|t| key(t)).collect::<Vec<_>>(),
        &b.iter().map(|t| key(t)).collect::<Vec<_>>(),
    )?;

    let removed = edits
        .iter()
        .filter(|e| matches!(e, Edit::Delete(_)))
        .count();
    let added = edits
        .iter()
        .filter(|e| matches!(e, Edit::Insert(_)))
        .count();
    if added + removed == 0 {
        return Ok(ToolOutput::text("Texts are identical"));
    }

    let mut out = String::new();
    if level == "Lines" {
        let unified = req.options.bool("unified");
        if unified {
            out.push_str("--- Text 1\n+++ Text 2\n");
        }
        for edit in &edits {
            let line = match edit {
                Edit::Equal(i) if unified => format!("  {}", a[*i]),
                Edit::Equal(_) => continue,
                Edit::Delete(i) => format!("- {}", a[*i]),
                Edit::Insert(j) => format!("+ {}", b[*j]),
            };
            out.push_str(line.trim_end());
            out.push('\n');
        }
    } else {
        let glue = if level == "Words" { " " } else { "" };
        let mut parts = Vec::new();
        for edit in &edits {
            parts.push(match edit {
                Edit::Equal(i) => a[*i].clone(),
                Edit::Delete(i) => format!("[-{}-]", a[*i]),
                Edit::Insert(j) => format!("{{+{}+}}", b[*j]),
            });
        }
        out = parts.join(glue);
        out.push('\n');
    }
    let _ = write!(out, "\n{added} added, {removed} removed");
    Ok(ToolOutput::text(out))
}

pub fn line_sorter_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "line-sorter",
            name: "Line Sorter",
            category: Category::Text,
            description: "Sort, shuffle, dedupe, and manipulate text lines",
            keywords: vec!["sort", "lines", "dedupe", "shuffle", "reverse", "unique"],
            icon: "arrow-down-a-z",
            input_placeholder: "Enter text with multiple lines...",
            output_placeholder: "Processed lines will appear here",
            primary_action: "Process",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "operation",
                    "Operation",
                    "Operation to perform on lines",
                    vec![
                        "Sort A-Z",
                        "Sort Z-A",
                        "Sort by Length",
                        "Sort Numeric",
                        "Reverse",
                        "Shuffle",
                        "Remove Duplicates",
                        "Remove Empty Lines",
                    ],
                ),
                toggle(
                    "caseSensitive",
                    "Case sensitive",
                    "Consider case when sorting/deduping",
                    false,
                ),
                toggle(
                    "trimLines",
                    "Trim whitespace",
                    "Remove leading/trailing whitespace from each line",
                    true,
                ),
                text_field(
                    "prefix",
                    "Add prefix",
                    "Add prefix to each line",
                    "Add prefix to each line",
                    "",
                ),
                text_field(
                    "suffix",
                    "Add suffix",
                    "Add suffix to each line",
                    "Add suffix to each line",
                    "",
                ),
            ],
        },
        process: Some(line_sorter_process),
    }
}

fn line_sorter_process(req: &Request) -> ToolResult {
    let case_sensitive = req.options.bool("caseSensitive");
    let trim = req.options.bool("trimLines");
    let mut lines: Vec<String> = req
        .input
        .split('\n')
        .map(|l| l.trim_end_matches('\r'))
        .map(|l| if trim { l.trim() } else { l })
        .map(str::to_string)
        .collect();
    let before = lines.len();
    let key = |l: &String| {
        if case_sensitive {
            l.clone()
        } else {
            l.to_lowercase()
        }
    };
    let leading_number = |l: &String| {
        let t = l.trim();
        let end = t
            .char_indices()
            .find(|(i, c)| {
                !(c.is_ascii_digit()
                    || matches!(c, '.' | 'e' | 'E')
                    || (*i == 0 && matches!(c, '-' | '+')))
            })
            .map_or(t.len(), |(i, _)| i);
        t[..end].parse::<f64>().ok()
    };
    match req.options.str("operation") {
        "Sort A-Z" => lines.sort_by_key(key),
        "Sort Z-A" => lines.sort_by_key(|l| std::cmp::Reverse(key(l))),
        "Sort by Length" => lines.sort_by_key(|l| l.chars().count()),
        "Sort Numeric" => lines.sort_by(|a, b| match (leading_number(a), leading_number(b)) {
            (Some(x), Some(y)) => x.total_cmp(&y),
            (Some(_), None) => Ordering::Less,
            (None, Some(_)) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        }),
        "Reverse" => lines.reverse(),
        "Shuffle" => shuffle(&mut lines),
        "Remove Duplicates" => {
            let mut seen = HashSet::new();
            lines.retain(|l| seen.insert(key(l)));
        }
        _ => lines.retain(|l| !l.trim().is_empty()),
    }
    let (prefix, suffix) = (req.options.str("prefix"), req.options.str("suffix"));
    let output = lines
        .iter()
        .map(|l| format!("{prefix}{l}{suffix}"))
        .collect::<Vec<_>>()
        .join("\n");
    let mut result = ToolOutput::text(output);
    if let ToolOutput::Text { metadata, .. } = &mut result {
        metadata.insert("linesRemoved".into(), (before - lines.len()).to_string());
    }
    Ok(result)
}

/// Fisher-Yates driven by std's randomly seeded hasher, to avoid a `rand` dependency.
fn shuffle(lines: &mut [String]) {
    use std::hash::{BuildHasher, Hasher};
    let state = std::collections::hash_map::RandomState::new();
    for i in (1..lines.len()).rev() {
        let mut hasher = state.build_hasher();
        hasher.write_usize(i);
        lines.swap(i, (hasher.finish() % (i as u64 + 1)) as usize);
    }
}

pub fn text_statistics_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "text-statistics",
            name: "Text Statistics",
            category: Category::Text,
            description: "Analyze text for word count, character frequency, and readability",
            keywords: vec![
                "count",
                "words",
                "characters",
                "statistics",
                "readability",
                "frequency",
            ],
            icon: "chart-bar",
            input_placeholder: "Enter text to analyze...",
            output_placeholder: "Statistics will appear here",
            primary_action: "Analyze",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                toggle(
                    "showCharFrequency",
                    "Show character frequency",
                    "Display character frequency analysis",
                    true,
                ),
                toggle(
                    "showReadability",
                    "Show readability scores",
                    "Calculate Flesch-Kincaid scores",
                    true,
                ),
                toggle(
                    "showWordFrequency",
                    "Show word frequency",
                    "Display most common words",
                    true,
                ),
            ],
        },
        process: Some(text_statistics_process),
    }
}

fn syllables(word: &str) -> usize {
    let letters: Vec<char> = word
        .to_lowercase()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .collect();
    let is_vowel = |c: &char| "aeiouy".contains(*c);
    let mut count = letters
        .iter()
        .enumerate()
        .filter(|(i, c)| is_vowel(c) && (*i == 0 || !is_vowel(&letters[i - 1])))
        .count();
    if count > 1 && letters.last() == Some(&'e') && !word.to_lowercase().ends_with("le") {
        count -= 1;
    }
    count.max(1)
}

fn duration_label(words: usize, words_per_minute: usize) -> String {
    let minutes = (words as f64 / words_per_minute as f64).round() as usize;
    if minutes < 1 {
        "< 1 min".into()
    } else {
        format!("{minutes} min")
    }
}

/// Top ten entries by count (ties alphabetical), formatted with `format`.
fn top_ten(counts: BTreeMap<String, usize>, format: impl Fn(&str, usize) -> String) -> String {
    let mut entries: Vec<_> = counts.into_iter().collect();
    entries.sort_by_key(|e| std::cmp::Reverse(e.1));
    entries
        .iter()
        .take(10)
        .map(|(k, n)| format(k, *n))
        .collect::<Vec<_>>()
        .join(", ")
}

fn text_statistics_process(req: &Request) -> ToolResult {
    let text = req.input;
    let words: Vec<&str> = text.split_whitespace().collect();
    let characters = text.chars().count();
    let sentences = text
        .split(['.', '!', '?'])
        .filter(|s| !s.trim().is_empty())
        .count();
    let paragraphs = text
        .replace("\r\n", "\n")
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .count();
    let unique: HashSet<String> = words.iter().map(|w| w.to_lowercase()).collect();

    let mut items = vec![
        OutputItem::new("Characters", characters.to_string()),
        OutputItem::new(
            "Characters (no spaces)",
            text.chars()
                .filter(|c| !c.is_whitespace())
                .count()
                .to_string(),
        ),
        OutputItem::new("Words", words.len().to_string()),
        OutputItem::new("Unique words", unique.len().to_string()),
        OutputItem::new("Sentences", sentences.to_string()),
        OutputItem::new("Paragraphs", paragraphs.to_string()),
        OutputItem::new("Lines", text.lines().count().to_string()),
        OutputItem::new("Bytes (UTF-8)", text.len().to_string()),
        OutputItem::new("Reading time", duration_label(words.len(), 200)),
        OutputItem::new("Speaking time", duration_label(words.len(), 130)),
    ];

    if req.options.bool("showReadability") && !words.is_empty() && sentences > 0 {
        let syllable_total: usize = words.iter().map(|w| syllables(w)).sum();
        let words_per_sentence = words.len() as f64 / sentences as f64;
        let syllables_per_word = syllable_total as f64 / words.len() as f64;
        let ease = 206.835 - 1.015 * words_per_sentence - 84.6 * syllables_per_word;
        let grade = 0.39 * words_per_sentence + 11.8 * syllables_per_word - 15.59;
        let level = match ease {
            e if e >= 90.0 => "Very Easy",
            e if e >= 80.0 => "Easy",
            e if e >= 70.0 => "Fairly Easy",
            e if e >= 60.0 => "Standard",
            e if e >= 50.0 => "Fairly Difficult",
            e if e >= 30.0 => "Difficult",
            _ => "Very Difficult",
        };
        items.push(OutputItem::new(
            "Flesch Reading Ease",
            format!("{ease:.1} ({level})"),
        ));
        items.push(OutputItem::new(
            "Flesch-Kincaid Grade",
            format!("{grade:.1}"),
        ));
    }

    if req.options.bool("showCharFrequency") {
        let mut counts = BTreeMap::new();
        for c in text.chars().filter(|c| !c.is_whitespace()) {
            *counts.entry(c.to_lowercase().to_string()).or_insert(0) += 1;
        }
        items.push(OutputItem::new(
            "Top characters",
            top_ten(counts, |c, n| format!("{c}: {n}")),
        ));
    }

    if req.options.bool("showWordFrequency") {
        let mut counts = BTreeMap::new();
        for w in &words {
            let word = w
                .trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase();
            if !word.is_empty() {
                *counts.entry(word).or_insert(0) += 1;
            }
        }
        items.push(OutputItem::new(
            "Top words",
            top_ten(counts, |w, n| format!("{w}: {n}")),
        ));
    }
    Ok(ToolOutput::multiple(items))
}

pub fn whitespace_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "whitespace-tool",
            name: "Whitespace Tool",
            category: Category::Text,
            description: "Remove, normalize, or format whitespace in text",
            keywords: vec![
                "whitespace",
                "trim",
                "spaces",
                "tabs",
                "newline",
                "line endings",
            ],
            icon: "space",
            input_placeholder: "Enter text to process...",
            output_placeholder: "Processed text will appear here",
            primary_action: "Process",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                picker(
                    "operation",
                    "Operation",
                    "Whitespace operation to perform",
                    vec![
                        "Trim all lines",
                        "Trim leading whitespace",
                        "Trim trailing whitespace",
                        "Collapse multiple spaces",
                        "Remove all whitespace",
                        "Remove empty lines",
                        "Tabs to spaces",
                        "Spaces to tabs",
                        "Remove zero-width spaces",
                        "Replace non-breaking spaces",
                        "Visualize whitespace",
                    ],
                ),
                stepper(
                    "tabWidth",
                    "Tab width",
                    "Spaces per tab for conversion",
                    (1, 16, 4),
                ),
                picker(
                    "lineEndings",
                    "Line endings",
                    "Normalize line endings",
                    vec![
                        "Keep original",
                        "Unix (LF)",
                        "Windows (CRLF)",
                        "Classic Mac (CR)",
                    ],
                ),
            ],
        },
        process: Some(whitespace_process),
    }
}

/// Splits into (content, line ending) pairs so original endings can be kept.
fn split_lines(text: &str) -> Vec<(&str, &str)> {
    let bytes = text.as_bytes();
    let (mut lines, mut start, mut i) = (Vec::new(), 0, 0);
    while i < bytes.len() {
        let ending_len = match bytes[i] {
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => 2,
            b'\r' | b'\n' => 1,
            _ => 0,
        };
        if ending_len == 0 {
            i += 1;
        } else {
            lines.push((&text[start..i], &text[i..i + ending_len]));
            i += ending_len;
            start = i;
        }
    }
    lines.push((&text[start..], ""));
    lines
}

fn whitespace_process(req: &Request) -> ToolResult {
    let tab_width = req.options.i64("tabWidth").max(1) as usize;
    let operation = req.options.str("operation");
    let is_blank = |l: &str| l.trim().is_empty();
    let mut lines: Vec<(String, &str)> = Vec::new();
    for (content, ending) in split_lines(req.input) {
        let content = match operation {
            "Trim all lines" => content.trim().to_string(),
            "Trim leading whitespace" => content.trim_start().to_string(),
            "Trim trailing whitespace" => content.trim_end().to_string(),
            "Collapse multiple spaces" => collapse_spaces(content),
            "Remove all whitespace" => content.chars().filter(|c| !c.is_whitespace()).collect(),
            "Remove empty lines" if is_blank(content) => continue,
            "Tabs to spaces" => content.replace('\t', &" ".repeat(tab_width)),
            "Spaces to tabs" => spaces_to_tabs(content, tab_width),
            "Remove zero-width spaces" => content
                .chars()
                .filter(|c| {
                    !matches!(
                        c,
                        '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{2060}' | '\u{FEFF}'
                    )
                })
                .collect(),
            "Replace non-breaking spaces" => content.replace('\u{a0}', " "),
            "Visualize whitespace" => visualize(content),
            _ => content.to_string(),
        };
        // Line breaks are whitespace too.
        let ending = if operation == "Remove all whitespace" {
            ""
        } else {
            ending
        };
        lines.push((content, ending));
    }
    let new_ending = match req.options.str("lineEndings") {
        "Unix (LF)" => Some("\n"),
        "Windows (CRLF)" => Some("\r\n"),
        "Classic Mac (CR)" => Some("\r"),
        _ => None,
    };
    let visualize = operation == "Visualize whitespace";
    let mut out = String::new();
    for (content, ending) in lines {
        out.push_str(&content);
        let ending = new_ending.filter(|_| !ending.is_empty()).unwrap_or(ending);
        if visualize && !ending.is_empty() {
            out.push_str(if ending.starts_with('\r') && ending.len() == 2 {
                "␍¶"
            } else if ending == "\r" {
                "␍"
            } else {
                "¶"
            });
        }
        out.push_str(ending);
    }
    Ok(ToolOutput::text(out))
}

fn collapse_spaces(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut previous_blank = false;
    for c in line.chars() {
        let blank = c == ' ' || c == '\t';
        if !(blank && previous_blank) {
            out.push(if blank { ' ' } else { c });
        }
        previous_blank = blank;
    }
    out
}

fn spaces_to_tabs(line: &str, tab_width: usize) -> String {
    let indent = line.chars().take_while(|c| *c == ' ').count();
    format!(
        "{}{}{}",
        "\t".repeat(indent / tab_width),
        " ".repeat(indent % tab_width),
        &line[indent..]
    )
}

fn visualize(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    for c in line.chars() {
        match c {
            ' ' => out.push('·'),
            '\t' => out.push('→'),
            '\u{a0}' => out.push('°'),
            '\u{200B}' => out.push_str("[ZWSP]"),
            '\u{2003}' => out.push_str("[EM]"),
            c => out.push(c),
        }
    }
    out
}
