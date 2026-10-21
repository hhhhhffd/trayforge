//! Reference cases (case id in each test name) plus checks for behaviors
//! the cases do not cover.

use serde_json::{json, Map, Value};
use trayforge_core::{run, ToolError, ToolOutput};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn go(id: &str, input: &str, options: Value) -> Result<ToolOutput, ToolError> {
    run(id, input, &opts(options), false)
}

fn text(id: &str, input: &str, options: Value) -> String {
    go(id, input, options).unwrap().as_text().to_string()
}

fn items(id: &str, input: &str, options: Value) -> Vec<(String, String)> {
    go(id, input, options)
        .unwrap()
        .items()
        .iter()
        .map(|i| (i.label.clone(), i.value.clone()))
        .collect()
}

fn value_of(items: &[(String, String)], label: &str) -> String {
    items
        .iter()
        .find(|(l, _)| l == label)
        .unwrap_or_else(|| panic!("no item {label}"))
        .1
        .clone()
}

#[test]
fn registry_lists_text_tools_in_order() {
    let ids: Vec<_> = trayforge_core::registry()
        .iter()
        .filter(|t| t.info.category == trayforge_core::Category::Text)
        .map(|t| t.info.id)
        .collect();
    assert_eq!(
        ids,
        [
            "string-case",
            "regex-tester",
            "regex-library",
            "string-diff",
            "line-sorter",
            "text-statistics",
            "string-escape",
            "whitespace-tool",
            "base-converter",
            "unix-timestamp",
            "epoch-batch-converter",
            "slug-generator"
        ]
    );
}

#[test]
fn string_case_acronym() {
    assert_eq!(
        text("string-case", "HTTP request_id FooBAR", json!({})),
        "httpRequestIdFooBar"
    );
}

#[test]
fn string_case_formats() {
    let input = "HTTP request_id FooBAR";
    let convert = |format: &str, preserve: bool| {
        text(
            "string-case",
            input,
            json!({"format": format, "preserveAcronyms": preserve}),
        )
    };
    assert_eq!(convert("camelCase", true), "HTTPRequestIdFooBAR");
    assert_eq!(convert("PascalCase", false), "HttpRequestIdFooBar");
    assert_eq!(convert("snake_case", false), "http_request_id_foo_bar");
    assert_eq!(
        convert("SCREAMING_SNAKE_CASE", false),
        "HTTP_REQUEST_ID_FOO_BAR"
    );
    assert_eq!(convert("kebab-case", false), "http-request-id-foo-bar");
    assert_eq!(convert("Title Case", false), "Http Request Id Foo Bar");
    assert_eq!(convert("Sentence case", false), "Http request id foo bar");
    assert_eq!(convert("UPPERCASE", false), "HTTP REQUEST_ID FOOBAR");
}

#[test]
fn sort_lines() {
    assert_eq!(
        text("line-sorter", "b\nA\na\n b \n10\n2\n", json!({})),
        "\n10\n2\nA\na\nb\nb"
    );
}

#[test]
fn line_sorter_operations() {
    let run_op = |operation: &str, input: &str, extra: Value| {
        let mut options = opts(extra);
        options.insert("operation".into(), json!(operation));
        text("line-sorter", input, Value::Object(options))
    };
    assert_eq!(run_op("Sort Z-A", "a\nc\nb", json!({})), "c\nb\na");
    assert_eq!(
        run_op("Sort Numeric", "10\n2\n1.5", json!({})),
        "1.5\n2\n10"
    );
    assert_eq!(
        run_op("Sort by Length", "ccc\na\nbb", json!({})),
        "a\nbb\nccc"
    );
    assert_eq!(run_op("Reverse", "1\n2\n3", json!({})), "3\n2\n1");
    assert_eq!(run_op("Remove Duplicates", "a\nA\nb\na", json!({})), "a\nb");
    assert_eq!(
        run_op("Remove Duplicates", "a\nA", json!({"caseSensitive": true})),
        "a\nA"
    );
    assert_eq!(
        run_op("Remove Empty Lines", "a\n\n  \nb", json!({})),
        "a\nb"
    );
    assert_eq!(
        run_op("Sort A-Z", "b\na", json!({"prefix": "- ", "suffix": ";"})),
        "- a;\n- b;"
    );
    let shuffled = run_op("Shuffle", "1\n2\n3\n4\n5", json!({}));
    let mut sorted: Vec<_> = shuffled.split('\n').collect();
    sorted.sort();
    assert_eq!(sorted, ["1", "2", "3", "4", "5"]);
}

#[test]
fn slug_multilingual() {
    let result = items(
        "slug-generator",
        "Привет мир! Café déjà vu — test",
        json!({}),
    );
    assert_eq!(value_of(&result, "Slug"), "privet-mir-cafe-deja-vu-test");
    assert_eq!(value_of(&result, "Length"), "28 characters");
    assert_eq!(
        value_of(&result, "URL preview"),
        "https://example.com/privet-mir-cafe-deja-vu-test"
    );
    assert_eq!(value_of(&result, "Words"), "7 → 6");
}

#[test]
fn slug_options() {
    let slug =
        |input: &str, options: Value| value_of(&items("slug-generator", input, options), "Slug");
    assert_eq!(
        slug(
            "The Quick Fox",
            json!({"separator": "_", "lowercase": false})
        ),
        "The_Quick_Fox"
    );
    assert_eq!(
        slug("The Quick Fox is here", json!({"removeStopWords": true})),
        "quick-fox-here"
    );
    assert_eq!(
        slug("hello wonderful world", json!({"maxLength": 12})),
        "hello-wonder"
    );
    assert_eq!(
        slug("Crème brûlée", json!({"transliterate": false})),
        "crème-brûlée"
    );
    assert!(go("slug-generator", "!!!", json!({})).is_err());
}

#[test]
fn base_hex() {
    let result = items("base-converter", "0xFF", json!({}));
    assert_eq!(value_of(&result, "Decimal (10)"), "255");
    assert_eq!(value_of(&result, "Binary (2)"), "0b1111 1111");
    assert_eq!(value_of(&result, "Octal (8)"), "0o377");
    assert_eq!(value_of(&result, "Hexadecimal (16)"), "0xFF");
    assert_eq!(value_of(&result, "Bits required"), "8");
    assert_eq!(
        value_of(&result, "Detected input base"),
        "Hexadecimal (auto)"
    );
}

#[test]
fn base_converter_variants() {
    let plain = json!({"showPrefixes": false, "groupDigits": false, "uppercaseHex": false});
    let result = items("base-converter", "0b101010", plain.clone());
    assert_eq!(value_of(&result, "Decimal (10)"), "42");
    assert_eq!(value_of(&result, "Hexadecimal (16)"), "2a");
    assert_eq!(value_of(&result, "Detected input base"), "Binary (auto)");

    let result = items("base-converter", "1234567", json!({}));
    assert_eq!(value_of(&result, "Decimal (10)"), "1,234,567");

    let result = items(
        "base-converter",
        "-ff",
        json!({"inputBase": "Hexadecimal (16)", "showPrefixes": false}),
    );
    assert_eq!(value_of(&result, "Decimal (10)"), "-255");

    let result = items(
        "base-converter",
        "zz",
        json!({"inputBase": "Custom", "customBase": 36}),
    );
    assert_eq!(value_of(&result, "Decimal (10)"), "1,295");
    assert_eq!(value_of(&result, "Base 36"), "ZZ");

    assert!(matches!(
        go("base-converter", "12", json!({"inputBase": "Binary (2)"})),
        Err(ToolError::InvalidInput(_))
    ));
    assert!(go("base-converter", &"9".repeat(60), json!({})).is_err());
}

#[test]
fn regex_tester_matches_and_replacement() {
    let out = text(
        "regex-tester",
        "\\d+\nThere are 42 apples and 7 oranges",
        json!({}),
    );
    assert_eq!(
        out,
        "Match 1: \"42\" (position 10-12)\nMatch 2: \"7\" (position 24-25)"
    );
    let out = text(
        "regex-tester",
        "(\\w)(\\d)\nA1 b2",
        json!({"replacement": "$2$1"}),
    );
    assert!(out.contains("Match 1: \"A1\" (position 0-2)\n  Group 1: \"A\"\n  Group 2: \"1\""));
    assert!(out.ends_with("Replacement result:\n1A 2b"));
    assert_eq!(
        text("regex-tester", "x\nabc", json!({})),
        "No matches found"
    );
}

#[test]
fn regex_tester_flags_and_lookaround() {
    assert!(text(
        "regex-tester",
        "HELLO\nhello",
        json!({"caseInsensitive": true})
    )
    .contains("\"hello\""));
    assert_eq!(
        text("regex-tester", "HELLO\nhello", json!({})),
        "No matches found"
    );
    assert!(text("regex-tester", "^b\na\nb", json!({"multiline": true})).contains("\"b\""));
    assert!(text(
        "regex-tester",
        "a.b\na\nb",
        json!({"dotMatchesNewlines": true})
    )
    .contains("Match 1"));
    assert!(text("regex-tester", "\\d+(?=px)\n12px 3em", json!({})).contains("\"12\""));
}

#[test]
fn regex_tester_errors() {
    assert_eq!(
        go("regex-tester", "\\d+", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Please provide test string after the pattern"
    );
    assert_eq!(
        go("regex-tester", "\nabc", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: First line must be the regex pattern"
    );
    assert!(matches!(
        go("regex-tester", "(\nabc", json!({})),
        Err(ToolError::InvalidInput(_))
    ));
}

#[test]
fn regex_library_pattern_and_matches() {
    let out = text("regex-library", "", json!({}));
    assert!(out.starts_with("Email & Communication: Email Address"));
    assert!(out.contains("--- Copy Formats ---"));
    assert!(!out.contains("--- Test Results ---"));

    let out = text("regex-library", "write a@b.co or c@d.org", json!({}));
    assert!(out.contains("--- Test Results ---\nMatch 1: \"a@b.co\""));
    assert!(out.contains("Match 2: \"c@d.org\""));
    let out = text(
        "regex-library",
        "write a@b.co or c@d.org",
        json!({"showAllMatches": false}),
    );
    assert!(!out.contains("Match 2"));

    let out = text("regex-library", "", json!({"category": "Numbers"}));
    assert!(out.starts_with("Numbers: Integer"));
    let out = text(
        "regex-library",
        "x",
        json!({"pattern": "Numbers: Percentage"}),
    );
    assert!(out.contains("No matches found"));
}

#[test]
fn string_diff_unified_and_modes() {
    assert_eq!(
        text("string-diff", "a\nb\nc\n---\na\nx\nc", json!({})),
        "--- Text 1\n+++ Text 2\n  a\n- b\n+ x\n  c\n\n1 added, 1 removed"
    );
    assert_eq!(
        text("string-diff", "a\nb\n---\na\nx", json!({"unified": false})),
        "- b\n+ x\n\n1 added, 1 removed"
    );
    assert_eq!(
        text("string-diff", "same\n---\nsame", json!({})),
        "Texts are identical"
    );
    assert_eq!(
        text(
            "string-diff",
            "one two three\n---\none 2 three",
            json!({"level": "Words"})
        ),
        "one [-two-] {+2+} three\n\n1 added, 1 removed"
    );
    assert_eq!(
        text(
            "string-diff",
            "cat\n---\ncut",
            json!({"level": "Characters"})
        ),
        "c[-a-]{+u+}t\n\n1 added, 1 removed"
    );
    assert_eq!(
        text(
            "string-diff",
            "Hello  World\n---\nhello world",
            json!({"ignoreCase": true, "ignoreWhitespace": true})
        ),
        "Texts are identical"
    );
    assert_eq!(
        go("string-diff", "no separator", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Please separate two texts with a line containing only '---'"
    );
}

#[test]
fn text_statistics_counts() {
    let result = items(
        "text-statistics",
        "The cat sat. The dog ran!\n\nNew paragraph here?",
        json!({}),
    );
    assert_eq!(value_of(&result, "Words"), "9");
    assert_eq!(value_of(&result, "Unique words"), "8");
    assert_eq!(value_of(&result, "Sentences"), "3");
    assert_eq!(value_of(&result, "Paragraphs"), "2");
    assert_eq!(value_of(&result, "Lines"), "3");
    assert_eq!(value_of(&result, "Bytes (UTF-8)"), "46");
    assert_eq!(value_of(&result, "Reading time"), "< 1 min");
    assert!(value_of(&result, "Top words").starts_with("the: 2"));
    assert!(value_of(&result, "Flesch Reading Ease").contains('('));

    let result = items(
        "text-statistics",
        "héllo",
        json!({"showCharFrequency": false, "showReadability": false, "showWordFrequency": false}),
    );
    assert_eq!(value_of(&result, "Characters"), "5");
    assert_eq!(value_of(&result, "Bytes (UTF-8)"), "6");
    assert!(result
        .iter()
        .all(|(l, _)| !l.starts_with("Top") && !l.starts_with("Flesch")));
}

#[test]
fn string_escape_round_trips() {
    let escape = |input: &str, format: &str| {
        run(
            "string-escape",
            input,
            &opts(json!({"format": format})),
            false,
        )
        .unwrap()
        .as_text()
        .to_string()
    };
    let unescape = |input: &str, format: &str| {
        run(
            "string-escape",
            input,
            &opts(json!({"format": format})),
            true,
        )
        .unwrap()
        .as_text()
        .to_string()
    };
    let sample = "say \"hi\"\n\tback\\slash 'q' \u{1}";
    assert_eq!(
        escape(sample, "JavaScript/JSON"),
        "say \\\"hi\\\"\\n\\tback\\\\slash 'q' \\u0001"
    );
    assert_eq!(
        escape(sample, "Python"),
        "say \\\"hi\\\"\\n\\tback\\\\slash \\'q\\' \\x01"
    );
    for format in ["JavaScript/JSON", "Python", "Java", "C/C++", "Swift"] {
        assert_eq!(
            unescape(&escape(sample, format), format),
            sample,
            "{format}"
        );
    }
    assert_eq!(
        unescape("\\u00e9\\uD83D\\uDE00\\x41\\u{1F600}", "Swift"),
        "é😀A😀"
    );
    assert_eq!(escape("it's", "SQL"), "it''s");
    assert_eq!(unescape("it''s", "SQL"), "it's");
    assert_eq!(escape("a.b*c", "Regex"), "a\\.b\\*c");
    assert_eq!(unescape("a\\.b\\*c\\d", "Regex"), "a.b*c\\d");
    assert_eq!(
        escape("<a href=\"x\">&'", "HTML"),
        "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
    );
    assert_eq!(
        unescape("&lt;b&gt; &amp;amp; &#65;&#x42; &bogus;", "HTML"),
        "<b> &amp; AB &bogus;"
    );
    assert_eq!(escape("a b/é", "URL"), "a%20b%2F%C3%A9");
    assert_eq!(unescape("a%20b%2F%C3%A9", "URL"), "a b/é");
    assert!(run(
        "string-escape",
        "%zz",
        &opts(json!({"format": "URL"})),
        true
    )
    .is_err());
    assert!(run("string-escape", "\\uD83D", &opts(json!({})), true).is_err());
}

#[test]
fn whitespace_operations() {
    let op = |operation: &str, input: &str, extra: Value| {
        let mut options = opts(extra);
        options.insert("operation".into(), json!(operation));
        text("whitespace-tool", input, Value::Object(options))
    };
    assert_eq!(
        op("Trim all lines", "  a  \r\n\tb\t\n", json!({})),
        "a\r\nb\n"
    );
    assert_eq!(
        op("Trim leading whitespace", "  a  \n b", json!({})),
        "a  \nb"
    );
    assert_eq!(
        op("Trim trailing whitespace", "a  \nb\t", json!({})),
        "a\nb"
    );
    assert_eq!(
        op("Collapse multiple spaces", "a   b \t c", json!({})),
        "a b c"
    );
    assert_eq!(op("Remove all whitespace", " a b\n c ", json!({})), "abc");
    assert_eq!(
        op("Remove empty lines", "a\n\n  \nb\n", json!({})),
        "a\nb\n"
    );
    assert_eq!(
        op("Tabs to spaces", "\ta\tb", json!({"tabWidth": 2})),
        "  a  b"
    );
    assert_eq!(op("Spaces to tabs", "     x", json!({})), "\t x");
    assert_eq!(
        op("Remove zero-width spaces", "a\u{200B}b\u{FEFF}", json!({})),
        "ab"
    );
    assert_eq!(
        op("Replace non-breaking spaces", "a\u{a0}b", json!({})),
        "a b"
    );
    assert_eq!(
        op("Visualize whitespace", "a b\t\r\nc", json!({})),
        "a·b→␍¶\r\nc"
    );
    assert_eq!(
        op(
            "Trim all lines",
            "a \r\nb \rc",
            json!({"lineEndings": "Unix (LF)"})
        ),
        "a\nb\nc"
    );
    assert_eq!(
        op(
            "Trim all lines",
            "a\nb",
            json!({"lineEndings": "Windows (CRLF)"})
        ),
        "a\r\nb"
    );
    assert_eq!(
        op(
            "Trim all lines",
            "a\nb",
            json!({"lineEndings": "Classic Mac (CR)"})
        ),
        "a\rb"
    );
}

#[test]
fn unix_timestamp_fixed_values() {
    let utc = json!({"timezone": "UTC"});
    let result = items("unix-timestamp", "1708963200", utc.clone());
    assert_eq!(value_of(&result, "Unix (seconds)"), "1708963200");
    assert_eq!(value_of(&result, "Unix (milliseconds)"), "1708963200000");
    assert_eq!(value_of(&result, "ISO 8601"), "2024-02-26T16:00:00Z");
    assert_eq!(value_of(&result, "Standard"), "2024-02-26 16:00:00");
    assert_eq!(
        value_of(&result, "Long format"),
        "Monday, February 26, 2024 at 4:00 PM"
    );
    assert_eq!(value_of(&result, "US format"), "02/26/2024 04:00:00 PM");
    assert_eq!(value_of(&result, "EU format"), "26/02/2024 16:00:00");
    assert_eq!(value_of(&result, "Day of year"), "57 (Week 09)");
    assert!(value_of(&result, "Relative").ends_with("years ago"));

    // Milliseconds are auto-detected and other timezones applied.
    let result = items(
        "unix-timestamp",
        "1708963200000",
        json!({"timezone": "JST", "showAllFormats": false, "showRelative": false}),
    );
    assert_eq!(value_of(&result, "ISO 8601"), "2024-02-27T01:00:00+09:00");
    assert_eq!(result.len(), 3);

    let result = items(
        "unix-timestamp",
        "1708963200000000",
        json!({"timezone": "UTC"}),
    );
    assert_eq!(value_of(&result, "Unix (seconds)"), "1708963200");
    let result = items(
        "unix-timestamp",
        "1708963200",
        json!({"timezone": "UTC", "precision": "Milliseconds"}),
    );
    assert_eq!(value_of(&result, "Unix (seconds)"), "1708963");

    // Dates are read in the selected timezone, explicit offsets win.
    let result = items(
        "unix-timestamp",
        "2024-02-26 16:00:00",
        json!({"timezone": "EST"}),
    );
    assert_eq!(value_of(&result, "Unix (seconds)"), "1708981200");
    assert_eq!(value_of(&result, "ISO 8601"), "2024-02-26T16:00:00-05:00");
    let result = items("unix-timestamp", "2024-02-26T12:00:00+02:00", utc);
    assert_eq!(value_of(&result, "Unix (seconds)"), "1708941600");
}

#[test]
fn unix_timestamp_now_and_errors() {
    let before = chrono_free_now();
    let result = items("unix-timestamp", "", json!({"timezone": "UTC"}));
    let seconds: i64 = value_of(&result, "Unix (seconds)").parse().unwrap();
    assert!(seconds >= before && seconds <= before + 5);
    assert_eq!(value_of(&result, "Relative"), "just now");
    assert_eq!(
        go("unix-timestamp", "nope", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Could not parse 'nope' as timestamp or date"
    );
}

fn chrono_free_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

#[test]
fn epoch_batch_to_dates_and_back() {
    let input = "1708963200\n2024-02-26T12:00:00Z\n1708963200000\n\nnot a date";
    assert_eq!(
        text("epoch-batch-converter", input, json!({"timezone": "UTC"})),
        "1708963200 → 2024-02-26T16:00:00Z\n2024-02-26T12:00:00Z → 2024-02-26T12:00:00Z\n1708963200000 → 2024-02-26T16:00:00Z"
    );
    assert_eq!(
        text(
            "epoch-batch-converter",
            input,
            json!({"timezone": "UTC", "includeOriginal": false, "skipInvalid": false, "outputFormat": "RFC 2822"})
        ),
        "Mon, 26 Feb 2024 16:00:00 +0000\nMon, 26 Feb 2024 12:00:00 +0000\nMon, 26 Feb 2024 16:00:00 +0000\nInvalid format"
    );
    assert_eq!(
        text(
            "epoch-batch-converter",
            "1708963200",
            json!({"timezone": "UTC", "outputFormat": "Custom", "includeOriginal": false, "customFormat": "yyyy-MM-dd HH:mm:ss"})
        ),
        "2024-02-26 16:00:00"
    );
    let reversed = run(
        "epoch-batch-converter",
        "2024-02-26T12:00:00Z\n2024-02-26 12:00:00",
        &opts(json!({"timezone": "UTC", "includeOriginal": false})),
        true,
    )
    .unwrap();
    assert_eq!(reversed.as_text(), "1708948800\n1708948800");
    assert_eq!(
        run(
            "epoch-batch-converter",
            "26/02/2024 12:00",
            &opts(json!({"timezone": "UTC", "inputFormat": "Custom", "customFormat": "dd/MM/yyyy HH:mm", "includeOriginal": false})),
            true,
        )
        .unwrap()
        .as_text(),
        "1708948800"
    );
}

#[test]
fn epoch_batch_errors() {
    assert_eq!(
        go("epoch-batch-converter", "garbage", json!({}))
            .unwrap_err()
            .to_string(),
        "Processing failed: No valid timestamps found"
    );
}
