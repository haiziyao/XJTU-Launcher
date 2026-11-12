//! Tauri commands for the mcp domain.

use crate::*;
use std::path::PathBuf;
use tauri::WebviewWindow;

pub(crate) fn mcp_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    storage::path(app, "state/mcp.json")
}
#[tauri::command]
pub(crate) fn list_mcp_servers(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
) -> Result<Vec<mcp::PublicServer>, String> {
    main_only(&window)?;
    let _guard = state.mcp_lock.lock().map_err(|_| "MCP 配置服务繁忙")?;
    Ok(mcp::public(&mcp::load(&mcp_path(&app)?)?))
}
#[tauri::command]
pub(crate) fn save_mcp_server(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    input: mcp::Input,
) -> Result<String, String> {
    main_only(&window)?;
    let _guard = state.mcp_lock.lock().map_err(|_| "MCP 配置服务繁忙")?;
    let path = mcp_path(&app)?;
    let mut book = mcp::load(&path)?;
    let id = mcp::upsert(&mut book, input)?;
    mcp::save(&path, &book)?;
    agent::invalidate_for_mcp(&app);
    Ok(id)
}
#[tauri::command]
pub(crate) fn remove_mcp_server(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.mcp_lock.lock().map_err(|_| "MCP 配置服务繁忙")?;
    let path = mcp_path(&app)?;
    let mut book = mcp::load(&path)?;
    let before = book.servers.len();
    book.servers.retain(|server| server.id != id);
    if book.servers.len() == before {
        return Err("MCP 服务不存在".into());
    }
    mcp::save(&path, &book)?;
    agent::invalidate_for_mcp(&app);
    Ok(())
}
#[tauri::command]
pub(crate) fn set_mcp_server_enabled(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
    enabled: bool,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.mcp_lock.lock().map_err(|_| "MCP 配置服务繁忙")?;
    let path = mcp_path(&app)?;
    let mut book = mcp::load(&path)?;
    let server = book
        .servers
        .iter_mut()
        .find(|server| server.id == id)
        .ok_or("MCP 服务不存在")?;
    server.enabled = enabled;
    mcp::save(&path, &book)?;
    agent::invalidate_for_mcp(&app);
    Ok(())
}
#[tauri::command]
pub(crate) async fn probe_mcp_server(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    id: String,
) -> Result<Vec<mcp::Tool>, String> {
    main_only(&window)?;
    let server = {
        let _guard = state.mcp_lock.lock().map_err(|_| "MCP 配置服务繁忙")?;
        mcp::load(&mcp_path(&app)?)?
            .servers
            .into_iter()
            .find(|server| server.id == id)
            .ok_or("MCP 服务不存在")?
    };
    mcp::probe(&server).await
}
// Image work runs off the main thread so the window stays responsive.
// The source goes back as raw bytes; the webview decodes the crop preview natively.
