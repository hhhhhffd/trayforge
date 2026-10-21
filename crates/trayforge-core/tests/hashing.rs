//! Reference cases (case id in each test name).
//! Random tools are checked by properties, not exact values.

use std::collections::HashSet;

use serde_json::{json, Map, Value};
use trayforge_core::{run, ToolError, ToolOutput};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn generate(id: &str, input: &str, options: Value) -> Result<ToolOutput, ToolError> {
    run(id, input, &opts(options), false)
}

fn pairs(output: &ToolOutput) -> Vec<(&str, &str)> {
    output
        .items()
        .iter()
        .map(|i| (i.label.as_str(), i.value.as_str()))
        .collect()
}

fn lines(id: &str, options: Value) -> Vec<String> {
    let output = generate(id, "", options).unwrap();
    output.as_text().lines().map(String::from).collect()
}

#[test]
fn hash_abc() {
    let output = generate("hash-generator", "abc", json!({})).unwrap();
    assert_eq!(
        pairs(&output),
        vec![
            ("MD5 (⚠️ legacy)", "900150983cd24fb0d6963f7d28e17f72"),
            ("SHA-1 (⚠️ legacy)", "a9993e364706816aba3e25717850c26c9cd0d89d"),
            (
                "SHA-256",
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            ),
            (
                "SHA-384",
                "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed8086072ba1e7cc2358baeca134c825a7"
            ),
            (
                "SHA-512",
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
            ),
        ]
    );
    let warnings: Vec<bool> = output.items().iter().map(|i| i.is_warning).collect();
    assert_eq!(warnings, [true, true, false, false, false]);
}

#[test]
fn hash_single_algorithm_uppercase() {
    let output = generate(
        "hash-generator",
        "abc",
        json!({"showAll": false, "algorithm": "MD5", "uppercase": true}),
    )
    .unwrap();
    assert_eq!(
        pairs(&output),
        vec![("MD5 (⚠️ legacy)", "900150983CD24FB0D6963F7D28E17F72")]
    );
}

#[test]
fn hash_empty_input_is_rejected() {
    assert_eq!(
        generate("hash-generator", "  ", json!({})).unwrap_err(),
        ToolError::EmptyInput
    );
}

#[test]
fn hmac_sha256() {
    let output = generate(
        "hmac-generator",
        "The quick brown fox jumps over the lazy dog",
        json!({"secretKey": "key"}),
    )
    .unwrap();
    assert_eq!(
        pairs(&output),
        vec![(
            "HMAC-SHA256 (recommended)",
            "f7bc83f430538424b13298e6aa6fb143ef4d59a14946175997479dbc2d1a3cd8"
        )]
    );
}

#[test]
fn hmac_legacy_md5_warns() {
    let output = generate(
        "hmac-generator",
        "The quick brown fox jumps over the lazy dog",
        json!({"secretKey": "key", "algorithm": "MD5", "uppercase": true}),
    )
    .unwrap();
    let items = output.items();
    assert_eq!(items[0].label, "HMAC-MD5 (⚠️ legacy)");
    assert_eq!(items[0].value, "80070713463E7749B90C2DC24911E275");
    assert!(items[0].is_warning);
    assert_eq!(
        items[1].value,
        "This algorithm is considered insecure. Use SHA-256 or higher for production."
    );
}

#[test]
fn hmac_base64_and_show_all() {
    let output = generate(
        "hmac-generator",
        "The quick brown fox jumps over the lazy dog",
        json!({"secretKey": "key", "outputFormat": "Base64", "showAll": true}),
    )
    .unwrap();
    let labels: Vec<&str> = output.items().iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        labels,
        [
            "HMAC-MD5 (⚠️ legacy)",
            "HMAC-SHA1 (⚠️ legacy)",
            "HMAC-SHA256 (recommended)",
            "HMAC-SHA384",
            "HMAC-SHA512",
            "Warning"
        ]
    );
    assert_eq!(output.items()[0].value, "gAcHE0Y+d0m5DC3CSRHidQ==");
}

#[test]
fn hmac_requires_secret_key() {
    let err = generate("hmac-generator", "message", json!({})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Secret key is required for HMAC generation"
    );
}

#[test]
fn password_hash_custom_salt() {
    let output = generate(
        "password-hash",
        "password",
        json!({"iterations": 1, "randomSalt": false, "customSalt": "salt"}),
    )
    .unwrap();
    assert_eq!(
        pairs(&output),
        vec![
            (
                "Full Hash String",
                "$pbkdf2-sha256$1000$c2FsdA$YywoEuRtRgQQK6dhjp1tfS+BKPYma0oDJk0qBGC33LM"
            ),
            (
                "Hash",
                "632c2812e46d4604102ba7618e9d6d7d2f8128f6266b4a03264d2a0460b7dcb3"
            ),
            ("Salt", "73616c74"),
            ("Iterations", "1000"),
            ("Key Length", "32 bytes"),
        ]
    );
}

#[test]
fn password_hash_sha1_key_length() {
    let output = generate(
        "password-hash",
        "password",
        json!({"iterations": 1, "keyLength": 20, "hashFunction": "SHA-1", "randomSalt": false, "customSalt": "salt"}),
    )
    .unwrap();
    assert_eq!(
        pairs(&output)[1],
        ("Hash", "6e88be8bad7eae9d9e10aa061224034fed48d03f")
    );
}

#[test]
fn password_hash_random_salt_differs() {
    let salt = |_| {
        let output = generate("password-hash", "pw", json!({"iterations": 1})).unwrap();
        output.items()[2].value.clone()
    };
    let salts: HashSet<String> = (0..5).map(salt).collect();
    assert_eq!(salts.len(), 5);
    assert!(salts.iter().all(|s| s.len() == 32));
}

#[test]
fn password_hash_needs_custom_salt() {
    let err = generate("password-hash", "pw", json!({"randomSalt": false})).unwrap_err();
    assert!(matches!(err, ToolError::InvalidInput(_)));
}

#[test]
fn checksum_reference() {
    let output = generate("checksum", "123456789", json!({})).unwrap();
    assert_eq!(
        pairs(&output),
        vec![
            ("CRC32", "CBF43926"),
            ("CRC16 (CCITT)", "29B1"),
            ("Adler-32", "091E01DE"),
            ("Input Size", "9 bytes"),
        ]
    );
}

#[test]
fn checksum_single_decimal() {
    let output = generate(
        "checksum",
        "123456789",
        json!({"showAll": false, "algorithm": "Adler-32", "hex": false}),
    )
    .unwrap();
    assert_eq!(
        pairs(&output),
        vec![("Adler-32", "152961502"), ("Input Size", "9 bytes")]
    );
}

#[test]
fn uuid_default_is_v4() {
    let output = generate("uuid-generator", "", json!({})).unwrap();
    let uuid = output.as_text();
    assert_eq!(uuid.len(), 36);
    assert_eq!(&uuid[14..15], "4");
    assert!(uuid.chars().all(|c| c == '-' || c.is_ascii_hexdigit()));
}

#[test]
fn uuid_count_and_uniqueness() {
    let all = lines("uuid-generator", json!({"count": 20}));
    assert_eq!(all.iter().collect::<HashSet<_>>().len(), 20);
}

#[test]
fn uuid_formats() {
    let one = |format: &str| lines("uuid-generator", json!({"format": format})).remove(0);
    let upper = one("Uppercase");
    assert_eq!(upper, upper.to_uppercase());
    assert_eq!(upper.len(), 36);
    assert_eq!(one("No Hyphens").len(), 32);
    let braces = one("Braces");
    assert!(braces.starts_with('{') && braces.ends_with('}'));
    assert!(one("URN").starts_with("urn:uuid:"));
}

#[test]
fn uuid_v3_v5_are_deterministic() {
    let v3 = generate(
        "uuid-generator",
        "python.org",
        json!({"version": "v3 (MD5)"}),
    )
    .unwrap();
    assert_eq!(v3.as_text(), "6fa459ea-ee8a-3ca4-894e-db77e160355e");
    let v5 = generate(
        "uuid-generator",
        "python.org",
        json!({"version": "v5 (SHA-1)"}),
    )
    .unwrap();
    assert_eq!(v5.as_text(), "886313e1-3b8a-5372-9b90-0c9aee199e5d");
}

#[test]
fn uuid_v3_v5_require_name() {
    let err = generate("uuid-generator", "", json!({"version": "v5 (SHA-1)"})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Name is required for UUID v5"
    );
    let err = generate("uuid-generator", "", json!({"version": "v3 (MD5)"})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Name is required for UUID v3"
    );
}

#[test]
fn uuid_time_based_versions() {
    let v1 = lines("uuid-generator", json!({"version": "v1 (Time-based)"})).remove(0);
    assert_eq!(&v1[14..15], "1");
    let v7 = lines("uuid-generator", json!({"version": "v7 (Unix Time)"})).remove(0);
    assert_eq!(&v7[14..15], "7");
    let v2 = lines(
        "uuid-generator",
        json!({"version": "v2 (DCE Security)", "domain": "Group (GID)"}),
    )
    .remove(0);
    assert_eq!(&v2[14..15], "2");
    assert_eq!(&v2[21..23], "01");
}

#[test]
fn ulid_generates_sorted_crockford() {
    let all = lines("ulid-generator", json!({"count": 5}));
    assert_eq!(all.len(), 5);
    for ulid in &all {
        assert_eq!(ulid.len(), 26);
        assert!(ulid
            .chars()
            .all(|c| "0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(c)));
    }
    assert_eq!(all.iter().collect::<HashSet<_>>().len(), 5);
}

#[test]
fn ulid_lowercase() {
    let ulid = lines("ulid-generator", json!({"case": "Lowercase"})).remove(0);
    assert_eq!(ulid, ulid.to_lowercase());
}

#[test]
fn ulid_decode_spec_example() {
    let output = generate("ulid-generator", "01arz3ndektsv4rrffq69g5fav", json!({})).unwrap();
    assert_eq!(
        pairs(&output),
        vec![
            ("Timestamp", "2016-07-30T23:54:10.259Z"),
            ("Unix Timestamp (ms)", "1469922850259"),
            ("Random Component", "TSV4RRFFQ69G5FAV"),
        ]
    );
}

#[test]
fn ulid_decode_rejects_garbage() {
    assert!(matches!(
        generate("ulid-generator", "not-a-ulid", json!({})).unwrap_err(),
        ToolError::InvalidInput(_)
    ));
    assert!(matches!(
        generate("ulid-generator", "81ARZ3NDEKTSV4RRFFQ69G5FAV", json!({})).unwrap_err(),
        ToolError::InvalidInput(_)
    ));
}

#[test]
fn random_string_defaults() {
    let s = lines("random-string", json!({})).remove(0);
    assert_eq!(s.chars().count(), 32);
    let all = lines("random-string", json!({"quantity": 10}));
    assert_eq!(all.len(), 10);
    assert!(all.iter().all(|s| s.chars().count() == 32));
    assert_eq!(all.iter().collect::<HashSet<_>>().len(), 10);
}

#[test]
fn random_string_presets() {
    let hex = lines("random-string", json!({"preset": "Hex", "length": 200})).remove(0);
    assert!(hex
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    let numbers = lines("random-string", json!({"preset": "Numbers", "length": 50})).remove(0);
    assert!(numbers.chars().all(|c| c.is_ascii_digit()));
    let letters = lines("random-string", json!({"preset": "Letters", "length": 50})).remove(0);
    assert!(letters.chars().all(|c| c.is_ascii_alphabetic()));
}

#[test]
fn random_string_custom_classes_and_ambiguous() {
    let s = lines(
        "random-string",
        json!({"preset": "Custom", "length": 500, "lowercase": false, "uppercase": false, "symbols": false, "excludeAmbiguous": true}),
    )
    .remove(0);
    assert!(s
        .chars()
        .all(|c| c.is_ascii_digit() && c != '0' && c != '1'));
}

#[test]
fn random_string_no_charset() {
    let err = generate(
        "random-string",
        "",
        json!({"preset": "Custom", "lowercase": false, "uppercase": false, "digits": false, "symbols": false}),
    )
    .unwrap_err();
    assert_eq!(err.to_string(), "Invalid input: No character set selected");
}

#[test]
fn nanoid_default_with_collision_info() {
    let output = generate("nanoid", "", json!({})).unwrap();
    let items = output.items();
    assert_eq!(items[0].label, "Nanoid");
    assert_eq!(items[0].value.chars().count(), 21);
    assert!(items[0]
        .value
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
    assert_eq!(items[1].label, "Collision probability");
    assert!(items[1]
        .value
        .starts_with("85070591730234615865843651857942052864 possible combinations\n"));
    assert!(items[1].value.ends_with("P IDs for 1% collision"));
}

#[test]
fn nanoid_options() {
    let output = generate(
        "nanoid",
        "",
        json!({"size": 10, "quantity": 5, "showCollision": false, "alphabet": "Hex (16 chars)"}),
    )
    .unwrap();
    let ids: Vec<&str> = output.as_text().lines().collect();
    assert_eq!(ids.len(), 5);
    assert!(ids.iter().all(|id| id.len() == 10
        && id
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))));
    assert_eq!(ids.iter().collect::<HashSet<_>>().len(), 5);
}

#[test]
fn nanoid_custom_alphabet() {
    let output = generate(
        "nanoid",
        "",
        json!({"customAlphabet": "abab", "showCollision": false, "size": 64}),
    )
    .unwrap();
    assert!(output.as_text().chars().all(|c| c == 'a' || c == 'b'));
}

#[test]
fn nanoid_alphabet_too_short() {
    let err = generate("nanoid", "", json!({"customAlphabet": "aaa"})).unwrap_err();
    assert_eq!(
        err.to_string(),
        "Invalid input: Alphabet must have at least 2 characters"
    );
}

#[test]
fn nanoid_huge_space_does_not_overflow() {
    let output = generate("nanoid", "", json!({"size": 256})).unwrap();
    assert_eq!(output.items()[0].value.chars().count(), 256);
}
