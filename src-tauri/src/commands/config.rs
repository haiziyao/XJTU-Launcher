//! Tauri commands for the config domain.

use crate::*;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) fn load_config(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
) -> Result<config::AppConfig, String> {
    main_only(&window)?;
    let _guard = state.config_lock.lock().map_err(|_| "配置服务繁忙")?;
    config::load(&config_path(&app)?)
}

#[tauri::command]
pub(crate) fn save_config(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    config: config::AppConfig,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.config_lock.lock().map_err(|_| "配置服务繁忙")?;
    config::save(&config_path(&app)?, &config)?;
    drop(_guard);
    pet_desktop::sync_config(&app, &config);
    Ok(())
}

#[tauri::command]
pub(crate) fn list_plugins(window: WebviewWindow) -> Result<plugins::PluginList, String> {
    main_only(&window)?;
    plugins::list()
}

#[tauri::command]
pub(crate) fn plugin_settings(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    plugin_settings::load(&app, &id)
}

#[tauri::command]
pub(crate) fn save_plugin_settings(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
    settings: serde_json::Value,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    let saved = plugin_settings::save(&app, &id, &settings)?;
    if id == plugin_settings::PET_ID {
        pet_desktop::sync_settings(&app);
    }
    Ok(saved)
}
