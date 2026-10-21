//! IPC commands exposed to the UI. Shapes are fixed by `docs/ARCHITECTURE.md`.

use std::{
    fs,
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_autostart::ManagerExt;
use trayforge_core::{
    totp::{self, Algorithm, TotpCode, TotpParams},
    ToolError, ToolInfo, ToolOutput,
};

use crate::{
    hotkey, picker,
    settings::{self, Settings},
    window::{self, PickerActive},
};

const MAX_TEXT_FILE_BYTES: u64 = 10 * 1024 * 1024;
const HTTP_TIMEOUT: Duration = Duration::from_secs(60);

#[derive(Serialize)]
pub struct ToolFailure {
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<String>,
    display: String,
}

impl ToolFailure {
    fn internal(message: String) -> Self {
        Self {
            kind: "processingFailed".into(),
            display: format!("Processing failed: {message}"),
            message: Some(message),
        }
    }
}

impl From<ToolError> for ToolFailure {
    fn from(error: ToolError) -> Self {
        // `ToolError` serializes as `{"kind": …, "message"?: …}`.
        let json = serde_json::to_value(&error).unwrap_or_default();
        Self {
            kind: json["kind"]
                .as_str()
                .unwrap_or("processingFailed")
                .to_string(),
            message: json["message"].as_str().map(str::to_string),
            display: error.to_string(),
        }
    }
}

#[tauri::command]
pub fn list_tools() -> Vec<&'static ToolInfo> {
    trayforge_core::registry()
        .iter()
        .map(|tool| &tool.info)
        .collect()
}

#[tauri::command]
pub async fn process_tool(
    tool_id: String,
    input: String,
    options: Option<Map<String, Value>>,
    reverse: Option<bool>,
) -> Result<ToolOutput, ToolFailure> {
    tauri::async_runtime::spawn_blocking(move || {
        trayforge_core::run(
            &tool_id,
            &input,
            &options.unwrap_or_default(),
            reverse.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| ToolFailure::internal(e.to_string()))?
    .map_err(ToolFailure::from)
}

#[tauri::command]
pub fn totp_generate(
    secret: String,
    algorithm: Algorithm,
    digits: u32,
    period: u64,
) -> Result<TotpCode, ToolFailure> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| ToolFailure::internal(e.to_string()))?
        .as_secs();
    Ok(totp::generate(&secret, algorithm, digits, period, now)?)
}

#[tauri::command]
pub fn totp_parse_uri(uri: String) -> Result<TotpParams, ToolFailure> {
    Ok(totp::parse_uri(&uri)?)
}

#[tauri::command]
pub async fn decode_qr_image(image_base64: String) -> Result<String, ToolFailure> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64.trim())
        .map_err(|e| ToolError::InvalidInput(format!("Image is not valid Base64: {e}")))?;
    tauri::async_runtime::spawn_blocking(move || totp::decode_qr_image(&bytes))
        .await
        .map_err(|e| ToolFailure::internal(e.to_string()))?
        .map_err(ToolFailure::from)
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    settings::read_object(&app, settings::SETTINGS_FILE)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let stored = settings::load_lenient(&app);
    let wanted = settings::hotkey(&settings);
    let previous = settings::hotkey(&stored);
    hotkey::apply(&app, &wanted)?;
    if let Err(e) = settings::write_object(&app, settings::SETTINGS_FILE, &settings) {
        if wanted != previous {
            if let Err(restore) = hotkey::apply(&app, &previous) {
                eprintln!("trayforge: cannot restore hotkey: {restore}");
            }
        }
        return Err(e);
    }
    // A new size picked in Appearance replaces the one the popover was dragged to.
    let size_changed = ["popoverWidth", "popoverHeight"]
        .iter()
        .any(|key| settings.get(*key) != stored.get(*key));
    if size_changed {
        settings::remove_object(&app, window::POPOVER_STATE_FILE)?;
    }
    Ok(())
}

#[derive(Deserialize)]
pub struct HttpRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<(String, String)>,
    body: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpResponse {
    status: u16,
    status_text: String,
    headers: Vec<(String, String)>,
    body: String,
    duration_ms: u64,
    size_bytes: usize,
}

#[tauri::command]
pub async fn http_request(request: HttpRequest) -> Result<HttpResponse, String> {
    let method = reqwest::Method::from_bytes(request.method.trim().to_ascii_uppercase().as_bytes())
        .map_err(|_| format!("Invalid HTTP method '{}'", request.method))?;
    let client = reqwest::Client::builder()
        .timeout(HTTP_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let mut builder = client.request(method, &request.url);
    for (name, value) in &request.headers {
        builder = builder.header(name, value);
    }
    if let Some(body) = request.body {
        builder = builder.body(body);
    }

    let started = Instant::now();
    let response = builder.send().await.map_err(|e| describe(&e))?;
    let status = response.status();
    let headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            (
                name.to_string(),
                String::from_utf8_lossy(value.as_bytes()).into_owned(),
            )
        })
        .collect();
    let bytes = response.bytes().await.map_err(|e| describe(&e))?;
    Ok(HttpResponse {
        status: status.as_u16(),
        status_text: status.canonical_reason().unwrap_or("").to_string(),
        headers,
        body: String::from_utf8_lossy(&bytes).into_owned(),
        duration_ms: started.elapsed().as_millis() as u64,
        size_bytes: bytes.len(),
    })
}

/// reqwest's Display hides the root cause ("error sending request"); append the chain.
fn describe(error: &reqwest::Error) -> String {
    let mut text = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        text.push_str(": ");
        text.push_str(&cause.to_string());
        source = cause.source();
    }
    text
}

#[tauri::command]
pub async fn pick_screen_color(app: AppHandle) -> Result<String, String> {
    let active = &app.state::<PickerActive>().0;
    if active.swap(true, Ordering::Relaxed) {
        return Err("Color picker is already active".into());
    }
    let result = tauri::async_runtime::spawn_blocking(picker::pick)
        .await
        .map_err(|e| e.to_string())
        .and_then(|picked| picked);
    active.store(false, Ordering::Relaxed);
    if let Some(window) = app.get_webview_window(window::LABEL) {
        if let Err(e) = window.set_focus() {
            eprintln!("trayforge: cannot refocus window: {e}");
        }
    }
    result
}

#[tauri::command]
pub async fn read_text_file(path: String) -> Result<String, String> {
    let size = fs::metadata(&path)
        .map_err(|e| format!("Cannot read {path}: {e}"))?
        .len();
    if size > MAX_TEXT_FILE_BYTES {
        return Err(format!(
            "File is larger than {} MB",
            MAX_TEXT_FILE_BYTES >> 20
        ));
    }
    let bytes = fs::read(&path).map_err(|e| format!("Cannot read {path}: {e}"))?;
    String::from_utf8(bytes).map_err(|_| format!("{path} is not valid UTF-8 text"))
}

#[tauri::command]
pub async fn write_text_file(path: String, contents: String) -> Result<(), String> {
    fs::write(&path, contents).map_err(|e| format!("Cannot write {path}: {e}"))
}

#[tauri::command]
pub async fn hide_window(app: AppHandle) {
    if let Some(window) = app.get_webview_window(window::LABEL) {
        window::close(&window);
    }
}

#[tauri::command]
pub fn reset_window_state(app: AppHandle) -> Result<(), String> {
    window::reset_popover(&app)
}

#[tauri::command]
pub async fn set_launch_at_login(app: AppHandle, enabled: bool) -> Result<(), String> {
    let launcher = app.autolaunch();
    let result = if enabled {
        launcher.enable()
    } else {
        launcher.disable()
    };
    result.map_err(|e| e.to_string())
}
