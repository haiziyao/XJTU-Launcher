//! Tauri commands for the storage domain.

use crate::*;
use tauri::WebviewWindow;

pub(crate) async fn run_storage<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|_| "存储任务失败".to_string())?
}

#[tauri::command]
pub(crate) async fn storage_status(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<storage::Status, String> {
    main_only(&window)?;
    run_storage(move || storage::status(&app)).await
}

/// Validates a candidate folder and reports exactly what a migration would
/// move, so the confirmation step can show sizes before anything is touched.
#[tauri::command]
pub(crate) async fn storage_preview(
    app: tauri::AppHandle,
    window: WebviewWindow,
    target: String,
) -> Result<storage::Preview, String> {
    main_only(&window)?;
    run_storage(move || storage::preview(&app, &target)).await
}

#[tauri::command]
pub(crate) async fn storage_migrate(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    target: String,
) -> Result<storage::Report, String> {
    main_only(&window)?;
    let _guard = state.storage_lock.lock().await;
    run_storage(move || storage::migrate(&app, &target)).await
}

#[tauri::command]
pub(crate) fn storage_reveal(app: tauri::AppHandle, window: WebviewWindow) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    main_only(&window)?;
    app.opener()
        .open_path(storage::file_root(&app)?.to_string_lossy(), None::<&str>)
        .map_err(|_| "无法打开文件文件夹".into())
}

/// Turns a chosen path into the `~/...` form the interface shows, so a user
/// name never appears on screen.
#[tauri::command]
pub(crate) fn storage_display_path(app: tauri::AppHandle, path: String) -> String {
    storage::display_for(&app, &path)
}

/// Default location for a save dialog. The backend owns the folder so a page
/// cannot invent a path outside the data folder.
#[tauri::command]
pub(crate) fn storage_download_path(
    app: tauri::AppHandle,
    window: WebviewWindow,
    name: String,
) -> Result<String, String> {
    main_only(&window)?;
    Ok(storage::download_target(&app, &name)?
        .to_string_lossy()
        .into_owned())
}
