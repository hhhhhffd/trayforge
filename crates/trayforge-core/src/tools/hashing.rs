use std::time::{SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::{STANDARD, STANDARD_NO_PAD};
use base64::Engine;
use hmac::{Hmac, Mac};
use md5::Md5;
use rand::seq::IndexedRandom;
use rand::Rng;
use sha1::{Digest, Sha1};
use sha2::{Sha256, Sha384, Sha512};
use uuid::Uuid;

use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

pub fn tools() -> Vec<Tool> {
    vec![
        hash_tool(),
        hmac_tool(),
        password_hash_tool(),
        checksum_tool(),
        uuid_tool(),
        ulid_tool(),
        random_string_tool(),
        nanoid_tool(),
    ]
}

const ALGORITHMS: [&str; 5] = ["MD5", "SHA-1", "SHA-256", "SHA-384", "SHA-512"];

fn option(
    id: &'static str,
    label: &'static str,
    help: &'static str,
    kind: OptionKind,
) -> ToolOption {
    ToolOption {
        id,
        label,
        help: Some(help),
        kind,
    }
}

fn stepper(
    id: &'static str,
    label: &'static str,
    help: &'static str,
    max: i64,
    default: i64,
) -> ToolOption {
    option(
        id,
        label,
        help,
        OptionKind::Stepper {
            min: 1,
            max,
            default,
        },
    )
}

fn is_legacy(algorithm: &str) -> bool {
    algorithm == "MD5" || algorithm == "SHA-1"
}

fn digest(algorithm: &str, data: &[u8]) -> Vec<u8> {
    match algorithm {
        "MD5" => Md5::digest(data).to_vec(),
        "SHA-1" => Sha1::digest(data).to_vec(),
        "SHA-384" => Sha384::digest(data).to_vec(),
        "SHA-512" => Sha512::digest(data).to_vec(),
        _ => Sha256::digest(data).to_vec(),
    }
}

// ---------------------------------------------------------------- Hash Generator

fn hash_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "hash-generator",
            name: "Hash Generator",
            category: Category::Hashing,
            description: "Generate MD5, SHA-1, SHA-256, and other hashes",
            keywords: vec!["hash", "md5", "sha", "sha1", "sha256", "sha512", "digest"],
            icon: "hash",
            input_placeholder: "Enter text to hash...",
            output_placeholder: "Hash values will appear here",
            primary_action: "Generate Hashes",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                option(
                    "uppercase",
                    "Uppercase output",
                    "Display hash in uppercase letters",
                    OptionKind::Toggle { default: false },
                ),
                option(
                    "showAll",
                    "Show all algorithms",
                    "Display all hash algorithms at once",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "algorithm",
                    "Algorithm",
                    "Select specific algorithm (when 'Show all' is off)",
                    OptionKind::Picker {
                        choices: ALGORITHMS.to_vec(),
                        default: 2,
                    },
                ),
            ],
        },
        process: Some(hash_process),
    }
}

fn hash_process(req: &Request) -> ToolResult {
    let uppercase = req.options.bool("uppercase");
    let selected = [req.options.str("algorithm")];
    let algorithms: &[&str] = if req.options.bool("showAll") {
        &ALGORITHMS
    } else {
        &selected
    };
    let items = algorithms
        .iter()
        .map(|&algorithm| {
            let mut hex = hex::encode(digest(algorithm, req.input.as_bytes()));
            if uppercase {
                hex = hex.to_uppercase();
            }
            if is_legacy(algorithm) {
                OutputItem::warning(format!("{algorithm} (⚠️ legacy)"), hex)
            } else {
                OutputItem::new(algorithm, hex)
            }
        })
        .collect();
    Ok(ToolOutput::multiple(items))
}

// ---------------------------------------------------------------- HMAC Generator

fn hmac_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "hmac-generator",
            name: "HMAC Generator",
            category: Category::Hashing,
            description: "Generate HMAC signatures for message authentication",
            keywords: vec!["hmac", "signature", "mac", "authentication", "secret"],
            icon: "key-round",
            input_placeholder: "Enter message to authenticate...",
            output_placeholder: "HMAC signature will appear here",
            primary_action: "Generate HMAC",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                option(
                    "secretKey",
                    "Secret Key",
                    "The secret key for HMAC generation (required)",
                    OptionKind::TextField {
                        placeholder: "Enter secret key...",
                        default: "",
                    },
                ),
                option(
                    "algorithm",
                    "Algorithm",
                    "Hash algorithm for HMAC",
                    OptionKind::Picker {
                        choices: ALGORITHMS.to_vec(),
                        default: 2,
                    },
                ),
                option(
                    "uppercase",
                    "Uppercase output",
                    "Display HMAC in uppercase letters",
                    OptionKind::Toggle { default: false },
                ),
                option(
                    "outputFormat",
                    "Output Format",
                    "Output encoding format",
                    // Hex is the default encoding.
                    OptionKind::Picker {
                        choices: vec!["Hex", "Base64"],
                        default: 0,
                    },
                ),
                option(
                    "showAll",
                    "Show all algorithms",
                    "Generate HMAC with all algorithms at once",
                    OptionKind::Toggle { default: false },
                ),
            ],
        },
        process: Some(hmac_process),
    }
}

fn hmac_bytes(algorithm: &str, key: &[u8], message: &[u8]) -> Vec<u8> {
    macro_rules! mac {
        ($hash:ty) => {{
            let mut mac = Hmac::<$hash>::new_from_slice(key).expect("HMAC accepts any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }};
    }
    match algorithm {
        "MD5" => mac!(Md5),
        "SHA-1" => mac!(Sha1),
        "SHA-384" => mac!(Sha384),
        "SHA-512" => mac!(Sha512),
        _ => mac!(Sha256),
    }
}

fn hmac_process(req: &Request) -> ToolResult {
    let key = req.options.str("secretKey");
    if key.is_empty() {
        return Err(ToolError::InvalidInput(
            "Secret key is required for HMAC generation".into(),
        ));
    }
    let uppercase = req.options.bool("uppercase");
    let base64 = req.options.str("outputFormat") == "Base64";
    let selected = [req.options.str("algorithm")];
    let algorithms: &[&str] = if req.options.bool("showAll") {
        &ALGORITHMS
    } else {
        &selected
    };
    let mut items: Vec<OutputItem> = algorithms
        .iter()
        .map(|&algorithm| {
            let bytes = hmac_bytes(algorithm, key.as_bytes(), req.input.as_bytes());
            let value = if base64 {
                STANDARD.encode(bytes)
            } else if uppercase {
                hex::encode_upper(bytes)
            } else {
                hex::encode(bytes)
            };
            let name = algorithm.replace('-', "");
            if is_legacy(algorithm) {
                OutputItem::warning(format!("HMAC-{name} (⚠️ legacy)"), value)
            } else if algorithm == "SHA-256" {
                OutputItem::new(format!("HMAC-{name} (recommended)"), value)
            } else {
                OutputItem::new(format!("HMAC-{name}"), value)
            }
        })
        .collect();
    if algorithms.iter().any(|a| is_legacy(a)) {
        items.push(OutputItem::warning(
            "Warning",
            "This algorithm is considered insecure. Use SHA-256 or higher for production.",
        ));
    }
    Ok(ToolOutput::multiple(items))
}

// ---------------------------------------------------------------- Password Hash

fn password_hash_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "password-hash",
            name: "Password Hash",
            category: Category::Hashing,
            description: "Generate secure password hashes with PBKDF2",
            keywords: vec!["password", "pbkdf2", "hash", "salt", "kdf"],
            icon: "lock-keyhole",
            input_placeholder: "Enter password to hash...",
            output_placeholder: "Hashed password will appear here",
            primary_action: "Generate Hash",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                stepper(
                    "iterations",
                    "Iterations (thousands)",
                    "Number of iterations in thousands (100 = 100,000)",
                    10_000,
                    100,
                ),
                stepper(
                    "keyLength",
                    "Key Length (bytes)",
                    "Length of derived key in bytes",
                    128,
                    32,
                ),
                option(
                    "hashFunction",
                    "Hash Function",
                    "Underlying hash function for PBKDF2",
                    OptionKind::Picker {
                        choices: vec!["SHA-1", "SHA-256", "SHA-512"],
                        default: 1,
                    },
                ),
                option(
                    "randomSalt",
                    "Generate random salt",
                    "Automatically generate a cryptographically secure salt",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "customSalt",
                    "Custom Salt",
                    "Use a specific salt instead of random",
                    OptionKind::TextField {
                        placeholder: "Enter custom salt...",
                        default: "",
                    },
                ),
            ],
        },
        process: Some(password_hash_process),
    }
}

fn password_hash_process(req: &Request) -> ToolResult {
    let rounds = req.options.i64("iterations") as u32 * 1000;
    let mut derived = vec![0u8; req.options.i64("keyLength") as usize];
    let salt: Vec<u8> = if req.options.bool("randomSalt") {
        let mut salt = vec![0u8; 16];
        rand::rng().fill(salt.as_mut_slice());
        salt
    } else {
        let custom = req.options.str("customSalt");
        if custom.is_empty() {
            return Err(ToolError::InvalidInput(
                "Custom salt is required when random salt is off".into(),
            ));
        }
        custom.as_bytes().to_vec()
    };
    let function = req.options.str("hashFunction");
    let password = req.input.as_bytes();
    match function {
        "SHA-1" => pbkdf2::pbkdf2_hmac::<Sha1>(password, &salt, rounds, &mut derived),
        "SHA-512" => pbkdf2::pbkdf2_hmac::<Sha512>(password, &salt, rounds, &mut derived),
        _ => pbkdf2::pbkdf2_hmac::<Sha256>(password, &salt, rounds, &mut derived),
    }
    // PHC-style string, as used by passlib-like PBKDF2 hashes.
    let full = format!(
        "$pbkdf2-{}${rounds}${}${}",
        function.replace("SHA-", "sha"),
        STANDARD_NO_PAD.encode(&salt),
        STANDARD_NO_PAD.encode(&derived)
    );
    Ok(ToolOutput::multiple(vec![
        OutputItem::new("Full Hash String", full),
        OutputItem::new("Hash", hex::encode(&derived)),
        OutputItem::new("Salt", hex::encode(&salt)),
        OutputItem::new("Iterations", rounds.to_string()),
        OutputItem::new("Key Length", format!("{} bytes", derived.len())),
    ]))
}

// ---------------------------------------------------------------- Checksum

fn checksum_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "checksum",
            name: "Checksum",
            category: Category::Hashing,
            description: "Calculate CRC32, CRC16, and Adler-32 checksums",
            keywords: vec!["checksum", "crc", "crc32", "crc16", "adler"],
            icon: "shield-check",
            input_placeholder: "Enter text to calculate checksum...",
            output_placeholder: "Checksum values will appear here",
            primary_action: "Calculate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                option(
                    "showAll",
                    "Show all checksums",
                    "Display all checksum algorithms",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "algorithm",
                    "Algorithm",
                    "Select specific algorithm",
                    OptionKind::Picker {
                        choices: vec!["CRC32", "CRC16 (CCITT)", "Adler-32"],
                        default: 0,
                    },
                ),
                option(
                    "hex",
                    "Hexadecimal output",
                    "Show checksum in hexadecimal format",
                    OptionKind::Toggle { default: true },
                ),
            ],
        },
        process: Some(checksum_process),
    }
}

fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

fn checksum_process(req: &Request) -> ToolResult {
    let data = req.input.as_bytes();
    let hex = req.options.bool("hex");
    let selected = req.options.str("algorithm");
    let show_all = req.options.bool("showAll");
    let mut items = Vec::new();
    for (name, hex_width) in [("CRC32", 8), ("CRC16 (CCITT)", 4), ("Adler-32", 8)] {
        if !show_all && name != selected {
            continue;
        }
        let value = match name {
            "CRC32" => crc::Crc::<u32>::new(&crc::CRC_32_ISO_HDLC).checksum(data),
            "CRC16 (CCITT)" => {
                u32::from(crc::Crc::<u16>::new(&crc::CRC_16_IBM_3740).checksum(data))
            }
            _ => adler32(data),
        };
        let text = if hex {
            format!("{value:0hex_width$X}")
        } else {
            value.to_string()
        };
        items.push(OutputItem::new(name, text));
    }
    items.push(OutputItem::new(
        "Input Size",
        format!("{} bytes", data.len()),
    ));
    Ok(ToolOutput::multiple(items))
}

// ---------------------------------------------------------------- UUID Generator

fn uuid_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "uuid-generator",
            name: "UUID Generator",
            category: Category::Hashing,
            description: "Generate UUIDs in various formats",
            keywords: vec!["uuid", "guid", "unique", "identifier", "v4"],
            icon: "fingerprint-pattern",
            input_placeholder:
                "Enter namespace/name for UUID v3 or v5, or leave empty for random UUID...",
            output_placeholder: "Generated UUIDs will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                option(
                    "version",
                    "UUID Version",
                    "UUID version to generate",
                    // Versions v1-v5 and v7 are supported.
                    OptionKind::Picker {
                        choices: vec![
                            "v1 (Time-based)",
                            "v2 (DCE Security)",
                            "v3 (MD5)",
                            "v4 (Random)",
                            "v5 (SHA-1)",
                            "v7 (Unix Time)",
                        ],
                        default: 3,
                    },
                ),
                option(
                    "format",
                    "Format",
                    "Output format",
                    OptionKind::Picker {
                        choices: vec!["Standard", "Uppercase", "No Hyphens", "Braces", "URN"],
                        default: 0,
                    },
                ),
                stepper("count", "Count", "Number of UUIDs to generate", 100, 1),
                option(
                    "namespace",
                    "Namespace (v3/v5)",
                    "Namespace for UUID v3 and v5",
                    OptionKind::Picker {
                        choices: vec!["DNS", "URL", "OID", "X.500"],
                        default: 0,
                    },
                ),
                option(
                    "domain",
                    "Local Domain (v2)",
                    "Local domain identifier for UUID v2",
                    OptionKind::Picker {
                        choices: vec!["Person (UID)", "Group (GID)", "Organization"],
                        default: 0,
                    },
                ),
            ],
        },
        process: Some(uuid_process),
    }
}

fn uuid_process(req: &Request) -> ToolResult {
    // Picker labels start with the short version, e.g. "v4".
    let version = &req.options.str("version")[..2];
    let name = req.input.trim();
    let namespace = match req.options.str("namespace") {
        "URL" => Uuid::NAMESPACE_URL,
        "OID" => Uuid::NAMESPACE_OID,
        "X.500" => Uuid::NAMESPACE_X500,
        _ => Uuid::NAMESPACE_DNS,
    };
    let domain = match req.options.str("domain") {
        "Group (GID)" => 1,
        "Organization" => 2,
        _ => 0,
    };
    if name.is_empty() && (version == "v3" || version == "v5") {
        return Err(ToolError::InvalidInput(format!(
            "Name is required for UUID {version}"
        )));
    }
    let mut rng = rand::rng();
    let mut lines = Vec::new();
    for _ in 0..req.options.i64("count") {
        let uuid = match version {
            "v1" | "v2" => {
                let mut node: [u8; 6] = rng.random();
                node[0] |= 1; // random node ids carry the multicast bit (RFC 9562)
                let uuid = Uuid::now_v1(&node);
                if version == "v1" {
                    uuid
                } else {
                    // DCE Security: time_low holds the local id, clock_seq_low the domain.
                    let mut bytes = *uuid.as_bytes();
                    bytes[..4].copy_from_slice(&rng.random::<u32>().to_be_bytes());
                    bytes[6] = (bytes[6] & 0x0f) | 0x20;
                    bytes[9] = domain;
                    Uuid::from_bytes(bytes)
                }
            }
            "v3" => Uuid::new_v3(&namespace, name.as_bytes()),
            "v5" => Uuid::new_v5(&namespace, name.as_bytes()),
            "v7" => Uuid::now_v7(),
            _ => Uuid::new_v4(),
        };
        lines.push(match req.options.str("format") {
            "Uppercase" => uuid.hyphenated().to_string().to_uppercase(),
            "No Hyphens" => uuid.simple().to_string(),
            "Braces" => uuid.braced().to_string(),
            "URN" => uuid.urn().to_string(),
            _ => uuid.hyphenated().to_string(),
        });
    }
    Ok(ToolOutput::text(lines.join("\n")))
}

// ---------------------------------------------------------------- ULID Generator

const CROCKFORD: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

fn ulid_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "ulid-generator",
            name: "ULID Generator",
            category: Category::Hashing,
            description: "Generate time-sortable unique identifiers (ULID)",
            keywords: vec!["ulid", "unique", "identifier", "sortable", "timestamp"],
            icon: "clock",
            input_placeholder: "Leave empty to generate a new ULID...",
            output_placeholder: "Generated ULID will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                stepper("count", "Count", "Number of ULIDs to generate", 100, 1),
                option(
                    "case",
                    "Case",
                    "Output letter case",
                    OptionKind::Segmented {
                        choices: vec!["Uppercase", "Lowercase"],
                        default: 0,
                    },
                ),
            ],
        },
        process: Some(ulid_process),
    }
}

fn ulid_process(req: &Request) -> ToolResult {
    let lowercase = req.options.str("case") == "Lowercase";
    let input = req.input.trim();
    if !input.is_empty() {
        return ulid_decode(input);
    }
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| ToolError::ProcessingFailed(e.to_string()))?
        .as_millis();
    let mut rng = rand::rng();
    let lines: Vec<String> = (0..req.options.i64("count"))
        .map(|_| {
            let value = (now_ms << 80) | (rng.random::<u128>() >> 48);
            let ulid: String = (0..26)
                .map(|i| CROCKFORD[(value >> (125 - 5 * i)) as usize & 31] as char)
                .collect();
            if lowercase {
                ulid.to_lowercase()
            } else {
                ulid
            }
        })
        .collect();
    Ok(ToolOutput::text(lines.join("\n")))
}

/// Shows the parts of an existing ULID ("Random Component" is the random part).
fn ulid_decode(ulid: &str) -> ToolResult {
    let invalid = || {
        ToolError::InvalidInput(
            "Invalid ULID. Expected 26 Crockford Base32 characters (first one 0-7).".into(),
        )
    };
    let upper = ulid.to_uppercase();
    if upper.len() != 26 || !upper.starts_with(|c| ('0'..='7').contains(&c)) {
        return Err(invalid());
    }
    let mut value = 0u128;
    for byte in upper.bytes() {
        let digit = CROCKFORD
            .iter()
            .position(|&c| c == byte)
            .ok_or_else(invalid)?;
        value = (value << 5) | digit as u128;
    }
    let ms = (value >> 80) as u64;
    // Days since the epoch to a civil date (Howard Hinnant's algorithm).
    let days = (ms / 86_400_000) as i64 + 719_468;
    let era = days / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_part + 2) / 5 + 1;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    let secs = ms / 1000 % 86_400;
    let timestamp = format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{:03}Z",
        secs / 3600,
        secs / 60 % 60,
        secs % 60,
        ms % 1000
    );
    Ok(ToolOutput::multiple(vec![
        OutputItem::new("Timestamp", timestamp),
        OutputItem::new("Unix Timestamp (ms)", ms.to_string()),
        OutputItem::new("Random Component", &upper[10..]),
    ]))
}

// ---------------------------------------------------------------- Random String

fn random_string_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "random-string",
            name: "Random String",
            category: Category::Hashing,
            description: "Generate secure random strings, passwords, and API keys",
            keywords: vec![
                "random", "string", "password", "api key", "token", "generate",
            ],
            icon: "shuffle",
            input_placeholder: "No input needed - configure options and generate",
            output_placeholder: "Generated random strings will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                stepper(
                    "length",
                    "Length",
                    "Number of characters to generate",
                    1024,
                    32,
                ),
                stepper(
                    "quantity",
                    "Quantity",
                    "Number of strings to generate",
                    100,
                    1,
                ),
                option(
                    "preset",
                    "Preset",
                    "Character set preset",
                    // Presets other than "Custom" fix the
                    // character set; "Custom" uses the checkboxes below.
                    OptionKind::Picker {
                        choices: vec![
                            "Password",
                            "Alphanumeric",
                            "Letters",
                            "Numbers",
                            "Hex",
                            "Custom",
                        ],
                        default: 0,
                    },
                ),
                option(
                    "lowercase",
                    "Lowercase (a-z)",
                    "Include lowercase letters",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "uppercase",
                    "Uppercase (A-Z)",
                    "Include uppercase letters",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "digits",
                    "Digits (0-9)",
                    "Include numbers",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "symbols",
                    "Symbols (!@#$...)",
                    "Include special characters",
                    OptionKind::Toggle { default: true },
                ),
                option(
                    "excludeAmbiguous",
                    "Exclude ambiguous (0OIl1)",
                    "Remove characters that look similar",
                    OptionKind::Toggle { default: false },
                ),
            ],
        },
        process: Some(random_string_process),
    }
}

fn random_string_process(req: &Request) -> ToolResult {
    const LOWER: &str = "abcdefghijklmnopqrstuvwxyz";
    const UPPER: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    const DIGITS: &str = "0123456789";
    const SYMBOLS: &str = "!@#$%^&*()_+-=[]{}|;:,.<>?";
    let o = &req.options;
    let classes: Vec<&str> = match o.str("preset") {
        "Password" => vec![LOWER, UPPER, DIGITS, SYMBOLS],
        "Alphanumeric" => vec![LOWER, UPPER, DIGITS],
        "Letters" => vec![LOWER, UPPER],
        "Numbers" => vec![DIGITS],
        "Hex" => vec!["0123456789abcdef"],
        _ => [
            ("lowercase", LOWER),
            ("uppercase", UPPER),
            ("digits", DIGITS),
            ("symbols", SYMBOLS),
        ]
        .into_iter()
        .filter(|(id, _)| o.bool(id))
        .map(|(_, chars)| chars)
        .collect(),
    };
    let exclude_ambiguous = o.bool("excludeAmbiguous");
    let charset: Vec<char> = classes
        .concat()
        .chars()
        .filter(|c| !(exclude_ambiguous && "0OIl1".contains(*c)))
        .collect();
    if charset.is_empty() {
        return Err(ToolError::InvalidInput("No character set selected".into()));
    }
    let mut rng = rand::rng();
    let lines: Vec<String> = (0..o.i64("quantity"))
        .map(|_| {
            (0..o.i64("length"))
                .map(|_| *charset.choose(&mut rng).expect("charset is not empty"))
                .collect()
        })
        .collect();
    Ok(ToolOutput::text(lines.join("\n")))
}

// ---------------------------------------------------------------- Nanoid

const NANOID_DEFAULT: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz-";
const NANOID_URL_SAFE: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz-_";

fn nanoid_tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "nanoid",
            name: "Nanoid",
            category: Category::Hashing,
            description: "Generate compact, URL-safe unique identifiers",
            keywords: vec!["nanoid", "id", "unique", "identifier", "url-safe"],
            icon: "dices",
            input_placeholder: "No input needed - configure options and generate",
            output_placeholder: "Generated Nanoid(s) will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: false,
            options: vec![
                stepper("size", "Size", "Length of the ID (default 21)", 256, 21),
                stepper("quantity", "Quantity", "Number of IDs to generate", 100, 1),
                option(
                    "alphabet",
                    "Alphabet",
                    "Character set to use",
                    OptionKind::Picker {
                        choices: vec![
                            "Default (64 chars)",
                            "URL Safe (64 chars)",
                            "Hex (16 chars)",
                            "Lowercase + Digits (36 chars)",
                        ],
                        default: 0,
                    },
                ),
                option(
                    "customAlphabet",
                    "Custom Alphabet",
                    "Your own character set (min 2 chars)",
                    OptionKind::TextField {
                        placeholder: "Enter custom characters...",
                        default: "",
                    },
                ),
                option(
                    "showCollision",
                    "Show collision probability",
                    "Display estimated collision info",
                    OptionKind::Toggle { default: true },
                ),
            ],
        },
        process: Some(nanoid_process),
    }
}

fn nanoid_process(req: &Request) -> ToolResult {
    let custom = req.options.str("customAlphabet");
    let source = if custom.is_empty() {
        match req.options.str("alphabet") {
            "URL Safe (64 chars)" => NANOID_URL_SAFE,
            "Hex (16 chars)" => "0123456789abcdef",
            "Lowercase + Digits (36 chars)" => "0123456789abcdefghijklmnopqrstuvwxyz",
            _ => NANOID_DEFAULT,
        }
    } else {
        custom
    };
    let mut alphabet: Vec<char> = Vec::new();
    for c in source.chars() {
        // Duplicate characters would skew the distribution.
        if !alphabet.contains(&c) {
            alphabet.push(c);
        }
    }
    if alphabet.len() < 2 {
        return Err(ToolError::InvalidInput(
            "Alphabet must have at least 2 characters".into(),
        ));
    }
    let size = req.options.i64("size");
    let mut rng = rand::rng();
    let ids: Vec<String> = (0..req.options.i64("quantity"))
        .map(|_| {
            (0..size)
                .map(|_| *alphabet.choose(&mut rng).expect("alphabet is not empty"))
                .collect()
        })
        .collect();
    let ids = ids.join("\n");
    if !req.options.bool("showCollision") {
        return Ok(ToolOutput::text(ids));
    }
    let combinations = (alphabet.len() as f64).powi(size as i32);
    // Birthday bound: n = sqrt(2 * N * ln(1 / (1 - p))) for p = 1%.
    let ids_for_collision = (-2.0 * combinations * (1.0f64 - 0.01).ln()).sqrt();
    let collision = if combinations.is_finite() {
        let (scaled, suffix) = match ids_for_collision {
            n if n >= 1e15 => (n / 1e15, "P"),
            n if n >= 1e12 => (n / 1e12, "T"),
            n if n >= 1e9 => (n / 1e9, "B"),
            n if n >= 1e6 => (n / 1e6, "M"),
            n => (n, ""),
        };
        let precision = usize::from(!suffix.is_empty());
        format!(
            "{combinations:.0} possible combinations\n{scaled:.precision$}{suffix} IDs for 1% collision"
        )
    } else {
        "More than 10^308 possible combinations; collisions are practically impossible".into()
    };
    Ok(ToolOutput::multiple(vec![
        OutputItem::new("Nanoid", ids),
        OutputItem::new("Collision probability", collision),
    ]))
}
