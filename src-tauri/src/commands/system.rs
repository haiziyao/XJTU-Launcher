//! Tauri commands for the system domain.

use crate::*;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) async fn save_export(
    window: WebviewWindow,
    path: String,
    data: String,
) -> Result<u64, String> {
    const MAX_EXPORT: usize = 256 * 1024 * 1024;
    main_only(&window)?;
    if data.len() > MAX_EXPORT / 3 * 4 + 8 {
        return Err("导出文件超过 256 MB，未保存".into());
    }
    let bytes = {
        use base64::{engine::general_purpose::STANDARD, Engine};
        STANDARD
            .decode(data.as_bytes())
            .map_err(|_| "导出数据无效")?
    };
    if bytes.len() > MAX_EXPORT {
        return Err("导出文件超过 256 MB，未保存".into());
    }
    let target = std::path::PathBuf::from(path);
    if !target.is_absolute() {
        return Err("请选择有效的保存位置".into());
    }
    let written = bytes.len() as u64;
    tauri::async_runtime::spawn_blocking(move || {
        crate::config::write_atomic(&target, &bytes).map(|_| written)
    })
    .await
    .map_err(|_| "导出任务失败".to_string())?
}

/// Entry icons the user dropped into `~/.xjtu-launcher/resources/icons`.
/// Read-only and best effort: a broken file returns an empty set, never an
/// error that would keep the sidebar from drawing its built-in glyphs.
#[tauri::command]
pub(crate) fn entry_icons(window: WebviewWindow, app: tauri::AppHandle) -> icons::IconSet {
    if main_only(&window).is_err() {
        return icons::IconSet::default();
    }
    icons::load(&app).unwrap_or_default()
}
