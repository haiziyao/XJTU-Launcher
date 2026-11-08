//! Work state for the replay capture page: scan settings, checkpoint, page
//! list and the JPEG originals.
//!
//! Everything lives under the campus cache directory for the active account
//! (`cache/<account>/capture/<activity>/`), so switching accounts never shows
//! the previous person's slides and deleting an account removes them with the
//! rest of that account's cache. Nothing here stores the replay address or any
//! credential; a session is only the lesson id, the settings and the images the
//! user chose to keep.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tauri::{AppHandle, WebviewWindow};

/// Same ceiling as the capture page's grid.
const MAX_PAGES: usize = 1000;
/// One slide JPEG is far below this even at full resolution.
const MAX_PAGE_BYTES: usize = 4 * 1024 * 1024;
/// Session settings are tiny; a bigger payload is not ours.
const MAX_SESSION_BYTES: usize = 1024 * 1024;

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct CapturePage {
    pub id: u64,
    #[serde(default)]
    pub order: u32,
    #[serde(default)]
    pub time: f64,
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
    #[serde(default)]
    pub signature: Vec<f64>,
    #[serde(default)]
    pub bytes: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureSession {
    pub session: serde_json::Value,
    pub pages: Vec<CapturePage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureImage {
    pub id: u64,
    pub data: String,
}

#[derive(Deserialize)]
pub struct CaptureOrder {
    pub id: u64,
    pub order: u32,
}

fn digest(value: &str) -> String {
    hex::encode(Sha256::digest(value))
}

fn session_dir(base: &Path, scope: &str, activity: u64) -> PathBuf {
    base.join(digest(scope))
        .join("capture")
        .join(activity.to_string())
}

fn image_path(dir: &Path, id: u64) -> PathBuf {
    dir.join("pages").join(format!("{id}.jpg"))
}

fn read_index(dir: &Path) -> Vec<CapturePage> {
    fs::read(dir.join("pages.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

fn write_index(dir: &Path, pages: &[CapturePage]) -> Result<(), String> {
    let bytes = serde_json::to_vec(pages).map_err(|_| "无法保存截取进度")?;
    crate::config::write_atomic(&dir.join("pages.json"), &bytes)
}

fn load_at(dir: &Path) -> Option<CaptureSession> {
    let bytes = fs::read(dir.join("session.json")).ok()?;
    let session = serde_json::from_slice(&bytes).ok()?;
    Some(CaptureSession {
        session,
        pages: read_index(dir),
    })
}

fn save_session_at(dir: &Path, session: &serde_json::Value) -> Result<(), String> {
    let bytes = serde_json::to_vec(session).map_err(|_| "无法保存截取进度")?;
    if bytes.len() > MAX_SESSION_BYTES {
        return Err("截取进度数据异常".into());
    }
    crate::config::write_atomic(&dir.join("session.json"), &bytes)
}

fn add_page_at(dir: &Path, mut page: CapturePage, data: &str) -> Result<CapturePage, String> {
    let bytes = STANDARD
        .decode(data.as_bytes())
        .map_err(|_| "截取图片数据无效")?;
    if bytes.is_empty() || bytes.len() > MAX_PAGE_BYTES {
        return Err("截取图片大小异常".into());
    }
    let mut pages = read_index(dir);
    if pages.len() >= MAX_PAGES {
        return Err(format!("已达到 {MAX_PAGES} 页上限"));
    }
    let id = pages.iter().map(|page| page.id).max().unwrap_or(0) + 1;
    page.id = id;
    page.bytes = bytes.len() as u64;
    crate::config::write_atomic(&image_path(dir, id), &bytes)?;
    pages.push(page.clone());
    write_index(dir, &pages)?;
    Ok(page)
}

fn remove_pages_at(dir: &Path, ids: &[u64]) -> Result<(), String> {
    let mut pages = read_index(dir);
    pages.retain(|page| !ids.contains(&page.id));
    for id in ids {
        let _ = fs::remove_file(image_path(dir, *id));
    }
    write_index(dir, &pages)
}

fn order_pages_at(dir: &Path, orders: &[CaptureOrder]) -> Result<(), String> {
    let mut pages = read_index(dir);
    for page in &mut pages {
        if let Some(entry) = orders.iter().find(|entry| entry.id == page.id) {
            page.order = entry.order;
        }
    }
    write_index(dir, &pages)
}

fn clear_at(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

fn root(app: &AppHandle, activity: u64) -> Result<PathBuf, String> {
    if activity == 0 {
        return Err("缺少课时".into());
    }
    let base = crate::storage::path(app, "cache")?;
    let scope = crate::cache::active_scope(app);
    Ok(session_dir(&base, &scope, activity))
}

#[tauri::command]
pub fn capture_session_load(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
) -> Result<Option<CaptureSession>, String> {
    crate::main_only(&window)?;
    Ok(load_at(&root(&app, activity)?))
}

#[tauri::command]
pub fn capture_session_save(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
    session: serde_json::Value,
) -> Result<(), String> {
    crate::main_only(&window)?;
    save_session_at(&root(&app, activity)?, &session)
}

#[tauri::command]
pub fn capture_page_add(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
    page: CapturePage,
    data: String,
) -> Result<CapturePage, String> {
    crate::main_only(&window)?;
    add_page_at(&root(&app, activity)?, page, &data)
}

#[tauri::command]
pub fn capture_pages_read(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
    ids: Vec<u64>,
) -> Result<Vec<CaptureImage>, String> {
    crate::main_only(&window)?;
    let dir = root(&app, activity)?;
    let mut images = Vec::new();
    for id in ids {
        if let Ok(bytes) = fs::read(image_path(&dir, id)) {
            images.push(CaptureImage {
                id,
                data: STANDARD.encode(bytes),
            });
        }
    }
    Ok(images)
}

#[tauri::command]
pub fn capture_pages_delete(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
    ids: Vec<u64>,
) -> Result<(), String> {
    crate::main_only(&window)?;
    remove_pages_at(&root(&app, activity)?, &ids)
}

#[tauri::command]
pub fn capture_pages_order(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
    orders: Vec<CaptureOrder>,
) -> Result<(), String> {
    crate::main_only(&window)?;
    order_pages_at(&root(&app, activity)?, &orders)
}

#[tauri::command]
pub fn capture_session_clear(
    app: AppHandle,
    window: WebviewWindow,
    activity: u64,
) -> Result<(), String> {
    crate::main_only(&window)?;
    clear_at(&root(&app, activity)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(time: f64) -> CapturePage {
        CapturePage {
            id: 0,
            order: 0,
            time,
            width: 1280,
            height: 720,
            signature: vec![1.0, 2.0],
            bytes: 0,
        }
    }

    fn base64(bytes: &[u8]) -> String {
        STANDARD.encode(bytes)
    }

    #[test]
    fn pages_round_trip_and_stay_in_their_activity() {
        let temp = tempfile::tempdir().unwrap();
        let first = session_dir(temp.path(), "account-a", 7);
        let second = session_dir(temp.path(), "account-a", 8);
        let other = session_dir(temp.path(), "account-b", 7);

        let added = add_page_at(&first, page(12.5), &base64(b"one")).unwrap();
        assert_eq!(added.id, 1);
        add_page_at(&first, page(30.0), &base64(b"two")).unwrap();
        save_session_at(
            &first,
            &serde_json::json!({ "activity": 7, "checkpoint": 42.0 }),
        )
        .unwrap();

        let loaded = load_at(&first).unwrap();
        assert_eq!(loaded.session["checkpoint"], 42.0);
        assert_eq!(loaded.pages.len(), 2);
        assert_eq!(loaded.pages[1].time, 30.0);
        assert_eq!(loaded.pages[0].bytes, 3);
        assert!(load_at(&second).is_none());
        assert!(load_at(&other).is_none());
    }

    #[test]
    fn delete_order_and_clear_touch_index_and_files() {
        let temp = tempfile::tempdir().unwrap();
        let dir = session_dir(temp.path(), "account", 1);
        let first = add_page_at(&dir, page(1.0), &base64(b"a")).unwrap();
        let second = add_page_at(&dir, page(2.0), &base64(b"b")).unwrap();

        remove_pages_at(&dir, &[first.id]).unwrap();
        assert!(!image_path(&dir, first.id).exists());
        assert!(image_path(&dir, second.id).exists());
        assert_eq!(read_index(&dir).len(), 1);

        order_pages_at(
            &dir,
            &[CaptureOrder {
                id: second.id,
                order: 5,
            }],
        )
        .unwrap();
        assert_eq!(read_index(&dir)[0].order, 5);

        clear_at(&dir);
        assert!(!dir.exists());
        assert!(load_at(&dir).is_none());
    }

    #[test]
    fn rejects_bad_payloads() {
        let temp = tempfile::tempdir().unwrap();
        let dir = session_dir(temp.path(), "account", 1);
        assert!(add_page_at(&dir, page(1.0), "not base64!").is_err());
        assert!(add_page_at(&dir, page(1.0), "").is_err());
        let huge = base64(&vec![0u8; MAX_PAGE_BYTES + 1]);
        assert!(add_page_at(&dir, page(1.0), &huge).is_err());
        assert!(read_index(&dir).is_empty());
    }
}
