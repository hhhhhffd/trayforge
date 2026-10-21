//! Reference cases (case id in each test name).

use serde_json::{json, Map, Value};
use trayforge_core::{run, ToolError};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn encode(id: &str, input: &str, options: Value) -> Result<String, ToolError> {
    run(id, input, &opts(options), false).map(|o| o.as_text().to_string())
}

fn decode(id: &str, input: &str, options: Value) -> Result<String, ToolError> {
    run(id, input, &opts(options), true).map(|o| o.as_text().to_string())
}

#[test]
fn base64_unicode() {
    assert_eq!(
        encode("base64", "Привет 👋\n", json!({})).unwrap(),
        "0J/RgNC40LLQtdGCIPCfkYsK"
    );
}

#[test]
fn base64_invalid() {
    let err = decode("base64", "%%%", json!({})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Invalid Base64 string. Make sure the input is properly encoded."
    );
}

#[test]
fn base64_unpadded() {
    assert_eq!(decode("base64", "SGk", json!({})).unwrap(), "Hi");
}

#[test]
fn base64_crlf() {
    assert_eq!(decode("base64", " S\r\nGk= \n", json!({})).unwrap(), "Hi");
}

#[test]
fn base64_inner_space() {
    assert!(matches!(
        decode("base64", "S Gk=", json!({})),
        Err(ToolError::InvalidInput(_))
    ));
}

#[test]
fn base64_urlsafe_auto() {
    assert_eq!(decode("base64", "8J-YgA", json!({})).unwrap(), "😀");
}

#[test]
fn base64_url_safe_without_padding() {
    assert_eq!(
        encode(
            "base64",
            "😀?",
            json!({"variant": "URL-Safe", "padding": false})
        )
        .unwrap(),
        "8J-YgD8"
    );
}

#[test]
fn base64_rejects_unknown_option_value() {
    assert!(matches!(
        encode("base64", "x", json!({"variant": "Nope"})),
        Err(ToolError::InvalidOptions(_))
    ));
}

#[test]
fn url_query() {
    assert_eq!(
        encode("url-encode", "a b+c/д?x=1&y=2", json!({})).unwrap(),
        "a%20b%2Bc/%D0%B4%3Fx%3D1%26y%3D2"
    );
}

#[test]
fn url_modes_and_decode() {
    let full = json!({"encodeMode": "Full URL", "spaceEncoding": "+"});
    assert_eq!(
        encode("url-encode", "http://a/b c?x=д", full).unwrap(),
        "http://a/b+c?x=%D0%B4"
    );
    assert_eq!(
        decode("url-encode", "b+c%D0%B4", json!({"spaceEncoding": "+"})).unwrap(),
        "b cд"
    );
    assert_eq!(
        encode("url-encode", "a/b", json!({"encodeSlashes": true})).unwrap(),
        "a%2Fb"
    );
}

#[test]
fn url_invalid_percent() {
    let err = decode("url-encode", "%zz", json!({})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Invalid percent-encoded string"
    );
    assert!(decode("url-encode", "%C3", json!({})).is_err());
}

#[test]
fn html_entities() {
    assert_eq!(
        encode("html-entity", "<a title=\"x\">& Привет 😀</a>", json!({})).unwrap(),
        "&lt;a title=&quot;x&quot;&gt;&amp; Привет 😀&lt;/a&gt;"
    );
}

#[test]
fn html_entity_formats_and_decode() {
    assert_eq!(
        encode(
            "html-entity",
            "é<😀",
            json!({"entityFormat": "Hexadecimal", "encodeAll": true})
        )
        .unwrap(),
        "&#xE9;&#x3C;&#x1F600;"
    );
    assert_eq!(
        encode("html-entity", "©", json!({"encodeAll": true})).unwrap(),
        "&copy;"
    );
    assert_eq!(
        decode(
            "html-entity",
            "&lt;&amp;lt;&#233;&#x1F600;&copy;&bogus; &",
            json!({})
        )
        .unwrap(),
        "<&lt;é😀©&bogus; &"
    );
}

#[test]
fn jwt_decodes_header_and_payload() {
    let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwiaWF0IjoxNTE2MjM5MDIyfQ.sig";
    let text = encode("jwt-decoder", token, json!({})).unwrap();
    assert!(text.contains("\"alg\": \"HS256\""));
    assert!(text.contains("\"sub\": \"1234567890\""));
    assert!(text.contains("Issued At (iat): 2018-01-18 01:30:22 UTC"));
    assert!(text.contains("Signature verification requires the secret key"));
}

#[test]
fn jwt_errors() {
    assert_eq!(
        encode("jwt-decoder", "abc.def", json!({})).unwrap_err().to_string(),
        "Invalid input: Invalid JWT format. Token should have 3 parts separated by dots (header.payload.signature)"
    );
    assert_eq!(
        encode("jwt-decoder", "!!.e30.x", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Failed to decode JWT header"
    );
    assert_eq!(
        encode("jwt-decoder", "e30.!!.x", json!({}))
            .unwrap_err()
            .to_string(),
        "Invalid input: Failed to decode JWT payload"
    );
}

#[test]
fn unicode_emoji() {
    assert_eq!(
        encode("unicode-escape", "Aé😀", json!({})).unwrap(),
        "A\\u00E9\\uD83D\\uDE00"
    );
}

#[test]
fn unicode_formats_and_decode() {
    assert_eq!(
        encode("unicode-escape", "é😀", json!({"format": "U+XXXX"})).unwrap(),
        "U+00E9U+1F600"
    );
    assert_eq!(
        encode(
            "unicode-escape",
            "é",
            json!({"format": "\\xXX", "uppercase": false})
        )
        .unwrap(),
        "\\xe9"
    );
    assert_eq!(
        decode(
            "unicode-escape",
            "A\\u00e9\\uD83D\\uDE00\\x41U+1F600\\U0001F600\\uD83D",
            json!({})
        )
        .unwrap(),
        "Aé😀A😀😀\\uD83D"
    );
}

#[test]
fn binary_utf8() {
    assert_eq!(
        encode("text-to-binary", "Aя", json!({})).unwrap(),
        "01000001 11010001 10001111"
    );
}

#[test]
fn binary_formats_and_decode() {
    let hex = json!({"outputFormat": "Hexadecimal", "showPrefix": true, "byteSeparator": "Comma"});
    assert_eq!(
        encode("text-to-binary", "Aя", hex.clone()).unwrap(),
        "0x41,0xD1,0x8F"
    );
    assert_eq!(
        decode("text-to-binary", "0x41,0xD1,0x8F", hex).unwrap(),
        "Aя"
    );
    assert_eq!(
        decode("text-to-binary", "01000001 11010001 10001111", json!({})).unwrap(),
        "Aя"
    );
    assert_eq!(
        decode("text-to-binary", "0100000101000010", json!({})).unwrap(),
        "AB"
    );
    assert_eq!(
        decode(
            "text-to-binary",
            "72 105",
            json!({"outputFormat": "Decimal"})
        )
        .unwrap(),
        "Hi"
    );
    assert_eq!(
        encode("text-to-binary", "Hi", json!({"outputFormat": "Octal"})).unwrap(),
        "110 151"
    );
}

#[test]
fn binary_decode_errors() {
    assert!(matches!(
        decode("text-to-binary", "0102", json!({})),
        Err(ToolError::InvalidInput(_))
    ));
    assert_eq!(
        decode("text-to-binary", "11111111", json!({}))
            .unwrap_err()
            .to_string(),
        "Processing failed: Could not decode as UTF-8 text"
    );
}

#[test]
fn morse_sos() {
    assert_eq!(
        encode("morse-code", "SOS TEST 123", json!({})).unwrap(),
        "···   −−−   ···       −   ·   ···   −       ·−−−−   ··−−−   ···−−"
    );
}

#[test]
fn morse_decode_roundtrip() {
    let original = "SOS TEST 123";
    for options in [
        json!({}),
        json!({"symbols": "./-"}),
        json!({"letterSpacing": false}),
    ] {
        let morse = encode("morse-code", original, options.clone()).unwrap();
        assert_eq!(decode("morse-code", &morse, options).unwrap(), original);
    }
    assert_eq!(
        decode("morse-code", "... --- ... / .-", json!({})).unwrap(),
        "SOS A"
    );
    assert!(matches!(
        decode("morse-code", "......", json!({})),
        Err(ToolError::InvalidInput(_))
    ));
    assert!(matches!(
        encode("morse-code", "😀", json!({})),
        Err(ToolError::InvalidInput(_))
    ));
}

#[test]
fn caesar_default() {
    assert_eq!(
        encode("caesar-cipher", "Hello, Мир! 0123456789", json!({})).unwrap(),
        "Uryyb, Мир! 0123456789"
    );
}

#[test]
fn caesar_options() {
    assert_eq!(
        encode(
            "caesar-cipher",
            "xyz 789",
            json!({"rotation": 3, "rotateNumbers": true})
        )
        .unwrap(),
        "abc 012"
    );
    let mapped = encode("caesar-cipher", "a", json!({"showMapping": true})).unwrap();
    assert!(mapped
        .contains("Original: ABCDEFGHIJKLMNOPQRSTUVWXYZ\nCipher:   NOPQRSTUVWXYZABCDEFGHIJKLM"));
    assert!(mapped.ends_with("OUTPUT\nn"));
    assert!(matches!(
        encode("caesar-cipher", "a", json!({"rotation": "x"})),
        Err(ToolError::InvalidOptions(_))
    ));
}
