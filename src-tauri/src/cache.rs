//! Stored copies of campus query results.
//!
//! Pages show the last stored value first and refresh in the background, so a
//! tab does not start empty after the app restarts. Entries live in a directory
//! per campus account and are saved as JSON, so switching accounts never serves
//! the previous account's data. Writes happen next to the query that produced
//! them, under the account that query used, so an answer that arrives after a
//! switch cannot land in the new account. Caching is best effort: a missing or
//! corrupt entry is treated as a miss.
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/// A campus payload is small; anything larger is not worth reading back.
const MAX_ENTRY: usize = 8 * 1024 * 1024;
/// Longest cache key accepted from the page.
const MAX_KEY: usize = 512;
/// Scope used before any account exists, matching a fresh installation.
const ANONYMOUS: &str = "anonymous";

fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value))
}

/// Entries belong to the active account. With no account the entry can still
/// be stored under [`ANONYMOUS`], but a later login uses another directory.
pub fn active_scope(app: &AppHandle) -> String {
    crate::accounts_path(app)
        .and_then(|path| crate::accounts::load(&path))
        .ok()
        .and_then(|book| book.active_id)
        .unwrap_or_else(|| ANONYMOUS.into())
}

fn cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位缓存目录")?
        .join("cache"))
}

/// The account id is hashed so a directory name cannot be read as an identity.
fn entry_path_in(base: &Path, scope: &str, key: &str) -> PathBuf {
    base.join(digest(scope))
        .join(format!("{}.json", digest(key)))
}

fn check_key(key: &str) -> Result<(), String> {
    if key.is_empty() || key.len() > MAX_KEY {
        return Err("缓存名称无效".into());
    }
    Ok(())
}

fn read_at(base: &Path, scope: &str, key: &str) -> Option<Value> {
    check_key(key).ok()?;
    let bytes = fs::read(entry_path_in(base, scope, key)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn read(app: &AppHandle, key: &str) -> Option<Value> {
    read_at(&cache_dir(app).ok()?, &active_scope(app), key)
}

fn write_at(base: &Path, scope: &str, key: &str, value: &Value) {
    if check_key(key).is_err() {
        return;
    }
    let Ok(bytes) = serde_json::to_vec(value) else {
        return;
    };
    if bytes.len() > MAX_ENTRY {
        return;
    }
    let path = entry_path_in(base, scope, key);
    let Some(dir) = path.parent() else {
        return;
    };
    if fs::create_dir_all(dir).is_err() {
        return;
    }
    // Write beside the target and rename so a crash cannot leave half a file.
    let temp = path.with_extension("json.tmp");
    if fs::write(&temp, &bytes).is_ok() {
        let _ = fs::rename(&temp, &path);
    }
}

/// Save a query result under the account that produced it, not whichever
/// account happens to be active when a slow answer arrives.
pub fn write_for(app: &AppHandle, scope: &str, key: &str, value: &Value) {
    if let Ok(base) = cache_dir(app) {
        write_at(&base, scope, key, value);
    }
}

fn purge_at(base: &Path, scope: &str) {
    let _ = fs::remove_dir_all(base.join(digest(scope)));
}

/// Drop everything stored for one account; used when the account is removed.
pub fn purge(app: &AppHandle, scope: &str) {
    if let Ok(base) = cache_dir(app) {
        purge_at(&base, scope);
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accounts_and_keys_get_separate_files() {
        let base = Path::new("/cache");
        let one = entry_path_in(base, "account-a", "grades");
        assert_eq!(one, entry_path_in(base, "account-a", "grades"));
        assert_ne!(one, entry_path_in(base, "account-b", "grades"));
        assert_ne!(one, entry_path_in(base, "account-a", "exams"));
        assert!(one.starts_with(base));
        assert!(one.to_string_lossy().ends_with(".json"));
    }

    #[test]
    fn entries_do_not_cross_accounts() {
        let base = std::env::temp_dir().join(format!(
            "xjtu-cache-test-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        let value = serde_json::json!({ "name": "one" });
        write_at(&base, "account-a", "grades", &value);
        assert_eq!(read_at(&base, "account-a", "grades"), Some(value.clone()));
        assert_eq!(read_at(&base, "account-b", "grades"), None);
        assert_eq!(read_at(&base, "account-a", "exams"), None);
        // Removing one account leaves another account's entries alone.
        write_at(&base, "account-b", "grades", &value);
        purge_at(&base, "account-a");
        assert_eq!(read_at(&base, "account-a", "grades"), None);
        assert_eq!(read_at(&base, "account-b", "grades"), Some(value));
        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn empty_or_oversized_keys_are_rejected() {
        assert!(check_key("").is_err());
        assert!(check_key(&"x".repeat(MAX_KEY + 1)).is_err());
        assert!(check_key("grades").is_ok());
    }

    #[test]
    fn unusable_entries_read_as_a_miss() {
        let base = std::env::temp_dir().join(format!(
            "xjtu-cache-miss-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default()
        ));
        let path = entry_path_in(&base, "account-a", "grades");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"{not json").unwrap();
        assert_eq!(read_at(&base, "account-a", "grades"), None);
        let _ = fs::remove_dir_all(&base);
    }
}
