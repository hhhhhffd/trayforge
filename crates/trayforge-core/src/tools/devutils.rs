mod ascii;
mod branch;
mod cert;
mod cron;
mod http_status;
mod ip;
mod mime;
mod port;
mod qr;
mod semver;
mod time;

use crate::{Category, Tool, ToolInfo};

pub fn tools() -> Vec<Tool> {
    vec![
        qr::tool(),
        cron::tool(),
        branch::tool(),
        semver::tool(),
        ascii::tool(),
        http_status::tool(),
        mime::tool(),
        port::tool(),
        ip::tool(),
        cert::tool(),
        panel(
            "http-client",
            "HTTP Client",
            "Build and send HTTP requests with full header and body control",
            vec![
                "http", "request", "rest", "post", "curl", "header", "json", "client",
            ],
            "send",
        ),
        panel(
            "totp-generator",
            "TOTP Generator",
            "Generate live TOTP codes from a secret or QR code, for testing 2FA integrations",
            vec![
                "totp",
                "authenticator",
                "hotp",
                "otpauth",
                "security",
                "rfc6238",
                "2fa",
            ],
            "key-round",
        ),
    ]
}

/// Dedicated-panel tools (see `graphics.rs`); the shell drives them through
/// `http_request` and the `totp` module.
fn panel(
    id: &'static str,
    name: &'static str,
    description: &'static str,
    keywords: Vec<&'static str>,
    icon: &'static str,
) -> Tool {
    Tool {
        info: ToolInfo {
            id,
            name,
            category: Category::DevUtils,
            description,
            keywords,
            icon,
            input_placeholder: "",
            output_placeholder: "",
            primary_action: "",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: true,
            custom_ui: true,
            options: Vec::new(),
        },
        process: None,
    }
}
