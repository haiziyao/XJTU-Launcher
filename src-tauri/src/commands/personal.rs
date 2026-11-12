//! Tauri commands for the personal domain.

use crate::*;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) async fn personal_image_source(
    window: WebviewWindow,
    path: String,
    kind: String,
) -> Result<tauri::ipc::Response, String> {
    main_only(&window)?;
    let kind = personal_image::Kind::parse(&kind)?;
    let bytes = tauri::async_runtime::spawn_blocking(move || {
        personal_image::source(std::path::Path::new(&path), kind)
    })
    .await
    .map_err(|_| "无法处理图片".to_string())??;
    Ok(tauri::ipc::Response::new(bytes))
}
#[tauri::command]
pub(crate) async fn read_personal_image(
    window: WebviewWindow,
    path: String,
    kind: String,
    crop: personal_image::Crop,
) -> Result<String, String> {
    main_only(&window)?;
    let kind = personal_image::Kind::parse(&kind)?;
    tauri::async_runtime::spawn_blocking(move || {
        personal_image::crop(std::path::Path::new(&path), kind, &crop)
    })
    .await
    .map_err(|_| "无法处理图片".to_string())?
}
