//! Global toggle hotkey registration.

use std::sync::Mutex;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};

/// The accelerator string currently registered ("" before the first registration).
#[derive(Default)]
pub struct CurrentHotkey(Mutex<String>);

pub fn parse(accelerator: &str) -> Result<Shortcut, String> {
    accelerator
        .parse()
        .map_err(|e| format!("Invalid hotkey '{accelerator}': {e}"))
}

/// Switches the global hotkey. The new one is registered before the old one is released,
/// so a failure (invalid or taken by another app) leaves the previous hotkey working.
pub fn apply(app: &AppHandle, accelerator: &str) -> Result<(), String> {
    let new = parse(accelerator)?;
    let state = app.state::<CurrentHotkey>();
    let mut current = state.0.lock().map_err(|_| "Hotkey state poisoned")?;
    let old = if current.is_empty() {
        None
    } else {
        Some(parse(&current)?)
    };
    if old == Some(new) {
        *current = accelerator.to_string();
        return Ok(());
    }
    let shortcuts = app.global_shortcut();
    shortcuts
        .register(new)
        .map_err(|e| format!("Cannot register hotkey '{accelerator}': {e}"))?;
    if let Some(old) = old {
        if let Err(e) = shortcuts.unregister(old) {
            eprintln!("trayforge: cannot unregister previous hotkey: {e}");
        }
    }
    *current = accelerator.to_string();
    Ok(())
}
