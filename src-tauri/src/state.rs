//! Shared application state and helpers used by every Tauri command module.

use std::path::PathBuf;
use std::sync::{atomic::AtomicU64, Mutex};
use tauri::WebviewWindow;

#[derive(Default)]
pub(crate) struct AppState {
    pub(crate) config_lock: Mutex<()>,
    pub(crate) providers_lock: Mutex<()>,
    pub(crate) mcp_lock: Mutex<()>,
    pub(crate) auth_lock: tokio::sync::Mutex<()>,
    /// Serializes direct-session logins so the startup warm-up and the first
    /// direct-only tool cannot sign in twice at the same time.
    pub(crate) direct_lock: tokio::sync::Mutex<()>,
    pub(crate) generation: AtomicU64,
    pub(crate) auth: crate::auth::Manager,
    pub(crate) school_lock: tokio::sync::Mutex<()>,
    pub(crate) evaluations: crate::school::evaluation::Manager,
    pub(crate) agent: crate::agent::Manager,
    /// Serializes timetable customization reads and writes.
    pub(crate) timetable_lock: Mutex<()>,
    /// Serializes writes to the local file library's index. Importing hashes
    /// and copies outside this lock; only the commit needs it.
    pub(crate) library_lock: std::sync::Arc<std::sync::Mutex<()>>,
    /// One migration at a time; a second run would copy half-moved data.
    pub(crate) storage_lock: tokio::sync::Mutex<()>,
    /// App-only font files, downloads and their index.
    pub(crate) fonts: crate::fonts::Manager,
}

pub(crate) fn accounts_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::storage::path(app, "state/campus-accounts.enc")
}

/// Stable identifier for the campus login's device fingerprint. The school ties
/// "trusted device" decisions to it, so it is generated once per installation;
/// a failed write only means the device is never remembered, never a login
/// failure. It holds no personal data.
pub(crate) fn device_visitor(app: &tauri::AppHandle) -> Result<String, String> {
    let path = crate::storage::path(app, "state/device-id")?;
    if let Ok(text) = std::fs::read_to_string(&path) {
        let id = text.trim();
        if id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Ok(id.to_string());
        }
    }
    let id = uuid::Uuid::new_v4().simple().to_string();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &id);
    Ok(id)
}

pub(crate) fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("此窗口不能访问启动器能力".into())
    }
}

pub(crate) fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    crate::storage::config_path(app)
}
