//! Resource-limit and input-validation cases: the release build aborts on panics,
//! stack overflows and huge allocations, so hostile inputs must fail cleanly.

use std::time::{Duration, Instant};

use serde_json::{json, Map, Value};
use trayforge_core::{run, ToolError};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn go(id: &str, input: &str, options: Value, reverse: bool) -> Result<String, ToolError> {
    run(id, input, &opts(options), reverse).map(|o| o.as_text().to_string())
}

fn nested_xml(depth: usize) -> String {
    format!("{}{}", "<a>".repeat(depth), "</a>".repeat(depth))
}

#[test]
fn xml_depth_limit() {
    for id in ["xml-formatter", "xml-json"] {
        assert!(go(id, &nested_xml(100), json!({}), false).is_ok(), "{id}");
        let err = go(id, &nested_xml(300), json!({}), false).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)), "{id}: {err:?}");
    }
}

#[test]
fn graphql_depth_limit() {
    let nested = |depth: usize| format!("{}{}", "{a".repeat(depth), "}".repeat(depth));
    assert!(go("graphql-formatter", &nested(100), json!({}), false).is_ok());
    let err = go("graphql-formatter", &nested(300), json!({}), false).unwrap_err();
    assert!(matches!(err, ToolError::InvalidInput(_)), "{err:?}");
}

#[test]
fn deep_nesting_keeps_output_linear() {
    let start = Instant::now();
    let out = go(
        "js-minifier",
        &"{".repeat(20_000),
        json!({"indent": 8}),
        true,
    )
    .unwrap();
    assert!(out.len() < 10_000_000, "{} bytes", out.len());
    assert!(start.elapsed() < Duration::from_secs(5));

    let css = go(
        "css-minifier",
        &"a{".repeat(20_000),
        json!({"indent": 8}),
        true,
    )
    .unwrap();
    assert!(css.len() < 10_000_000, "{} bytes", css.len());
}

#[test]
fn json_path_match_cap() {
    let doc = format!("{}{}", "[".repeat(100), "]".repeat(100));
    for path in ["$..*..*..*", "$..*..*..*..*"] {
        let result = go("json-path", &doc, json!({ "path": path }), false);
        assert!(
            matches!(&result, Err(ToolError::ProcessingFailed(m)) if m.contains("Too many matches")),
            "{path}: {result:?}"
        );
    }
}

#[test]
fn jwt_expired_with_extreme_exp_does_not_overflow() {
    let token = "e30.eyJleHAiOi0xZTMwOH0.sig";
    let out = go("jwt-decoder", token, json!({}), false).unwrap();
    assert!(out.contains("Expired"), "{out}");
}

#[test]
fn yaml_special_floats() {
    for s in [".inf", "+.inf", "-.inf", ".nan", ".Inf", "inf", "NaN"] {
        let input = json!({ "a": s }).to_string();
        let out = go("json-yaml", &input, json!({}), false).unwrap();
        assert!(
            out.contains(&format!("\"{s}\"")) || out.contains(&format!("'{s}'")),
            "{s}: {out}"
        );
    }
    for yaml in ["a: .inf", "a: -.inf", "a: .nan"] {
        let err = go("json-yaml", yaml, json!({}), true).unwrap_err();
        assert!(matches!(err, ToolError::InvalidInput(_)), "{yaml}: {err:?}");
    }
    assert!(go("json-yaml", "a: 1.5", json!({}), true).is_ok());
}

#[test]
fn signed_digits_are_rejected() {
    assert!(go("url-encode", "%+1%+F", json!({}), true).is_err());
    assert_eq!(go("url-encode", "%41", json!({}), true).unwrap(), "A");
    assert_eq!(
        go("html-entity", "&#+65; &#x+41; &#65;", json!({}), true).unwrap(),
        "&#+65; &#x+41; A"
    );
    let no_options = Map::new();
    assert!(run("base-converter", "0x+ff", &no_options, false).is_err());
    assert!(run("base-converter", "-+5", &no_options, false).is_err());
    assert!(run("ip-address", "10.0.0.1/+8", &no_options, false).is_err());
    assert!(trayforge_core::totp::parse_uri("otpauth://totp/a%+1b?secret=ABC").is_err());
    assert!(trayforge_core::totp::parse_uri("otpauth://totp/a?secret=ABC&issuer=%+1").is_err());
}

#[test]
fn json_to_xml_rejects_bad_attribute_names() {
    let err = go("xml-json", r#"{"r":{"@a b":"x"}}"#, json!({}), true).unwrap_err();
    assert!(matches!(err, ToolError::InvalidInput(m) if m.contains("attribute")));
    assert!(go("xml-json", r#"{"r":{"@a":"x"}}"#, json!({}), true).is_ok());
}

#[test]
fn markdown_safe_mode_sanitizes_image_urls() {
    let safe = go(
        "markdown-preview",
        "![x](javascript:alert(1))",
        json!({"allowRawHtml": false}),
        false,
    )
    .unwrap();
    assert!(!safe.contains("javascript"), "{safe}");
}

#[test]
fn regex_tester_offsets_stay_correct_and_fast() {
    let out = go("regex-tester", "b\nабвb ббb", json!({}), false).unwrap();
    assert_eq!(
        out,
        "Match 1: \"b\" (position 3-4)\nMatch 2: \"b\" (position 7-8)"
    );
    let start = Instant::now();
    let text = format!(".\n{}", "a".repeat(200_000));
    go("regex-tester", &text, json!({"limit": 200_000}), false).ok();
    assert!(start.elapsed() < Duration::from_secs(5));
}
