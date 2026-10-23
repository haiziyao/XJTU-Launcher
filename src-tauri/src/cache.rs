//! Stored copies of campus query results.
//!
//! Pages show the last stored value first and refresh in the background, so a
//! tab does not start empty after the app restarts. Entries are scoped to the
//! active campus account and saved as JSON, so switching accounts never serves
//! the previous account's data. Caching is best effort: a missing or corrupt
//! entry is treated as a miss.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use tauri::{AppHandle, Manager};

/// A campus payload is small; anything larger is not worth reading back.
const MAX_ENTRY: usize = 8 * 1024 * 1024;
/// Longest cache key accepted from the page.
const MAX_KEY: usize = 512;

/// Cache entries belong to the active account. With no account the entry can
/// still be stored, but a later login uses a different namespace.
fn scope(app: &AppHandle) -> String {
    crate::accounts_path(app)
        .and_then(|path| crate::accounts::load(&path))
        .ok()
        .and_then(|book| book.active_id)
        .unwrap_or_else(|| "anonymous".into())
}

fn file_name(scope: &str, key: &str) -> String {
    let digest = Sha256::digest(format!("{scope}\n{key}"));
    format!("{}.json", hex::encode(digest))
}

fn entry_path(app: &AppHandle, key: &str) -> Result<std::path::PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位缓存目录")?
        .join("cache");
    Ok(dir.join(file_name(&scope(app), key)))
}

fn check_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > MAX_KEY {
        return Err("缓存名称无效".into());
    }
    Ok(())
}

pub fn read(app: &AppHandle, key: &str) -> Option<Value> {
    check_key(key).ok()?;
    let path = entry_path(app, key).ok()?;
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn write(app: &AppHandle, key: &str, value: &Value) -> Result<(), String> {
    check_key(key)?;
    let path = entry_path(app, key)?;
    let bytes = serde_json::to_vec(value).map_err(|error| format!("无法序列化缓存：{error}"))?;
    if bytes.len() > MAX_ENTRY {
        return Ok(());
    }
    let dir = path.parent().ok_or("无法定位缓存目录")?;
    fs::create_dir_all(dir).map_err(|error| format!("无法创建缓存目录：{error}"))?;
    // Write beside the target and rename so a crash cannot leave half a file.
    let temp = path.with_extension("json.tmp");
    fs::write(&temp, &bytes).map_err(|error| format!("无法写入缓存：{error}"))?;
    fs::rename(&temp, &path).map_err(|error| format!("无法保存缓存：{error}"))?;
    Ok(())
}

#[tauri::command]
pub fn cache_read(
    app: AppHandle,
    window: tauri::WebviewWindow,
    key: String,
) -> Result<Option<Value>, String> {
    crate::main_only(&window)?;
    Ok(read(&app, &key))
}

#[tauri::command]
pub fn cache_write(
    app: AppHandle,
    window: tauri::WebviewWindow,
    key: String,
    value: Value,
) -> Result<(), String> {
    crate::main_only(&window)?;
    write(&app, &key, &value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_are_scoped_by_account_and_key() {
        let one = file_name("account-a", "grades");
        assert_eq!(one, file_name("account-a", "grades"));
        assert_ne!(one, file_name("account-b", "grades"));
        assert_ne!(one, file_name("account-a", "exams"));
        assert!(one.ends_with(".json"));
    }

    #[test]
    fn empty_or_oversized_keys_are_rejected() {
        assert!(check_key("").is_err());
        assert!(check_key(&"x".repeat(MAX_KEY + 1)).is_err());
        assert!(check_key("grades").is_ok());
    }
}
