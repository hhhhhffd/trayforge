//! `settings.json` in the app config directory. The UI owns the schema; the shell only
//! reads the few keys it needs (`globalHotkey`, `standaloneWindow`, `popoverWidth`,
//! `popoverHeight`, `windowEffect`).

use std::{fs, path::PathBuf};

use serde_json::{Map, Value};
use tauri::{AppHandle, Manager};

pub type Settings = Map<String, Value>;

pub const DEFAULT_HOTKEY: &str = "Ctrl+Alt+D";

fn config_file(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Cannot resolve config directory: {e}"))?;
    Ok(dir.join(name))
}

/// Reads a JSON object file; a missing file is an empty object.
pub fn read_object(app: &AppHandle, name: &str) -> Result<Settings, String> {
    let path = config_file(app, name)?;
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Settings::new()),
        Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
    };
    // Notepad and PowerShell 5 write UTF-8 with a BOM, which serde_json rejects.
    match serde_json::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text)) {
        Ok(Value::Object(map)) => Ok(map),
        Ok(_) => Err(format!("{} is not a JSON object", path.display())),
        Err(e) => Err(format!("Cannot parse {}: {e}", path.display())),
    }
}

/// Writes through a temp file and renames, so a crash never leaves a half-written file.
pub fn write_object(app: &AppHandle, name: &str, object: &Settings) -> Result<(), String> {
    let path = config_file(app, name)?;
    let dir = path.parent().ok_or("Invalid settings path")?;
    fs::create_dir_all(dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    let text = serde_json::to_string_pretty(object).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, text).map_err(|e| format!("Cannot write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Cannot replace {}: {e}", path.display()))
}

/// Deletes a JSON object file; a missing file is fine.
pub fn remove_object(app: &AppHandle, name: &str) -> Result<(), String> {
    let path = config_file(app, name)?;
    match fs::remove_file(&path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("Cannot remove {}: {e}", path.display()))
        }
        _ => Ok(()),
    }
}

pub const SETTINGS_FILE: &str = "settings.json";

/// Settings for internal use: a damaged file must not stop the app from opening.
pub fn load_lenient(app: &AppHandle) -> Settings {
    read_object(app, SETTINGS_FILE).unwrap_or_else(|e| {
        eprintln!("trayforge: {e}");
        Settings::new()
    })
}

pub fn hotkey(settings: &Settings) -> String {
    match settings.get("globalHotkey").and_then(Value::as_str) {
        Some(h) if !h.trim().is_empty() => h.trim().to_string(),
        _ => DEFAULT_HOTKEY.to_string(),
    }
}
