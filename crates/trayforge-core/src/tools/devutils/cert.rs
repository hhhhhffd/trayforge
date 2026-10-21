use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use x509_parser::certificate::X509Certificate;
use x509_parser::extensions::{GeneralName, ParsedExtension};
use x509_parser::prelude::FromDer;
use x509_parser::public_key::PublicKey;

use super::time::{format_datetime, now_unix};
use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

const PEM_BEGIN: &str = "-----BEGIN CERTIFICATE-----";
const PEM_END: &str = "-----END CERTIFICATE-----";

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "certificate-decoder",
            name: "Certificate Decoder",
            category: Category::DevUtils,
            description: "Parse and display X.509 certificate information",
            keywords: vec!["certificate", "x509", "public key", "cert", "pem", "ssl", "tls"],
            icon: "shield-check",
            input_placeholder: "Paste PEM certificate here:\n-----BEGIN CERTIFICATE-----\n...base64 encoded certificate...\n-----END CERTIFICATE-----",
            output_placeholder: "Certificate details will appear here",
            primary_action: "Decode",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "format",
                    label: "Input Format",
                    help: Some("Format of the certificate input"),
                    kind: OptionKind::Picker {
                        choices: vec!["Auto-detect", "PEM", "Base64 (DER)"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "showPublicKey",
                    label: "Show public key details",
                    help: Some("Display public key information"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "showExtensions",
                    label: "Show extensions",
                    help: Some("Display certificate extensions"),
                    kind: OptionKind::Toggle { default: true },
                },
                ToolOption {
                    id: "showRaw",
                    label: "Show raw data",
                    help: Some("Show raw hexadecimal data"),
                    kind: OptionKind::Toggle { default: false },
                },
            ],
        },
        process: Some(process),
    }
}

fn hex(bytes: &[u8], separator: &str) -> String {
    bytes
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect::<Vec<_>>()
        .join(separator)
}

fn oid_name(oid: &str) -> String {
    match oid {
        "1.2.840.113549.1.1.1" => "RSA Encryption",
        "1.2.840.113549.1.1.5" => "SHA-1 with RSA Encryption",
        "1.2.840.113549.1.1.10" => "RSASSA-PSS",
        "1.2.840.113549.1.1.11" => "SHA-256 with RSA Encryption",
        "1.2.840.113549.1.1.12" => "SHA-384 with RSA Encryption",
        "1.2.840.113549.1.1.13" => "SHA-512 with RSA Encryption",
        "1.2.840.10045.2.1" => "Elliptic Curve (EC)",
        "1.2.840.10045.3.1.7" => "P-256 (prime256v1)",
        "1.3.132.0.34" => "P-384 (secp384r1)",
        "1.3.132.0.35" => "P-521 (secp521r1)",
        "1.2.840.10045.4.3.2" => "ECDSA with SHA-256",
        "1.2.840.10045.4.3.3" => "ECDSA with SHA-384",
        "1.2.840.10045.4.3.4" => "ECDSA with SHA-512",
        "1.3.101.112" => "Ed25519",
        "1.3.101.113" => "Ed448",
        other => return other.to_string(),
    }
    .to_string()
}

fn der_from_input(input: &str, format: &str) -> Result<Vec<u8>, ToolError> {
    let unparsable = || {
        ToolError::InvalidInput(
            "Could not parse certificate. Ensure it's in PEM or DER format.".into(),
        )
    };
    let pem = match format {
        "PEM" => true,
        "Base64 (DER)" => false,
        _ => input.contains("-----BEGIN"),
    };
    let body = if pem {
        let start = input.find(PEM_BEGIN).ok_or_else(unparsable)? + PEM_BEGIN.len();
        let end = input[start..].find(PEM_END).ok_or_else(unparsable)? + start;
        &input[start..end]
    } else {
        input
    };
    let compact: String = body.chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(compact).map_err(|_| unparsable())
}

fn process(req: &Request) -> ToolResult {
    let der = der_from_input(req.input, req.options.str("format"))?;
    let (_, cert) = X509Certificate::from_der(&der).map_err(|_| {
        ToolError::InvalidInput("Invalid certificate data. Could not create certificate.".into())
    })?;

    let subject = cert.subject();
    let summary = subject
        .iter_common_name()
        .chain(subject.iter_organization())
        .find_map(|a| a.as_str().ok())
        .map_or_else(|| subject.to_string(), str::to_string);
    let validity = cert.validity();
    let (not_before, not_after) = (
        validity.not_before.timestamp(),
        validity.not_after.timestamp(),
    );
    let days_left = (not_after - now_unix()).div_euclid(86_400);
    let expiry = if days_left >= 0 {
        format!(" ({days_left} days remaining)")
    } else {
        format!(" (expired {} days ago)", -days_left)
    };

    let mut out = vec![
        format!("Subject (Summary): {summary}"),
        format!("Subject: {subject}"),
        format!("Issuer: {}", cert.issuer()),
        format!(
            "Serial Number: {}",
            cert.raw_serial_as_string().to_uppercase()
        ),
        format!("Version: {}", cert.version().0 + 1),
        format!("Not Before: {} UTC", format_datetime(not_before)),
        format!("Not After: {} UTC{expiry}", format_datetime(not_after)),
        format!(
            "Signature Algorithm: {}",
            oid_name(&cert.signature_algorithm.algorithm.to_id_string())
        ),
        String::new(),
        "--- Fingerprints ---".into(),
        format!("SHA-1: {}", hex(&Sha1::digest(&der), ":")),
        format!("SHA-256: {}", hex(&Sha256::digest(&der), ":")),
    ];

    if req.options.bool("showPublicKey") {
        let spki = cert.public_key();
        out.push(String::new());
        out.push("--- Public Key ---".into());
        out.push(format!(
            "Algorithm: {}",
            oid_name(&spki.algorithm.algorithm.to_id_string())
        ));
        match spki.parsed() {
            Ok(PublicKey::RSA(rsa)) => out.push(format!("Key Size: {} bits", rsa.key_size())),
            Ok(PublicKey::EC(point)) => {
                out.push(format!("Key Size: {} bits", point.key_size()));
                if let Some(curve) = spki
                    .algorithm
                    .parameters
                    .as_ref()
                    .and_then(|p| p.as_oid().ok())
                {
                    out.push(format!("Curve: {}", oid_name(&curve.to_id_string())));
                }
            }
            _ => {}
        }
        out.push(format!(
            "Key Fingerprint (SHA-256): {}",
            hex(&Sha256::digest(spki.raw), ":")
        ));
    }

    if req.options.bool("showExtensions") {
        out.push(String::new());
        out.push("--- Extensions ---".into());
        for ext in cert.extensions() {
            match ext.parsed_extension() {
                ParsedExtension::BasicConstraints(bc) => out.push(format!(
                    "Basic Constraints: CA: {}{}",
                    if bc.ca { "Yes" } else { "No" },
                    bc.path_len_constraint
                        .map_or(String::new(), |n| format!(", Path Length: {n}"))
                )),
                ParsedExtension::KeyUsage(ku) => out.push(format!("Key Usage: {ku}")),
                ParsedExtension::ExtendedKeyUsage(eku) => {
                    let mut usages: Vec<String> = [
                        (eku.any, "Any"),
                        (eku.server_auth, "Server Authentication"),
                        (eku.client_auth, "Client Authentication"),
                        (eku.code_signing, "Code Signing"),
                        (eku.email_protection, "Email Protection"),
                        (eku.time_stamping, "Time Stamping"),
                        (eku.ocsp_signing, "OCSP Signing"),
                    ]
                    .iter()
                    .filter(|(set, _)| *set)
                    .map(|(_, name)| name.to_string())
                    .collect();
                    usages.extend(eku.other.iter().map(|o| o.to_id_string()));
                    out.push(format!("Extended Key Usage: {}", usages.join(", ")));
                }
                ParsedExtension::SubjectAlternativeName(san) => {
                    let names: Vec<String> = san
                        .general_names
                        .iter()
                        .map(|n| match n {
                            GeneralName::DNSName(d) => format!("DNS:{d}"),
                            GeneralName::RFC822Name(e) => format!("Email:{e}"),
                            GeneralName::URI(u) => format!("URI:{u}"),
                            GeneralName::IPAddress(ip) => format!("IP:{}", ip_text(ip)),
                            other => format!("{other}"),
                        })
                        .collect();
                    out.push(format!("Subject Alt Names: {}", names.join(", ")));
                }
                _ => {}
            }
        }
    }

    if req.options.bool("showRaw") {
        out.push(String::new());
        out.push("--- Raw Data ---".into());
        out.push("Hex (first 256 bytes):".into());
        out.push(hex(&der[..der.len().min(256)], " "));
    }
    Ok(ToolOutput::text(out.join("\n")))
}

fn ip_text(bytes: &[u8]) -> String {
    match *bytes {
        [a, b, c, d] => std::net::Ipv4Addr::new(a, b, c, d).to_string(),
        _ => <[u8; 16]>::try_from(bytes).map_or_else(
            |_| hex(bytes, ":"),
            |octets| std::net::Ipv6Addr::from(octets).to_string(),
        ),
    }
}
