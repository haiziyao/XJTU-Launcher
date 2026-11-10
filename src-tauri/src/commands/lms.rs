//! Tauri commands for the lms domain.

use super::network::open_campus_url;
use super::school::{require_school_feature, school_account};
use crate::*;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) async fn lms_open_file(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    upload_id: u64,
) -> Result<(), String> {
    main_only(&window)?;
    require_school_feature(&app, "lms")?;
    let _service_guard = state.school_lock.lock().await;
    let (account, _) = school_account(&app, &state).await?;
    let url = school::lms::Client::connect(&account)?
        .open_file(upload_id)
        .await?;
    open_campus_url(&app, &state, url).await
}

/// Stream a 思源学堂 attachment to a user-chosen path.
#[tauri::command]
pub(crate) async fn lms_save_file(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    upload_id: u64,
    path: String,
) -> Result<u64, String> {
    main_only(&window)?;
    require_school_feature(&app, "lms")?;
    let path = std::path::PathBuf::from(path);
    if !path.is_absolute() {
        return Err("请选择有效的保存位置".into());
    }
    // No school lock here: the body comes from the public media host and a
    // large download must not block other campus queries for minutes.
    let (account, _) = school_account(&app, &state).await?;
    school::lms::Client::connect(&account)?
        .save_file(upload_id, &path)
        .await
}

/// Resolve the direct replay videos for one lesson. The addresses are
/// presigned and expire, so the result is never cached; the RMS bearer token
/// stays in the backend.
#[tauri::command]
pub(crate) async fn lms_replay_videos(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    activity: u64,
) -> Result<school::lms::ReplayVideos, String> {
    main_only(&window)?;
    require_school_feature(&app, "lms")?;
    let _service_guard = state.school_lock.lock().await;
    let (account, _) = school_account(&app, &state).await?;
    school::lms::Client::connect(&account)?
        .replay_videos(activity)
        .await
}
