//! Tauri commands for the providers domain.

use crate::*;
use std::path::PathBuf;
use tauri::WebviewWindow;

pub(crate) fn providers_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    storage::path(app, "state/model-profiles.enc")
}
#[tauri::command]
pub(crate) fn list_model_providers(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
) -> Result<providers::View, String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    Ok(providers::load(&providers_path(&app)?)?.view(vault::api_key_set()?))
}
#[tauri::command]
pub(crate) async fn list_provider_models(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    input: providers::ListInput,
) -> Result<Vec<String>, String> {
    main_only(&window)?;
    // Resolve the key while holding the config lock, then release it before I/O.
    let key = {
        let _guard = state
            .providers_lock
            .lock()
            .map_err(|_| "模型配置服务繁忙")?;
        if input.use_legacy_key {
            vault::get("model-api-key")?.unwrap_or_default()
        } else if let Some(key) = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
        {
            key.to_string()
        } else if let Some(id) = input.id.as_deref() {
            providers::load(&providers_path(&app)?)?
                .providers
                .iter()
                .find(|p| p.id == id)
                .map(|p| p.api_key.clone())
                .ok_or("模型配置不存在")?
        } else {
            String::new()
        }
    };
    providers::list_models(&input.base_url, &input.protocol, &key).await
}
#[tauri::command]
pub(crate) fn save_model_provider(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    input: providers::Input,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    let path = providers_path(&app)?;
    let mut book = providers::load(&path)?;
    let legacy = if input.use_legacy_key {
        vault::get("model-api-key")?
    } else {
        None
    };
    book.upsert(input, legacy)?;
    providers::save(&path, &book)
}
#[tauri::command]
pub(crate) fn remove_model_provider(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    let path = providers_path(&app)?;
    let mut book = providers::load(&path)?;
    book.remove(&id)?;
    providers::save(&path, &book)
}
#[tauri::command]
pub(crate) fn select_model_provider(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    let path = providers_path(&app)?;
    let mut book = providers::load(&path)?;
    book.select(&id)?;
    providers::save(&path, &book)
}
#[tauri::command]
pub(crate) fn save_provider_models(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
    models: Vec<String>,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    let path = providers_path(&app)?;
    let mut book = providers::load(&path)?;
    book.set_models(&id, models)?;
    providers::save(&path, &book)
}
#[tauri::command]
pub(crate) fn set_provider_model(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
    model: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    let path = providers_path(&app)?;
    let mut book = providers::load(&path)?;
    book.set_model(&id, &model)?;
    providers::save(&path, &book)
}
/// Shows the stored key on request; it never leaves the local window otherwise.
#[tauri::command]
pub(crate) fn reveal_provider_key(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
) -> Result<String, String> {
    main_only(&window)?;
    let _guard = state
        .providers_lock
        .lock()
        .map_err(|_| "模型配置服务繁忙")?;
    let book = providers::load(&providers_path(&app)?)?;
    let provider = book
        .providers
        .iter()
        .find(|p| p.id == id)
        .ok_or("模型配置不存在")?;
    Ok(provider.api_key.clone())
}
/// Checks reachability, the key and the model endpoint in one call.
#[tauri::command]
pub(crate) async fn test_provider(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<usize, String> {
    main_only(&window)?;
    let (base_url, protocol, key) = {
        let _guard = state
            .providers_lock
            .lock()
            .map_err(|_| "模型配置服务繁忙")?;
        let book = providers::load(&providers_path(&app)?)?;
        let provider = book
            .providers
            .iter()
            .find(|p| p.id == id)
            .ok_or("模型配置不存在")?;
        (
            provider.base_url.clone(),
            provider.protocol.clone(),
            provider.api_key.clone(),
        )
    };
    providers::list_models(&base_url, &protocol, &key)
        .await
        .map(|models| models.len())
}
/// The balance endpoint is vendor specific; unknown vendors report nothing.
#[tauri::command]
pub(crate) async fn provider_balance(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Option<providers::Balance>, String> {
    main_only(&window)?;
    let (base_url, key) = {
        let _guard = state
            .providers_lock
            .lock()
            .map_err(|_| "模型配置服务繁忙")?;
        let book = providers::load(&providers_path(&app)?)?;
        let provider = book
            .providers
            .iter()
            .find(|p| p.id == id)
            .ok_or("模型配置不存在")?;
        (provider.base_url.clone(), provider.api_key.clone())
    };
    providers::balance(&base_url, &key).await
}
