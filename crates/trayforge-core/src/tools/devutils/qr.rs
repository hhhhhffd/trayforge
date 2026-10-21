use std::io::Cursor;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use image::{ImageBuffer, ImageFormat, Luma};
use qrcode::{Color, EcLevel, QrCode};

use crate::{
    Category, OptionKind, OutputItem, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput,
    ToolResult,
};

/// Modules of white border around the symbol, as required by the QR spec.
const QUIET_ZONE: usize = 4;

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "qr-code",
            name: "QR Code",
            category: Category::DevUtils,
            description: "Generate QR codes from text, URLs, or data",
            keywords: vec!["qr", "code", "barcode", "scan", "generate", "image"],
            icon: "qr-code",
            input_placeholder: "Enter text, URL, or data to encode...",
            output_placeholder: "QR code will be generated",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "errorCorrection",
                    label: "Error correction",
                    help: Some("Higher = more damage resistance"),
                    kind: OptionKind::Picker {
                        choices: vec!["Low (7%)", "Medium (15%)", "Quartile (25%)", "High (30%)"],
                        default: 1,
                    },
                },
                ToolOption {
                    id: "size",
                    label: "Size (px)",
                    help: Some("Output image size"),
                    kind: OptionKind::Stepper {
                        min: 100,
                        max: 2000,
                        default: 300,
                    },
                },
                ToolOption {
                    id: "output",
                    label: "Output",
                    help: Some("Output format"),
                    kind: OptionKind::Picker {
                        choices: vec!["Preview", "Base64 PNG", "Data URL"],
                        default: 0,
                    },
                },
            ],
        },
        process: Some(process),
    }
}

fn process(req: &Request) -> ToolResult {
    let level = match req.options.str("errorCorrection") {
        "Low (7%)" => EcLevel::L,
        "Quartile (25%)" => EcLevel::Q,
        "High (30%)" => EcLevel::H,
        _ => EcLevel::M,
    };
    let code = QrCode::with_error_correction_level(req.input.as_bytes(), level)
        .map_err(|_| ToolError::ProcessingFailed("Failed to generate QR code".into()))?;
    let png = render_png(&code, req.options.i64("size") as usize)?;
    let base64 = STANDARD.encode(png);
    Ok(match req.options.str("output") {
        "Base64 PNG" => ToolOutput::text(base64),
        "Data URL" => ToolOutput::text(format!("data:image/png;base64,{base64}")),
        _ => ToolOutput::multiple(vec![OutputItem::image(
            "QR Code",
            format!("data:image/png;base64,{base64}"),
        )]),
    })
}

/// Crisp integer-scaled modules, centered on a white `size` x `size` canvas
/// (larger than `size` only when `size` cannot hold one pixel per module).
fn render_png(code: &QrCode, size: usize) -> Result<Vec<u8>, ToolError> {
    let modules = code.width();
    let total = modules + 2 * QUIET_ZONE;
    let scale = (size / total).max(1);
    let side = size.max(total * scale);
    let margin = (side - total * scale) / 2;
    let colors = code.to_colors();
    let image = ImageBuffer::from_fn(side as u32, side as u32, |x, y| {
        let (x, y) = (x as usize, y as usize);
        let dark = (|| {
            let mx = (x.checked_sub(margin)? / scale).checked_sub(QUIET_ZONE)?;
            let my = (y.checked_sub(margin)? / scale).checked_sub(QUIET_ZONE)?;
            (mx < modules && my < modules).then(|| colors[my * modules + mx] == Color::Dark)
        })();
        Luma([if dark == Some(true) { 0u8 } else { 255 }])
    });
    let mut png = Cursor::new(Vec::new());
    image
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|_| ToolError::ProcessingFailed("Failed to encode QR code image".into()))?;
    Ok(png.into_inner())
}
