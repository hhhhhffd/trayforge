use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use trayforge_core::totp::{self, Algorithm};
use trayforge_core::{run, ToolError, ToolOutput};

fn opts(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

fn text(id: &str, input: &str, options: Value) -> Result<String, ToolError> {
    run(id, input, &opts(options), false).map(|o| o.as_text().to_string())
}

fn items(id: &str, input: &str, options: Value) -> Result<Vec<(String, String)>, ToolError> {
    run(id, input, &opts(options), false).map(|o| {
        o.items()
            .iter()
            .map(|i| (i.label.clone(), i.value.clone()))
            .collect()
    })
}

fn item(items: &[(String, String)], label: &str) -> String {
    items
        .iter()
        .find(|(l, _)| l == label)
        .unwrap_or_else(|| panic!("no item '{label}' in {items:?}"))
        .1
        .clone()
}

fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let (mut out, mut buffer, mut bits) = (String::new(), 0u32, 0);
    for b in bytes {
        buffer = (buffer << 8) | u32::from(*b);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(ALPHABET[(buffer >> bits & 31) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(ALPHABET[(buffer << (5 - bits) & 31) as usize] as char);
    }
    out
}

// RFC 6238 appendix B.
const SHA1_KEY: &[u8] = b"12345678901234567890";
const SHA256_KEY: &[u8] = b"12345678901234567890123456789012";
const SHA512_KEY: &[u8] = b"1234567890123456789012345678901234567890123456789012345678901234";

type Vector<'a> = (Algorithm, &'a [u8], [(u64, &'a str); 6]);

#[test]
fn totp_rfc6238_vectors() {
    let cases: [Vector; 3] = [
        (
            Algorithm::Sha1,
            SHA1_KEY,
            [
                (59, "94287082"),
                (1111111109, "07081804"),
                (1111111111, "14050471"),
                (1234567890, "89005924"),
                (2000000000, "69279037"),
                (20000000000, "65353130"),
            ],
        ),
        (
            Algorithm::Sha256,
            SHA256_KEY,
            [
                (59, "46119246"),
                (1111111109, "68084774"),
                (1111111111, "67062674"),
                (1234567890, "91819424"),
                (2000000000, "90698825"),
                (20000000000, "77737706"),
            ],
        ),
        (
            Algorithm::Sha512,
            SHA512_KEY,
            [
                (59, "90693936"),
                (1111111109, "25091201"),
                (1111111111, "99943326"),
                (1234567890, "93441116"),
                (2000000000, "38618901"),
                (20000000000, "47863826"),
            ],
        ),
    ];
    for (algorithm, key, vectors) in cases {
        let secret = base32(key);
        for (time, expected) in vectors {
            let got = totp::generate(&secret, algorithm, 8, 30, time).unwrap();
            assert_eq!(got.code, expected, "{algorithm:?} at {time}");
        }
    }
}

#[test]
fn totp_six_digits_and_remaining_seconds() {
    let secret = base32(SHA1_KEY);
    let got = totp::generate(&secret, Algorithm::Sha1, 6, 30, 59).unwrap();
    assert_eq!(got.code, "287082");
    assert_eq!(got.remaining_seconds, 1);
    assert_eq!(
        totp::generate(&secret, Algorithm::Sha1, 6, 30, 60)
            .unwrap()
            .remaining_seconds,
        30
    );
}

#[test]
fn totp_secret_formats() {
    let canonical = totp::generate(
        "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ",
        Algorithm::Sha1,
        8,
        30,
        59,
    )
    .unwrap();
    for secret in [
        "gezdgnbvgy3tqojqgezdgnbvgy3tqojq",
        "GEZD GNBV GY3T QOJQ GEZD GNBV GY3T QOJQ",
        "GEZD-GNBV-GY3T-QOJQ-GEZD-GNBV-GY3T-QOJQ====",
    ] {
        assert_eq!(
            totp::generate(secret, Algorithm::Sha1, 8, 30, 59).unwrap(),
            canonical
        );
    }
}

#[test]
fn totp_rejects_bad_parameters() {
    let secret = base32(SHA1_KEY);
    assert!(totp::generate(&secret, Algorithm::Sha1, 7, 30, 59).is_err());
    assert!(totp::generate(&secret, Algorithm::Sha1, 6, 0, 59).is_err());
    assert!(totp::generate("not base32!", Algorithm::Sha1, 6, 30, 59).is_err());
    assert!(totp::generate("", Algorithm::Sha1, 6, 30, 59).is_err());
}

#[test]
fn totp_serde_names() {
    assert_eq!(
        serde_json::to_string(&Algorithm::Sha256).unwrap(),
        "\"SHA256\""
    );
    assert_eq!(
        serde_json::from_str::<Algorithm>("\"SHA512\"").unwrap(),
        Algorithm::Sha512
    );
    let code = totp::TotpCode {
        code: "123456".into(),
        remaining_seconds: 7,
    };
    assert_eq!(
        serde_json::to_value(code).unwrap(),
        json!({"code": "123456", "remainingSeconds": 7})
    );
}

#[test]
fn totp_parse_uri_full() {
    let params = totp::parse_uri(
        "otpauth://totp/ACME%20Co:john.doe%40example.com?secret=JBSWY3DPEHPK3PXP&issuer=ACME%20Co&algorithm=SHA256&digits=8&period=60",
    )
    .unwrap();
    assert_eq!(params.account, "john.doe@example.com");
    assert_eq!(params.issuer, "ACME Co");
    assert_eq!(params.secret, "JBSWY3DPEHPK3PXP");
    assert_eq!(params.algorithm, Algorithm::Sha256);
    assert_eq!((params.digits, params.period), (8, 60));
    assert_eq!(
        serde_json::to_value(&params).unwrap()["algorithm"],
        "SHA256"
    );
}

#[test]
fn totp_parse_uri_defaults_and_label_issuer() {
    let params = totp::parse_uri("otpauth://totp/GitHub:octocat?secret=ABC234").unwrap();
    assert_eq!(params.issuer, "GitHub");
    assert_eq!(params.account, "octocat");
    assert_eq!(params.algorithm, Algorithm::Sha1);
    assert_eq!((params.digits, params.period), (6, 30));
}

#[test]
fn totp_parse_uri_errors() {
    for bad in [
        "https://example.com",
        "otpauth://hotp/x?secret=ABC234",
        "otpauth://totp/x",
        "otpauth://totp/x?secret=ABC234&algorithm=MD5",
        "otpauth://totp/x?secret=ABC234&digits=abc",
        "otpauth://totp/x%ZZ?secret=ABC234",
    ] {
        assert!(totp::parse_uri(bad).is_err(), "{bad}");
    }
}

fn qr_png(input: &str, options: Value) -> Vec<u8> {
    let encoded = text("qr-code", input, options).unwrap();
    STANDARD.decode(encoded).unwrap()
}

#[test]
fn qr_roundtrip_through_decoder() {
    let uri = "otpauth://totp/Trayforge:me@example.com?secret=JBSWY3DPEHPK3PXP&issuer=Trayforge";
    for level in ["Low (7%)", "Medium (15%)", "Quartile (25%)", "High (30%)"] {
        let png = qr_png(
            uri,
            json!({"output": "Base64 PNG", "errorCorrection": level}),
        );
        assert_eq!(totp::decode_qr_image(&png).unwrap(), uri, "{level}");
    }
    let png = qr_png(uri, json!({"output": "Base64 PNG", "size": 600}));
    let params = totp::parse_uri(&totp::decode_qr_image(&png).unwrap()).unwrap();
    assert_eq!(params.account, "me@example.com");
}

#[test]
fn qr_outputs() {
    let preview = run("qr-code", "hello", &Map::new(), false).unwrap();
    assert!(preview.items()[0].is_image);
    assert!(preview.items()[0]
        .value
        .starts_with("data:image/png;base64,"));
    let url = text("qr-code", "hello", json!({"output": "Data URL"})).unwrap();
    assert!(url.starts_with("data:image/png;base64,"));

    let png = qr_png("hello", json!({"output": "Base64 PNG", "size": 300}));
    let image = image::load_from_memory(&png).unwrap();
    assert_eq!((image.width(), image.height()), (300, 300));
}

#[test]
fn qr_too_much_data_and_bad_image() {
    let huge = "x".repeat(5000);
    assert_eq!(
        text("qr-code", &huge, json!({})),
        Err(ToolError::ProcessingFailed(
            "Failed to generate QR code".into()
        ))
    );
    assert!(totp::decode_qr_image(b"not an image").is_err());
}

#[test]
fn cron_describes_and_lists() {
    let out = text("cron-parser", "0 9 * * MON-FRI", json!({"count": 3})).unwrap();
    assert!(out.contains("Description: At 09:00 on Monday through Friday"));
    assert_eq!(out.matches("09:00:00").count(), 3);
    let six = text(
        "cron-parser",
        "*/30 * * * * *",
        json!({"seconds": true, "count": 2}),
    )
    .unwrap();
    assert!(six.contains("Every 30 seconds"));
    assert!(matches!(
        text("cron-parser", "* * * *", json!({})),
        Err(ToolError::InvalidInput(m)) if m.contains("expected 5 fields")
    ));
}

#[test]
fn git_branch_name_options() {
    let out = items("git-branch-name", "PROJ-123: Add user login!", json!({})).unwrap();
    assert_eq!(item(&out, "Branch name"), "feature/proj-123-add-user-login");
    assert_eq!(
        item(&out, "Create command"),
        "git checkout -b feature/proj-123-add-user-login"
    );
    assert!(item(&out, "Alternatives").contains("bugfix/proj-123-add-user-login"));

    let out = items(
        "git-branch-name",
        "Fix Crash",
        json!({"branchType": "None", "separator": "_", "lowercase": false, "username": "JSmith"}),
    )
    .unwrap();
    assert_eq!(item(&out, "Branch name"), "JSmith/Fix_Crash");

    let out = items(
        "git-branch-name",
        "a very long description of the change",
        json!({"maxLength": 20}),
    )
    .unwrap();
    assert_eq!(item(&out, "Branch name"), "feature/a-very-long");

    let dated = items("git-branch-name", "x", json!({"includeDate": true})).unwrap();
    let name = item(&dated, "Branch name");
    assert!(
        name.starts_with("feature/20") && name.ends_with("-x"),
        "{name}"
    );

    assert!(text("git-branch-name", "!!!", json!({})).is_err());
}

#[test]
fn semver_parse_compare_bump() {
    let info = text("semver-comparator", "v1.2.3-beta.1+build.5", json!({})).unwrap();
    assert!(info.contains("VERSION ANALYSIS"));
    assert!(info.contains("Pre-release: beta.1"));
    assert!(info.contains("Build:       build.5"));
    assert!(info.contains("Stability:   Pre-release"));

    let compare = |input: &str, include: bool| {
        text(
            "semver-comparator",
            input,
            json!({"action": "Compare", "includePrerelease": include}),
        )
        .unwrap()
    };
    assert!(compare("1.0.0\n2.0.0", true).contains("1.0.0 is older than 2.0.0"));
    assert!(compare("1.0.0-alpha, 1.0.0", true).contains("1.0.0-alpha is older than 1.0.0"));
    assert!(compare("1.0.0-alpha, 1.0.0", false).contains("1.0.0-alpha equals 1.0.0"));
    let sorted = compare("1.0.0-rc.1, 1.0.0-beta.11, 1.0.0-beta.2, 1.0.0", true);
    let beta2 = sorted.find("1.0.0-beta.2").unwrap();
    let beta11 = sorted.find("1.0.0-beta.11").unwrap();
    let rc = sorted.find("1.0.0-rc.1").unwrap();
    assert!(beta2 < beta11 && beta11 < rc);
    assert!(sorted.contains("SORTED (ASCENDING)") && sorted.contains("RANGE"));

    let bump = |action: &str| {
        text(
            "semver-comparator",
            "1.2.3-rc.1",
            json!({ "action": action }),
        )
        .unwrap()
    };
    assert!(bump("Bump Major").contains("→ 2.0.0"));
    assert!(bump("Bump Minor").contains("→ 1.3.0"));
    assert!(bump("Bump Patch").contains("→ 1.2.4"));
}

#[test]
fn semver_errors() {
    let err = text("semver-comparator", "1.2", json!({})).unwrap_err();
    assert!(err.to_string().contains("Invalid version: 1.2"));
    assert!(text("semver-comparator", "01.2.3", json!({})).is_err());
    assert!(text("semver-comparator", "1.2.3", json!({"action": "Compare"})).is_err());
    assert!(text("semver-comparator", "help", json!({}))
        .unwrap()
        .contains("SEMANTIC VERSIONING HELP"));
}

#[test]
fn ascii_art_fonts() {
    let standard = text("ascii-art", "Hi", json!({})).unwrap();
    assert!(standard.lines().count() >= 5);
    for font in ["Banner", "Block", "Slant", "Small", "Mini"] {
        let out = text("ascii-art", "Hello 42!", json!({ "font": font })).unwrap();
        assert!(!out.trim().is_empty(), "{font}");
    }
    let banner = text("ascii-art", "I", json!({"font": "Banner"})).unwrap();
    assert_eq!(banner.lines().next().unwrap(), "#####");
    assert_eq!(banner.lines().count(), 7);
}

#[test]
fn ascii_art_width_and_alignment() {
    let wide = text(
        "ascii-art",
        "AAA BBB CCC",
        json!({"font": "Banner", "width": 40}),
    )
    .unwrap();
    assert!(wide.lines().all(|l| l.chars().count() <= 40), "{wide}");
    assert!(wide.lines().count() > 7);
    let right = text(
        "ascii-art",
        "I",
        json!({"font": "Banner", "alignment": "Right", "width": 20}),
    )
    .unwrap();
    // Alignment is relative to the widest line of the art itself.
    assert_eq!(right.lines().next().unwrap(), "#####");
}

#[test]
fn http_status_lookup() {
    let out = text("http-status", "404", json!({})).unwrap();
    assert!(out.starts_with("404 Not Found"));
    assert!(out.contains("Client Error"));
    assert!(out.contains("4xx codes indicate client error"));
    assert!(text("http-status", "teapot", json!({}))
        .unwrap()
        .starts_with("418 I'm a teapot"));
    assert!(text("http-status", "499", json!({}))
        .unwrap()
        .starts_with("Unknown HTTP status code: 499"));
    let err = text("http-status", "zzzzz", json!({})).unwrap_err();
    assert!(err
        .to_string()
        .contains("No status codes found matching 'zzzzz'"));
}

#[test]
fn http_status_lists() {
    let first = text("http-status", "", json!({})).unwrap();
    assert!(first.contains("Showing first 10 of "));
    let class = text("http-status", "5xx", json!({"showAll": false})).unwrap();
    assert!(class.contains("500  Internal Server Error"));
    let all = text("http-status", "", json!({"showAll": true})).unwrap();
    assert!(all.contains("100  Continue") && all.contains("511  Network Authentication Required"));
    assert!(!all.contains("Showing first"));
}

#[test]
fn mime_lookup_both_directions() {
    let out = items("mime-type", ".json", json!({})).unwrap();
    assert_eq!(item(&out, "MIME type"), "application/json");
    assert_eq!(
        item(&out, "Content-Type header"),
        "Content-Type: application/json"
    );
    let out = items("mime-type", "image/jpeg", json!({})).unwrap();
    assert_eq!(item(&out, "Related extensions"), ".jpg, .jpeg");
    let out = items("mime-type", "HTM", json!({})).unwrap();
    assert_eq!(item(&out, "MIME type"), "text/html");
}

#[test]
fn mime_search_and_filter() {
    let list = text("mime-type", "text", json!({})).unwrap();
    assert!(list.contains("text/plain") && list.contains("Showing first 15 of"));
    let filtered = text("mime-type", "x", json!({"category": "Video"})).unwrap();
    assert!(filtered.contains("video/") && !filtered.contains("text/"));
    assert!(text("mime-type", ".json", json!({"category": "Image"})).is_err());
    let err = text("mime-type", ".nope", json!({})).unwrap_err();
    assert!(err.to_string().contains("No MIME type found for '.nope'"));
}

#[test]
fn port_lookup() {
    let out = items("port-lookup", "443", json!({})).unwrap();
    assert_eq!(item(&out, "Service"), "HTTPS");
    assert_eq!(
        item(&out, "Range"),
        "Well-known port (0-1023) - requires root/admin"
    );
    let out = items("port-lookup", "50000", json!({})).unwrap();
    assert_eq!(item(&out, "Service"), "Unknown/Unregistered");
    assert_eq!(item(&out, "Range"), "Dynamic/Ephemeral port (49152-65535)");
    let out = items("port-lookup", "5432", json!({})).unwrap();
    assert_eq!(item(&out, "Range"), "Registered port (1024-49151)");

    let by_name = text("port-lookup", "postgres", json!({})).unwrap();
    assert!(by_name.contains("5432"));
    let db = text("port-lookup", "", json!({"category": "Database"})).unwrap();
    assert!(db.contains("MySQL") && !db.contains("HTTPS"));
    let common = text("port-lookup", "", json!({"showCommon": true})).unwrap();
    assert!(common.contains("HTTPS") && !common.contains("Showing first"));
    let err = text("port-lookup", "nonexistent", json!({})).unwrap_err();
    assert!(err.to_string().contains("No ports found for 'nonexistent'"));
}

#[test]
fn ip_analyze_and_subnet() {
    let out = items("ip-address", "192.168.1.10/24", json!({})).unwrap();
    assert_eq!(item(&out, "Version"), "IPv4");
    assert_eq!(item(&out, "Binary"), "11000000.10101000.00000001.00001010");
    assert_eq!(item(&out, "Decimal"), "3232235786");
    assert_eq!(item(&out, "Type"), "Private");
    assert_eq!(item(&out, "Subnet Mask"), "255.255.255.0");
    assert_eq!(item(&out, "Broadcast Address"), "192.168.1.255");

    let out = items(
        "ip-address",
        "8.8.8.8",
        json!({"showBinary": false, "showDecimal": false}),
    )
    .unwrap();
    assert!(out.iter().all(|(l, _)| l != "Binary" && l != "Decimal"));
    assert_eq!(item(&out, "Type"), "Public");

    let out = items(
        "ip-address",
        "10.0.0.0/30",
        json!({"operation": "Subnet Calculator"}),
    )
    .unwrap();
    assert_eq!(item(&out, "Usable Hosts"), "2");
    assert_eq!(item(&out, "First Usable Host"), "10.0.0.1");
    assert_eq!(item(&out, "Last Usable Host"), "10.0.0.2");
    assert_eq!(item(&out, "Wildcard Mask"), "0.0.0.3");
    let out = items(
        "ip-address",
        "10.0.0.1/32",
        json!({"operation": "Subnet Calculator"}),
    )
    .unwrap();
    assert_eq!(item(&out, "Usable Hosts"), "1");
}

#[test]
fn ip_v6_and_conversions() {
    let out = items("ip-address", "::1", json!({})).unwrap();
    assert_eq!(item(&out, "Version"), "IPv6");
    assert_eq!(
        item(&out, "Expanded"),
        "0000:0000:0000:0000:0000:0000:0000:0001"
    );
    assert_eq!(item(&out, "Type"), "Loopback (::1)");
    assert_eq!(
        item(&items("ip-address", "fe80::1", json!({})).unwrap(), "Type"),
        "Link-Local"
    );

    assert_eq!(
        text(
            "ip-address",
            "1.2.3.4",
            json!({"operation": "Convert to Binary"})
        )
        .unwrap(),
        "00000001.00000010.00000011.00000100"
    );
    assert_eq!(
        text(
            "ip-address",
            "1.2.3.4",
            json!({"operation": "Convert to Decimal"})
        )
        .unwrap(),
        "16909060"
    );
    assert_eq!(
        text(
            "ip-address",
            "16909060",
            json!({"operation": "Convert to Decimal"})
        )
        .unwrap(),
        "1.2.3.4"
    );
    assert_eq!(
        text(
            "ip-address",
            "2001:db8::1",
            json!({"operation": "Validate Only"})
        )
        .unwrap(),
        "Valid IPv6 Address"
    );
}

#[test]
fn ip_errors() {
    assert_eq!(
        text("ip-address", "999.1.1.1", json!({})),
        Err(ToolError::InvalidInput("Invalid IP address format".into()))
    );
    assert!(text(
        "ip-address",
        "::1",
        json!({"operation": "Convert to Binary"})
    )
    .is_err());
    assert!(text(
        "ip-address",
        "10.0.0.1",
        json!({"operation": "Subnet Calculator"})
    )
    .is_err());
    assert!(text("ip-address", "10.0.0.1/33", json!({})).is_err());
}

// Generated offline with OpenSSL: EC P-256, valid until 2126.
const TEST_CERT: &str = "-----BEGIN CERTIFICATE-----
MIICLzCCAdagAwIBAgIUITTQGQ2gGTDFg8vj0ogs8v8/M8YwCgYIKoZIzj0EAwIw
RzELMAkGA1UEBhMCVVMxFzAVBgNVBAoMDlRyYXlmb3JnZSBUZXN0MR8wHQYDVQQD
DBZ0ZXN0LnRyYXlmb3JnZS5leGFtcGxlMCAXDTI2MTAyMTA3MzE1OFoYDzIxMjYw
OTI3MDczMTU4WjBHMQswCQYDVQQGEwJVUzEXMBUGA1UECgwOVHJheWZvcmdlIFRl
c3QxHzAdBgNVBAMMFnRlc3QudHJheWZvcmdlLmV4YW1wbGUwWTATBgcqhkjOPQIB
BggqhkjOPQMBBwNCAAQx0rmRzDO7OfHMavc2DBUilyxm1uVqsFv2+0lAoBA+xtRZ
5dIk2I0d8xlgqRxcic6Gkq2lHO+SLRiZOd5FeNifo4GdMIGaMB0GA1UdDgQWBBQI
qLRVdOGFhtEwEnMsiHFJqJ5j+zA8BgNVHREENTAzghZ0ZXN0LnRyYXlmb3JnZS5l
eGFtcGxlghMqLnRyYXlmb3JnZS5leGFtcGxlhwTAAAIBMAwGA1UdEwEB/wQCMAAw
DgYDVR0PAQH/BAQDAgeAMB0GA1UdJQQWMBQGCCsGAQUFBwMBBggrBgEFBQcDAjAK
BggqhkjOPQQDAgNHADBEAiAoNOw2Q4GwHmgOjaxzO7mOLtBqavlh++H3iDq3i1vD
1wIgPw9rOkq5tFmRqWRSbODSs6uz9WnZIoba1j/xKpR6wCk=
-----END CERTIFICATE-----";

#[test]
fn certificate_decoder_pem() {
    let out = text("certificate-decoder", TEST_CERT, json!({})).unwrap();
    assert!(out.contains("Subject (Summary): test.trayforge.example"));
    assert!(
        out.contains("Serial Number: 21:34:D0:19:0D:A0:19:30:C5:83:CB:E3:D2:88:2C:F2:FF:3F:33:C6")
    );
    assert!(out.contains("Not Before: 2026-10-21 07:31:58 UTC"));
    assert!(
        out.contains("Not After: 2126-09-27 07:31:58 UTC (") && out.contains("days remaining)")
    );
    assert!(out.contains("Signature Algorithm: ECDSA with SHA-256"));
    assert!(out.contains(
        "SHA-256: DB:53:2C:46:02:70:17:44:A9:EB:49:AE:E1:FF:94:DF:EF:14:D8:A2:1C:66:8E:39:57:0A:C3:26:64:4B:E2:BF"
    ));
    assert!(out.contains("--- Public Key ---") && out.contains("Key Size: 256 bits"));
    assert!(out.contains("Curve: P-256 (prime256v1)"));
    assert!(out.contains("Basic Constraints: CA: No"));
    assert!(out.contains("Extended Key Usage: Server Authentication, Client Authentication"));
    assert!(out.contains(
        "Subject Alt Names: DNS:test.trayforge.example, DNS:*.trayforge.example, IP:192.0.2.1"
    ));
    assert!(!out.contains("--- Raw Data ---"));
}

#[test]
fn certificate_decoder_options_and_formats() {
    let hidden = text(
        "certificate-decoder",
        TEST_CERT,
        json!({"showPublicKey": false, "showExtensions": false, "showRaw": true}),
    )
    .unwrap();
    assert!(!hidden.contains("--- Public Key ---") && !hidden.contains("--- Extensions ---"));
    assert!(hidden.contains("--- Raw Data ---") && hidden.contains("30 82 02 2F"));

    let body: String = TEST_CERT
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    let der = text(
        "certificate-decoder",
        &body,
        json!({"format": "Base64 (DER)"}),
    )
    .unwrap();
    assert!(der.contains("Subject (Summary): test.trayforge.example"));
    assert!(text("certificate-decoder", &body, json!({"format": "PEM"})).is_err());
}

#[test]
fn certificate_decoder_errors() {
    assert_eq!(
        text("certificate-decoder", "garbage", json!({})),
        Err(ToolError::InvalidInput(
            "Could not parse certificate. Ensure it's in PEM or DER format.".into()
        ))
    );
    let not_a_cert = format!(
        "-----BEGIN CERTIFICATE-----\n{}\n-----END CERTIFICATE-----",
        STANDARD.encode("hello")
    );
    assert_eq!(
        text("certificate-decoder", &not_a_cert, json!({})),
        Err(ToolError::InvalidInput(
            "Invalid certificate data. Could not create certificate.".into()
        ))
    );
}

#[test]
fn dev_utils_registry_order_and_panels() {
    let ids: Vec<&str> = trayforge_core::registry()
        .iter()
        .filter(|t| t.info.category == trayforge_core::Category::DevUtils)
        .map(|t| t.info.id)
        .collect();
    assert_eq!(
        ids,
        [
            "qr-code",
            "cron-parser",
            "git-branch-name",
            "semver-comparator",
            "ascii-art",
            "http-status",
            "mime-type",
            "port-lookup",
            "ip-address",
            "certificate-decoder",
            "http-client",
            "totp-generator"
        ]
    );
    for id in ["http-client", "totp-generator"] {
        let tool = trayforge_core::find(id).unwrap();
        assert!(tool.info.custom_ui && tool.process.is_none());
        assert!(matches!(
            run(id, "", &Map::new(), false),
            Err(ToolError::UnsupportedOperation(_))
        ));
    }
    let output: ToolOutput = run("qr-code", "x", &Map::new(), false).unwrap();
    assert!(matches!(output, ToolOutput::Multiple { .. }));
}
