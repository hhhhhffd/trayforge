use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD};
use base64::Engine;
use serde_json::Value;

use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

pub fn tools() -> Vec<Tool> {
    vec![
        base64_tool(),
        url_encode_tool(),
        html_entity_tool(),
        jwt_decoder_tool(),
        unicode_escape_tool(),
        text_to_binary_tool(),
        morse_code_tool(),
        caesar_cipher_tool(),
    ]
}

fn base64_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "base64",
            name: "Base64",
            category: Category::Encoding,
            description: "Encode and decode Base64 strings",
            keywords: vec!["base64", "encode", "decode", "b64"],
            icon: "binary",
            input_placeholder: "Enter text to encode or Base64 to decode...",
            output_placeholder: "Encoded/decoded result will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: true,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "variant",
                    label: "Variant",
                    help: Some("URL-Safe replaces + with - and / with _"),
                    kind: OptionKind::Segmented {
                        choices: vec!["Standard", "URL-Safe"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "lineWrap",
                    label: "Line Wrap",
                    help: Some("Add line breaks for readability"),
                    // "None" is the default; wrapping is optional.
                    kind: OptionKind::Picker {
                        choices: vec!["None", "64 chars", "76 chars"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "padding",
                    label: "Add Padding",
                    help: Some("Include = padding characters"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(base64_process),
    }
}

fn base64_process(req: &Request) -> ToolResult {
    let url_safe = req.options.str("variant") == "URL-Safe";
    if req.reverse {
        return base64_decode(req.input, url_safe);
    }
    let padding = req.options.bool("padding");
    let engine = match (url_safe, padding) {
        (false, true) => &STANDARD,
        (false, false) => &STANDARD_NO_PAD,
        (true, true) => &URL_SAFE,
        (true, false) => &URL_SAFE_NO_PAD,
    };
    let encoded = engine.encode(req.input.as_bytes());
    let wrapped = match req.options.str("lineWrap") {
        "64 chars" => wrap(&encoded, 64),
        "76 chars" => wrap(&encoded, 76),
        _ => encoded,
    };
    Ok(with_lengths(req.input, wrapped))
}

/// Lenient decoder: trim outer
/// whitespace, drop CR/LF, auto-detect URL-safe alphabet, restore padding.
/// Inner spaces stay and make the input invalid.
fn base64_decode(input: &str, url_safe: bool) -> ToolResult {
    let mut s: String = input
        .trim()
        .chars()
        .filter(|c| *c != '\n' && *c != '\r')
        .collect();
    if url_safe || s.contains('-') || s.contains('_') {
        s = s.replace('-', "+").replace('_', "/");
    }
    let remainder = s.chars().count() % 4;
    if remainder != 0 {
        s.push_str(&"=".repeat(4 - remainder));
    }
    let bytes = STANDARD.decode(&s).map_err(|_| {
        ToolError::InvalidInput(
            "Invalid Base64 string. Make sure the input is properly encoded.".into(),
        )
    })?;
    let text = String::from_utf8(bytes)
        .map_err(|_| ToolError::ProcessingFailed("Could not decode data as UTF-8 text".into()))?;
    Ok(with_lengths(input, text))
}

fn wrap(s: &str, width: usize) -> String {
    // Base64 output is ASCII, so byte chunks are character chunks.
    s.as_bytes()
        .chunks(width)
        .map(|c| std::str::from_utf8(c).expect("base64 output is ASCII"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn with_lengths(input: &str, output: String) -> ToolOutput {
    let mut result = ToolOutput::text(output);
    if let ToolOutput::Text { value, metadata } = &mut result {
        metadata.insert("inputLength".into(), input.chars().count().to_string());
        metadata.insert("outputLength".into(), value.chars().count().to_string());
    }
    result
}

fn invalid(message: impl Into<String>) -> ToolError {
    ToolError::InvalidInput(message.into())
}

// ---------------------------------------------------------------- URL Encode

fn url_encode_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "url-encode",
            name: "URL Encode / Decode",
            category: Category::Encoding,
            description: "Encode and decode URL percent-encoding",
            keywords: vec!["url", "percent", "encode", "decode", "uri", "query"],
            icon: "link",
            input_placeholder: "Enter URL or text to encode/decode...",
            output_placeholder: "Encoded/decoded result will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "encodeMode",
                    label: "Encode Mode",
                    help: Some("What part of the URL to encode"),
                    kind: OptionKind::Picker {
                        choices: vec!["Query Parameter", "Full URL", "Path Component"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "spaceEncoding",
                    label: "Space Encoding",
                    help: Some("How to encode spaces"),
                    kind: OptionKind::Segmented {
                        choices: vec!["%20", "+"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "encodeSlashes",
                    label: "Encode slashes",
                    help: Some("Encode forward slash characters"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(url_process),
    }
}

fn url_process(req: &Request) -> ToolResult {
    let plus_for_space = req.options.str("spaceEncoding") == "+";
    if req.reverse {
        let input = if plus_for_space {
            req.input.replace('+', " ")
        } else {
            req.input.to_string()
        };
        let decoded =
            percent_decode(&input).ok_or_else(|| invalid("Invalid percent-encoded string"))?;
        return Ok(with_lengths(req.input, decoded));
    }
    // Besides unreserved characters, each mode keeps the reserved characters
    // that are meaningful in that part of a URL.
    let mut keep = match req.options.str("encodeMode") {
        "Full URL" => ":/?#[]@!$&'()*+,;=",
        "Path Component" => "!$&'()*+,;=:@/",
        _ => "/",
    }
    .to_string();
    if req.options.bool("encodeSlashes") {
        keep.retain(|c| c != '/');
    }
    let mut out = String::new();
    for byte in req.input.bytes() {
        if byte == b' ' && plus_for_space {
            out.push('+');
        } else if byte.is_ascii_alphanumeric()
            || b"-._~".contains(&byte)
            || keep.as_bytes().contains(&byte)
        {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    Ok(with_lengths(req.input, out))
}

fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = s
                .get(i + 1..i + 3)
                .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

// --------------------------------------------------------------- HTML Entity

fn html_entity_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "html-entity",
            name: "HTML Entity",
            category: Category::Encoding,
            description: "Encode and decode HTML entities",
            keywords: vec!["html", "entity", "entities", "escape", "unescape", "amp"],
            icon: "code",
            input_placeholder: "Enter text with special characters or HTML entities...",
            output_placeholder: "Encoded/decoded result will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "entityFormat",
                    label: "Entity Format",
                    help: Some("Format for encoding entities"),
                    kind: OptionKind::Picker {
                        choices: vec!["Named", "Decimal", "Hexadecimal"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "encodeAll",
                    label: "Encode all characters",
                    help: Some("Encode all non-ASCII characters, not just special ones"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(html_entity_process),
    }
}

/// Names of U+00A0..=U+00FF in order.
const LATIN1_ENTITIES: [&str; 96] = [
    "nbsp", "iexcl", "cent", "pound", "curren", "yen", "brvbar", "sect", "uml", "copy", "ordf",
    "laquo", "not", "shy", "reg", "macr", "deg", "plusmn", "sup2", "sup3", "acute", "micro",
    "para", "middot", "cedil", "sup1", "ordm", "raquo", "frac14", "frac12", "frac34", "iquest",
    "Agrave", "Aacute", "Acirc", "Atilde", "Auml", "Aring", "AElig", "Ccedil", "Egrave", "Eacute",
    "Ecirc", "Euml", "Igrave", "Iacute", "Icirc", "Iuml", "ETH", "Ntilde", "Ograve", "Oacute",
    "Ocirc", "Otilde", "Ouml", "times", "Oslash", "Ugrave", "Uacute", "Ucirc", "Uuml", "Yacute",
    "THORN", "szlig", "agrave", "aacute", "acirc", "atilde", "auml", "aring", "aelig", "ccedil",
    "egrave", "eacute", "ecirc", "euml", "igrave", "iacute", "icirc", "iuml", "eth", "ntilde",
    "ograve", "oacute", "ocirc", "otilde", "ouml", "divide", "oslash", "ugrave", "uacute", "ucirc",
    "uuml", "yacute", "thorn", "yuml",
];

/// Named entities outside Latin-1 and the XML essentials.
const EXTRA_ENTITIES: [(&str, char); 22] = [
    ("ndash", '\u{2013}'),
    ("mdash", '\u{2014}'),
    ("lsquo", '\u{2018}'),
    ("rsquo", '\u{2019}'),
    ("ldquo", '\u{201C}'),
    ("rdquo", '\u{201D}'),
    ("bull", '\u{2022}'),
    ("hellip", '\u{2026}'),
    ("euro", '\u{20AC}'),
    ("trade", '\u{2122}'),
    ("larr", '\u{2190}'),
    ("uarr", '\u{2191}'),
    ("rarr", '\u{2192}'),
    ("darr", '\u{2193}'),
    ("harr", '\u{2194}'),
    ("infin", '\u{221E}'),
    ("ne", '\u{2260}'),
    ("le", '\u{2264}'),
    ("ge", '\u{2265}'),
    ("asymp", '\u{2248}'),
    ("sum", '\u{2211}'),
    ("radic", '\u{221A}'),
];

fn named_entity(c: char) -> Option<&'static str> {
    match c {
        '&' => Some("amp"),
        '<' => Some("lt"),
        '>' => Some("gt"),
        '"' => Some("quot"),
        '\u{A0}'..='\u{FF}' => Some(LATIN1_ENTITIES[c as usize - 0xA0]),
        _ => EXTRA_ENTITIES
            .iter()
            .find(|(_, e)| *e == c)
            .map(|(n, _)| *n),
    }
}

fn entity_char(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => LATIN1_ENTITIES
            .iter()
            .position(|n| *n == name)
            .and_then(|i| char::from_u32(0xA0 + i as u32))
            .or_else(|| {
                EXTRA_ENTITIES
                    .iter()
                    .find(|(n, _)| *n == name)
                    .map(|(_, c)| *c)
            }),
    }
}

fn html_entity_process(req: &Request) -> ToolResult {
    if req.reverse {
        return Ok(with_lengths(req.input, html_decode(req.input)));
    }
    let format = req.options.str("entityFormat");
    let encode_all = req.options.bool("encodeAll");
    let mut out = String::new();
    for c in req.input.chars() {
        let special = matches!(c, '&' | '<' | '>' | '"' | '\'');
        if !special && (c.is_ascii() || !encode_all) {
            out.push(c);
            continue;
        }
        match (format, named_entity(c)) {
            ("Named", Some(name)) => out.push_str(&format!("&{name};")),
            ("Hexadecimal", _) => out.push_str(&format!("&#x{:X};", c as u32)),
            _ => out.push_str(&format!("&#{};", c as u32)),
        }
    }
    Ok(with_lengths(req.input, out))
}

/// Single left-to-right pass, so `&amp;lt;` decodes to `&lt;`, not `<`.
/// Unknown or malformed entities stay as written.
fn html_decode(input: &str) -> String {
    let mut out = String::new();
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let decoded = rest[1..]
            .find(';')
            .filter(|end| *end <= 32)
            .and_then(|end| {
                let body = &rest[1..1 + end];
                let c = if let Some(hex) = body.strip_prefix("#x").or(body.strip_prefix("#X")) {
                    hex.bytes()
                        .all(|b| b.is_ascii_hexdigit())
                        .then(|| u32::from_str_radix(hex, 16).ok().and_then(char::from_u32))
                        .flatten()
                } else if let Some(dec) = body.strip_prefix('#') {
                    dec.bytes()
                        .all(|b| b.is_ascii_digit())
                        .then(|| dec.parse::<u32>().ok().and_then(char::from_u32))
                        .flatten()
                } else {
                    entity_char(body)
                };
                c.map(|c| (c, end + 2))
            });
        match decoded {
            Some((c, len)) => {
                out.push(c);
                rest = &rest[len..];
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

// --------------------------------------------------------------- JWT Decoder

fn jwt_decoder_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "jwt-decoder",
            name: "JWT Decoder",
            category: Category::Encoding,
            description: "Decode and inspect JSON Web Tokens",
            keywords: vec![
                "jwt",
                "token",
                "json web token",
                "bearer",
                "claims",
                "decode",
            ],
            icon: "key-round",
            input_placeholder: "Paste your JWT token here (eyJhbGciOiJIUzI1NiIs...)...",
            output_placeholder: "Decoded token information will appear here",
            primary_action: "Decode",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "prettyPrint",
                    label: "Pretty print JSON",
                    help: Some("Format JSON with indentation"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "showRaw",
                    label: "Show raw Base64",
                    help: Some("Display raw Base64 values"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(jwt_process),
    }
}

fn jwt_process(req: &Request) -> ToolResult {
    let token = req.input.trim();
    let parts: Vec<&str> = token.split('.').collect();
    let [header_b64, payload_b64, signature_b64] = parts[..] else {
        return Err(invalid(
            "Invalid JWT format. Token should have 3 parts separated by dots (header.payload.signature)",
        ));
    };
    let parse = |part: &str, what: &str| -> Result<Value, ToolError> {
        URL_SAFE_NO_PAD
            .decode(part.trim_end_matches('='))
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .ok_or_else(|| invalid(format!("Failed to decode JWT {what}")))
    };
    let header = parse(header_b64, "header")?;
    let payload = parse(payload_b64, "payload")?;
    let json = |v: &Value| {
        if req.options.bool("prettyPrint") {
            serde_json::to_string_pretty(v)
        } else {
            serde_json::to_string(v)
        }
        .expect("JSON value serializes")
    };

    let mut out = String::from("                JWT DECODED\n");
    out.push_str(&"=".repeat(40));
    out.push_str(&format!(
        "\n\n HEADER\n{}\n\n PAYLOAD (Claims)\n{}\n",
        json(&header),
        json(&payload)
    ));

    let time_claim = |key: &str| payload.get(key).and_then(Value::as_f64).map(|t| t as i64);
    let mut claims = String::new();
    for (key, label) in [
        ("iat", "Issued At (iat)"),
        ("nbf", "Not Before (nbf)"),
        ("exp", "Expiration (exp)"),
    ] {
        if let Some(t) = time_claim(key) {
            claims.push_str(&format!("{label}: {}\n", format_utc(t)));
        }
    }
    if let Some(exp) = time_claim("exp") {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_secs() as i64);
        let remaining = exp.saturating_sub(now);
        let text = if remaining <= 0 {
            "Expired".to_string()
        } else {
            format!(
                "{}d {}h {}m {}s",
                remaining / 86400,
                remaining % 86400 / 3600,
                remaining % 3600 / 60,
                remaining % 60
            )
        };
        claims.push_str(&format!("Time until expiration: {text}\n"));
    }
    match payload.get("aud") {
        Some(Value::String(aud)) => claims.push_str(&format!("Audience (aud): {aud}\n")),
        Some(Value::Array(list)) => {
            let names: Vec<&str> = list.iter().filter_map(Value::as_str).collect();
            claims.push_str(&format!("Audience (aud): {}\n", names.join(", ")));
        }
        _ => {}
    }
    if !claims.is_empty() {
        out.push('\n');
        out.push_str(&claims);
    }
    if req.options.bool("showRaw") {
        out.push_str(&format!(
            "\n RAW BASE64\nHeader: {header_b64}\nPayload: {payload_b64}\nSignature: {signature_b64}\n"
        ));
    }
    out.push_str("\n Signature verification requires the secret key");
    Ok(with_lengths(req.input, out))
}

/// `YYYY-MM-DD HH:MM:SS UTC` from Unix seconds (Hinnant's civil-from-days).
fn format_utc(secs: i64) -> String {
    let days = secs.div_euclid(86400) + 719_468;
    let rem = secs.rem_euclid(86400);
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}

// ----------------------------------------------------------- Unicode Escape

fn unicode_escape_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "unicode-escape",
            name: "Unicode Escape",
            category: Category::Encoding,
            description: "Encode and decode Unicode escape sequences",
            keywords: vec!["unicode", "escape", "unescape", "utf", "codepoint", "emoji"],
            icon: "globe",
            input_placeholder: "Enter text or Unicode escapes (\\uXXXX)...",
            output_placeholder: "Encoded/decoded result will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "format",
                    label: "Format",
                    help: Some("Unicode escape format"),
                    kind: OptionKind::Picker {
                        choices: vec!["\\uXXXX", "\\UXXXXXXXX", "\\xXX", "U+XXXX"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "escapeAll",
                    label: "Escape all characters",
                    help: Some("Escape all characters, not just non-ASCII"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "uppercase",
                    label: "Uppercase hex",
                    help: Some("Use uppercase letters in hex values"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(unicode_process),
    }
}

fn unicode_process(req: &Request) -> ToolResult {
    if req.reverse {
        return Ok(with_lengths(req.input, unicode_unescape(req.input)));
    }
    let format = req.options.str("format");
    let escape_all = req.options.bool("escapeAll");
    let upper = req.options.bool("uppercase");
    let hex = |value: u32, width: usize| {
        if upper {
            format!("{value:0width$X}")
        } else {
            format!("{value:0width$x}")
        }
    };
    let mut out = String::new();
    for c in req.input.chars() {
        if c.is_ascii() && !escape_all {
            out.push(c);
            continue;
        }
        let cp = c as u32;
        match format {
            "\\UXXXXXXXX" => out.push_str(&format!("\\U{}", hex(cp, 8))),
            "U+XXXX" => out.push_str(&format!("U+{}", hex(cp, 4))),
            "\\xXX" if cp <= 0xFF => out.push_str(&format!("\\x{}", hex(cp, 2))),
            // \xXX cannot hold larger code points; fall back to UTF-16 units.
            _ => {
                for unit in c.encode_utf16(&mut [0; 2]) {
                    out.push_str(&format!("\\u{}", hex(u32::from(*unit), 4)));
                }
            }
        }
    }
    Ok(with_lengths(req.input, out))
}

/// Decodes `\uXXXX` (with surrogate pairs), `\UXXXXXXXX`, `\xXX` and `U+XXXX`
/// in one pass; anything malformed stays as written.
fn unicode_unescape(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let hex = |at: usize, len: usize| -> Option<u32> {
        let digits = chars.get(at..at + len)?;
        if !digits.iter().all(char::is_ascii_hexdigit) {
            return None;
        }
        u32::from_str_radix(&digits.iter().collect::<String>(), 16).ok()
    };
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let decoded = match (chars[i], chars.get(i + 1)) {
            ('\\', Some('U')) => hex(i + 2, 8).and_then(char::from_u32).map(|c| (c, 10)),
            ('\\', Some('x')) => hex(i + 2, 2).and_then(char::from_u32).map(|c| (c, 4)),
            ('\\', Some('u')) => hex(i + 2, 4).and_then(|hi| {
                if (0xD800..0xDC00).contains(&hi) {
                    let lo = (chars.get(i + 6) == Some(&'\\') && chars.get(i + 7) == Some(&'u'))
                        .then(|| hex(i + 8, 4))
                        .flatten()
                        .filter(|lo| (0xDC00..0xE000).contains(lo))?;
                    char::from_u32(0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)).map(|c| (c, 12))
                } else {
                    char::from_u32(hi).map(|c| (c, 6))
                }
            }),
            ('U', Some('+')) => (4..=6)
                .rev()
                .find_map(|n| hex(i + 2, n).and_then(char::from_u32).map(|c| (c, 2 + n))),
            _ => None,
        };
        match decoded {
            Some((c, len)) => {
                out.push(c);
                i += len;
            }
            None => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }
    out
}

// -------------------------------------------------- Text to Binary/Hex/...

fn text_to_binary_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "text-to-binary",
            name: "Text to Binary/Hex/Octal/Decimal",
            category: Category::Encoding,
            description: "Encode text as binary, hexadecimal, octal, or decimal bytes",
            keywords: vec![
                "binary", "hex", "octal", "decimal", "bytes", "text", "base2", "base16",
            ],
            icon: "binary",
            input_placeholder:
                "Enter text to encode or binary/hex/octal/decimal bytes to decode...",
            output_placeholder: "Converted result will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "outputFormat",
                    label: "Output Format",
                    help: Some("Select output format"),
                    kind: OptionKind::Picker {
                        choices: vec!["Binary", "Hexadecimal", "Octal", "Decimal"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "byteSeparator",
                    label: "Byte Separator",
                    help: Some("Separator between bytes"),
                    kind: OptionKind::Picker {
                        choices: vec!["Space", "Comma", "Newline", "None"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "uppercase",
                    label: "Uppercase (hex)",
                    help: Some("Use uppercase letters for hexadecimal"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "showPrefix",
                    label: "Show prefix (0x, 0b, 0o)",
                    help: Some("Add number format prefix"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(binary_process),
    }
}

fn binary_process(req: &Request) -> ToolResult {
    // (radix, digits per byte, prefix)
    let (radix, width, prefix) = match req.options.str("outputFormat") {
        "Hexadecimal" => (16, 2, "0x"),
        "Octal" => (8, 3, "0o"),
        "Decimal" => (10, 0, ""),
        _ => (2, 8, "0b"),
    };
    if req.reverse {
        let tokens = req
            .input
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|t| !t.is_empty());
        let mut bytes = Vec::new();
        for token in tokens {
            let digits = match token.get(..2) {
                Some(p) if !prefix.is_empty() && p.eq_ignore_ascii_case(prefix) => &token[2..],
                _ => token,
            };
            // Bytes written without a separator are glued together: split by width.
            let digits: Vec<char> = digits.chars().collect();
            let chunk = if width == 0 {
                digits.len().max(1)
            } else {
                width
            };
            for part in digits.chunks(chunk) {
                let part: String = part.iter().collect();
                let byte = u8::from_str_radix(&part, radix)
                    .map_err(|_| invalid(format!("Invalid byte value '{part}'")))?;
                bytes.push(byte);
            }
        }
        let text = String::from_utf8(bytes)
            .map_err(|_| ToolError::ProcessingFailed("Could not decode as UTF-8 text".into()))?;
        return Ok(with_lengths(req.input, text));
    }
    let upper = req.options.bool("uppercase");
    let show_prefix = req.options.bool("showPrefix");
    let separator = match req.options.str("byteSeparator") {
        "Comma" => ",",
        "Newline" => "\n",
        "None" => "",
        _ => " ",
    };
    let items: Vec<String> = req
        .input
        .bytes()
        .map(|b| {
            let digits = match radix {
                2 => format!("{b:08b}"),
                16 if upper => format!("{b:02X}"),
                16 => format!("{b:02x}"),
                8 => format!("{b:03o}"),
                _ => b.to_string(),
            };
            if show_prefix {
                format!("{prefix}{digits}")
            } else {
                digits
            }
        })
        .collect();
    Ok(with_lengths(req.input, items.join(separator)))
}

// ---------------------------------------------------------------- Morse Code

fn morse_code_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "morse-code",
            name: "Morse Code",
            category: Category::Encoding,
            description: "Convert text to and from Morse code",
            keywords: vec!["morse", "code", "telegraph", "dots", "dashes", "sos"],
            icon: "radio",
            input_placeholder: "Enter text or Morse code (use . and -)...",
            output_placeholder: "Morse code or decoded text will appear here",
            primary_action: "Encode",
            secondary_action: Some("Decode"),
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "symbols",
                    label: "Symbols",
                    help: Some("Dot and dash symbols to use"),
                    kind: OptionKind::Picker {
                        choices: vec!["·/−", "./-"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "letterSpacing",
                    label: "Space between letters",
                    help: Some("Add visible spacing between letters"),
                    kind: OptionKind::Toggle { default: true },
                },
            ],
        },
        process: Some(morse_process),
    }
}

const MORSE: [(char, &str); 49] = [
    ('A', ".-"),
    ('B', "-..."),
    ('C', "-.-."),
    ('D', "-.."),
    ('E', "."),
    ('F', "..-."),
    ('G', "--."),
    ('H', "...."),
    ('I', ".."),
    ('J', ".---"),
    ('K', "-.-"),
    ('L', ".-.."),
    ('M', "--"),
    ('N', "-."),
    ('O', "---"),
    ('P', ".--."),
    ('Q', "--.-"),
    ('R', ".-."),
    ('S', "..."),
    ('T', "-"),
    ('U', "..-"),
    ('V', "...-"),
    ('W', ".--"),
    ('X', "-..-"),
    ('Y', "-.--"),
    ('Z', "--.."),
    ('0', "-----"),
    ('1', ".----"),
    ('2', "..---"),
    ('3', "...--"),
    ('4', "....-"),
    ('5', "....."),
    ('6', "-...."),
    ('7', "--..."),
    ('8', "---.."),
    ('9', "----."),
    ('.', ".-.-.-"),
    (',', "--..--"),
    ('?', "..--.."),
    ('\'', ".----."),
    ('!', "-.-.--"),
    ('/', "-..-."),
    ('(', "-.--."),
    (')', "-.--.-"),
    ('&', ".-..."),
    (':', "---..."),
    (';', "-.-.-."),
    ('=', "-...-"),
    ('+', ".-.-."),
];

fn morse_process(req: &Request) -> ToolResult {
    let spaced = req.options.bool("letterSpacing");
    if req.reverse {
        return morse_decode(req.input, spaced);
    }
    let (dot, dash) = if req.options.str("symbols") == "./-" {
        ('.', '-')
    } else {
        ('·', '−')
    };
    let (letter_sep, word_sep) = if spaced {
        ("   ", "       ")
    } else {
        (" ", "   ")
    };
    let mut words = Vec::new();
    for word in req.input.split_whitespace() {
        let mut letters = Vec::new();
        for c in word.chars() {
            let (_, code) = MORSE
                .iter()
                .find(|(l, _)| *l == c.to_ascii_uppercase())
                .ok_or_else(|| invalid(format!("Character '{c}' has no Morse code")))?;
            letters.push(
                code.chars()
                    .map(|s| if s == '.' { dot } else { dash })
                    .collect::<String>(),
            );
        }
        words.push(letters.join(letter_sep));
    }
    Ok(with_lengths(req.input, words.join(word_sep)))
}

/// Accepts `.`/`-` as well as `·`/`−`. A word break is `/` or a run of spaces
/// of at least 5 (letters spaced by 3) or 3 (letters spaced by 1), matching
/// what the encoder emits for the "Space between letters" option.
fn morse_decode(input: &str, spaced: bool) -> ToolResult {
    let word_threshold = if spaced { 5 } else { 3 };
    let mut text = String::new();
    let mut code = String::new();
    let mut run = 0;
    let mut word_break = false;
    let flush = |code: &mut String, text: &mut String, word_break: &mut bool| {
        if code.is_empty() {
            return Ok(());
        }
        let (letter, _) = MORSE
            .iter()
            .find(|(_, m)| *m == code.as_str())
            .ok_or_else(|| invalid(format!("Unknown Morse code sequence '{code}'")))?;
        if *word_break && !text.is_empty() {
            text.push(' ');
        }
        *word_break = false;
        text.push(*letter);
        code.clear();
        Ok::<(), ToolError>(())
    };
    for c in input.trim().chars() {
        match c {
            '.' | '·' | '•' | '-' | '−' | '–' | '_' => {
                if run > 0 {
                    flush(&mut code, &mut text, &mut word_break)?;
                    word_break |= run >= word_threshold;
                    run = 0;
                }
                code.push(if matches!(c, '.' | '·' | '•') {
                    '.'
                } else {
                    '-'
                });
            }
            '/' => {
                flush(&mut code, &mut text, &mut word_break)?;
                word_break = true;
                run = 0;
            }
            c if c.is_whitespace() => run += 1,
            c => return Err(invalid(format!("Invalid Morse code character '{c}'"))),
        }
    }
    flush(&mut code, &mut text, &mut word_break)?;
    Ok(with_lengths(input, text))
}

// ------------------------------------------------------------ Caesar Cipher

fn caesar_cipher_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "caesar-cipher",
            name: "Caesar Cipher",
            category: Category::Encoding,
            description: "Apply a Caesar cipher with any rotation (ROT13 by default)",
            keywords: vec!["caesar", "rot13", "cipher", "rotate", "shift", "encrypt"],
            icon: "refresh-cw",
            input_placeholder: "Enter text to encrypt/decrypt...",
            output_placeholder: "Ciphered text will appear here",
            primary_action: "Apply Cipher",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "rotation",
                    label: "Rotation",
                    help: Some("Number of positions to rotate (13 for ROT13)"),
                    kind: OptionKind::Stepper {
                        min: 1,
                        max: 25,
                        default: 13,
                    },
                },
                ToolOption {
                    id: "rotateNumbers",
                    label: "Rotate numbers",
                    help: Some("Apply the same rotation to digits 0\u{2013}9 (wraps at 10)"),
                    kind: OptionKind::Toggle { default: false },
                },
                ToolOption {
                    id: "showMapping",
                    label: "Show character mapping",
                    help: Some("Display the cipher alphabet"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(caesar_process),
    }
}

fn caesar_process(req: &Request) -> ToolResult {
    let rotation = req.options.i64("rotation");
    if !(1..=25).contains(&rotation) {
        return Err(ToolError::InvalidOptions(format!(
            "Rotation must be between 1 and 25, got {rotation}"
        )));
    }
    let rotation = rotation as u8;
    let rotate_numbers = req.options.bool("rotateNumbers");
    let shift = |c: char| -> char {
        match c {
            'A'..='Z' => ((c as u8 - b'A' + rotation) % 26 + b'A') as char,
            'a'..='z' => ((c as u8 - b'a' + rotation) % 26 + b'a') as char,
            '0'..='9' if rotate_numbers => ((c as u8 - b'0' + rotation % 10) % 10 + b'0') as char,
            c => c,
        }
    };
    let ciphered: String = req.input.chars().map(shift).collect();
    if !req.options.bool("showMapping") {
        return Ok(with_lengths(req.input, ciphered));
    }
    let mapping = |alphabet: &str| {
        format!(
            "Original: {alphabet}\nCipher:   {}\n",
            alphabet.chars().map(shift).collect::<String>()
        )
    };
    let mut out = format!(
        "            CHARACTER MAPPING\n{}{}",
        mapping("ABCDEFGHIJKLMNOPQRSTUVWXYZ"),
        mapping("abcdefghijklmnopqrstuvwxyz")
    );
    if rotate_numbers {
        out.push_str(&mapping("0123456789"));
    }
    out.push_str(&format!("\n                 OUTPUT\n{ciphered}"));
    Ok(with_lengths(req.input, out))
}
