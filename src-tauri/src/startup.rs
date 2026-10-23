//! Launch at login. The MSIX (Microsoft Store) build cannot use the registry Run key: writes
//! to HKCU are redirected into the package's private hive, and the exe path under
//! WindowsApps is not launchable from there. It declares a `windows.startupTask` in its
//! manifest instead, toggled through the WinRT StartupTask API.

use tauri::AppHandle;
use tauri_plugin_autostart::ManagerExt;
use windows::{
    core::h,
    ApplicationModel::{StartupTask, StartupTaskState},
    Win32::{
        Foundation::APPMODEL_ERROR_NO_PACKAGE, Storage::Packaging::Appx::GetCurrentPackageFullName,
    },
};

fn is_packaged() -> bool {
    let mut length = 0u32;
    // SAFETY: a zero length with a null buffer only queries whether a package identity exists.
    let status = unsafe { GetCurrentPackageFullName(&mut length, None) };
    status != APPMODEL_ERROR_NO_PACKAGE
}

pub fn set_enabled(app: &AppHandle, enabled: bool) -> Result<(), String> {
    if !is_packaged() {
        let launcher = app.autolaunch();
        let result = if enabled {
            launcher.enable()
        } else {
            launcher.disable()
        };
        return result.map_err(|e| e.to_string());
    }

    // Must match `TaskId` of the startup task in `packaging/msix/AppxManifest.xml`.
    let task = StartupTask::GetAsync(h!("TrayforgeStartup"))
        .and_then(|operation| operation.get())
        .map_err(|e| format!("Cannot find the startup task: {e}"))?;
    if !enabled {
        return task.Disable().map_err(|e| e.to_string());
    }
    let state = task
        .RequestEnableAsync()
        .and_then(|operation| operation.get())
        .map_err(|e| e.to_string())?;
    match state {
        StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy => Ok(()),
        StartupTaskState::DisabledByUser => {
            Err("turned off in Windows Settings > Apps > Startup".to_string())
        }
        StartupTaskState::DisabledByPolicy => Err("blocked by a group policy".to_string()),
        _ => Err("Windows did not enable the startup task".to_string()),
    }
}
