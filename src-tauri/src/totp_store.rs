//! Saved TOTP accounts of the TOTP Generator panel. The file holds the JSON list encrypted
//! with DPAPI, so only the same Windows user on the same machine can read the secrets.

use std::{fs, path::PathBuf};

use tauri::{AppHandle, Manager};
use trayforge_core::totp::TotpParams;
use windows::Win32::{
    Foundation::{LocalFree, HLOCAL},
    Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
    },
};

const FILE: &str = "totp-accounts.bin";

fn path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("Cannot resolve config directory: {e}"))?;
    Ok(dir.join(FILE))
}

/// A missing file is an empty list.
pub fn load(app: &AppHandle) -> Result<Vec<TotpParams>, String> {
    let path = path(app)?;
    let sealed = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Cannot read {}: {e}", path.display())),
    };
    let json = unprotect(&sealed).map_err(|e| format!("Cannot decrypt {}: {e}", path.display()))?;
    serde_json::from_slice(&json).map_err(|e| format!("Cannot parse {}: {e}", path.display()))
}

/// Writes through a temp file and renames, so a crash never leaves a half-written file.
pub fn save(app: &AppHandle, accounts: &[TotpParams]) -> Result<(), String> {
    let path = path(app)?;
    let dir = path.parent().ok_or("Invalid accounts path")?;
    fs::create_dir_all(dir).map_err(|e| format!("Cannot create {}: {e}", dir.display()))?;
    let json = serde_json::to_vec(accounts).map_err(|e| e.to_string())?;
    let sealed = protect(&json).map_err(|e| format!("Cannot encrypt accounts: {e}"))?;
    let tmp = path.with_extension("bin.tmp");
    fs::write(&tmp, sealed).map_err(|e| format!("Cannot write {}: {e}", tmp.display()))?;
    fs::rename(&tmp, &path).map_err(|e| format!("Cannot replace {}: {e}", path.display()))
}

fn protect(data: &[u8]) -> windows::core::Result<Vec<u8>> {
    let input = blob(data);
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptProtectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )?;
        Ok(take(output))
    }
}

fn unprotect(data: &[u8]) -> windows::core::Result<Vec<u8>> {
    let input = blob(data);
    let mut output = CRYPT_INTEGER_BLOB::default();
    unsafe {
        CryptUnprotectData(
            &input,
            None,
            None,
            None,
            None,
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )?;
        Ok(take(output))
    }
}

fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
    CRYPT_INTEGER_BLOB {
        cbData: data.len() as u32,
        // DPAPI only reads the input blob.
        pbData: data.as_ptr() as *mut u8,
    }
}

/// Copies a DPAPI output blob and frees the buffer Windows allocated for it.
unsafe fn take(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    let bytes = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
    LocalFree(Some(HLOCAL(blob.pbData.cast())));
    bytes
}
