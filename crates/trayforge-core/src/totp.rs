//! RFC 6238 TOTP, `otpauth://` URI parsing and QR image decoding for the
//! TOTP Generator panel.

use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

use crate::ToolError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Algorithm {
    #[serde(rename = "SHA1")]
    Sha1,
    #[serde(rename = "SHA256")]
    Sha256,
    #[serde(rename = "SHA512")]
    Sha512,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpParams {
    pub account: String,
    pub issuer: String,
    pub secret: String,
    pub algorithm: Algorithm,
    pub digits: u32,
    pub period: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TotpCode {
    pub code: String,
    pub remaining_seconds: u64,
}

fn invalid(message: &str) -> ToolError {
    ToolError::InvalidInput(message.into())
}

/// Case-insensitive RFC 4648 Base32; spaces, hyphens and `=` padding are ignored.
fn decode_base32(secret: &str) -> Result<Vec<u8>, ToolError> {
    let mut bytes = Vec::new();
    let (mut buffer, mut bits) = (0u32, 0u32);
    for c in secret
        .chars()
        .filter(|c| !c.is_whitespace() && !matches!(c, '-' | '='))
    {
        let value = match c.to_ascii_uppercase() {
            c @ 'A'..='Z' => c as u32 - 'A' as u32,
            c @ '2'..='7' => c as u32 - '2' as u32 + 26,
            _ => return Err(invalid("Secret is not valid Base32")),
        };
        buffer = (buffer << 5) | value;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            bytes.push((buffer >> bits) as u8);
            buffer &= (1 << bits) - 1;
        }
    }
    if bytes.is_empty() {
        return Err(invalid("Secret is empty"));
    }
    Ok(bytes)
}

fn hmac(algorithm: Algorithm, key: &[u8], message: &[u8]) -> Vec<u8> {
    // HMAC accepts keys of any length, so `new_from_slice` cannot fail.
    match algorithm {
        Algorithm::Sha1 => {
            let mut mac = Hmac::<Sha1>::new_from_slice(key).expect("any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha256 => {
            let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
        Algorithm::Sha512 => {
            let mut mac = Hmac::<Sha512>::new_from_slice(key).expect("any key length");
            mac.update(message);
            mac.finalize().into_bytes().to_vec()
        }
    }
}

pub fn generate(
    secret_base32: &str,
    algorithm: Algorithm,
    digits: u32,
    period: u64,
    unix_time: u64,
) -> Result<TotpCode, ToolError> {
    if digits != 6 && digits != 8 {
        return Err(invalid("Digits must be 6 or 8"));
    }
    if period == 0 {
        return Err(invalid("Period must be greater than zero"));
    }
    let key = decode_base32(secret_base32)?;
    let digest = hmac(algorithm, &key, &(unix_time / period).to_be_bytes());
    let offset = (digest[digest.len() - 1] & 0x0f) as usize;
    let truncated = u32::from_be_bytes([
        digest[offset] & 0x7f,
        digest[offset + 1],
        digest[offset + 2],
        digest[offset + 3],
    ]);
    let code = truncated % 10u32.pow(digits);
    Ok(TotpCode {
        code: format!("{code:0width$}", width = digits as usize),
        remaining_seconds: period - unix_time % period,
    })
}

fn percent_decode(s: &str, plus_is_space: bool) -> Result<String, ToolError> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = s
                    .get(i + 1..i + 3)
                    .filter(|h| h.bytes().all(|b| b.is_ascii_hexdigit()))
                    .and_then(|h| u8::from_str_radix(h, 16).ok())
                    .ok_or_else(|| invalid("Invalid percent-encoding in URI"))?;
                out.push(hex);
                i += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| invalid("URI is not valid UTF-8"))
}

pub fn parse_uri(uri: &str) -> Result<TotpParams, ToolError> {
    let rest = uri
        .trim()
        .strip_prefix("otpauth://")
        .ok_or_else(|| invalid("Not an otpauth:// URI"))?;
    let (kind, rest) = rest
        .split_once('/')
        .ok_or_else(|| invalid("Malformed otpauth:// URI"))?;
    if !kind.eq_ignore_ascii_case("totp") {
        return Err(invalid("Only TOTP codes are supported"));
    }
    let (label, query) = rest.split_once('?').unwrap_or((rest, ""));
    let label = percent_decode(label, false)?;
    let (label_issuer, account) = match label.split_once(':') {
        Some((issuer, account)) => (issuer.trim(), account.trim()),
        None => ("", label.trim()),
    };

    let mut params = TotpParams {
        account: account.to_string(),
        issuer: label_issuer.to_string(),
        secret: String::new(),
        algorithm: Algorithm::Sha1,
        digits: 6,
        period: 30,
    };
    for pair in query.split('&').filter(|p| !p.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        let value = percent_decode(value, true)?;
        match key.to_ascii_lowercase().as_str() {
            "secret" => params.secret = value,
            "issuer" => params.issuer = value,
            "algorithm" => {
                params.algorithm = match value.to_ascii_uppercase().as_str() {
                    "SHA1" => Algorithm::Sha1,
                    "SHA256" => Algorithm::Sha256,
                    "SHA512" => Algorithm::Sha512,
                    _ => return Err(invalid("Unsupported algorithm")),
                }
            }
            "digits" => {
                params.digits = value
                    .parse()
                    .map_err(|_| invalid("Digits must be a number"))?
            }
            "period" => {
                params.period = value
                    .parse()
                    .map_err(|_| invalid("Period must be a number"))?
            }
            _ => {}
        }
    }
    if params.secret.is_empty() {
        return Err(invalid("URI does not contain a secret"));
    }
    Ok(params)
}

pub fn decode_qr_image(bytes: &[u8]) -> Result<String, ToolError> {
    let image = image::load_from_memory(bytes)
        .map_err(|_| invalid("Could not read the image (PNG or JPEG expected)"))?
        .to_luma8();
    let width = image.width() as usize;
    let mut prepared =
        rqrr::PreparedImage::prepare_from_greyscale(width, image.height() as usize, |x, y| {
            image.get_pixel(x as u32, y as u32).0[0]
        });
    prepared
        .detect_grids()
        .into_iter()
        .find_map(|grid| grid.decode().ok().map(|(_, content)| content))
        .ok_or_else(|| invalid("No QR code found in the image"))
}
