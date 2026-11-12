//! Tauri commands for the expose domain.

use crate::*;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) fn expose_status(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, expose::State>,
) -> Result<expose::Status, String> {
    main_only(&window)?;
    let config = config::load(&config_path(&app)?)?;
    Ok(state.status(config.mcp_server_enabled, config.mcp_server_port))
}

#[tauri::command]
pub(crate) fn expose_tools(window: WebviewWindow) -> Result<Vec<expose::Tool>, String> {
    main_only(&window)?;
    Ok(expose::tools())
}

/// A busy port makes the endpoint fall back to a free one; save that port so
/// the next start tries it first instead of stumbling on the same conflict.
pub(crate) fn remember_expose_port(app: &tauri::AppHandle, port: u16) {
    let Ok(path) = config_path(app) else { return };
    let Ok(mut config) = config::load(&path) else {
        return;
    };
    if config.mcp_server_port != port {
        config.mcp_server_port = port;
        let _ = config::save(&path, &config);
    }
}

#[tauri::command]
pub(crate) fn expose_set_enabled(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, expose::State>,
    enabled: bool,
) -> Result<expose::Status, String> {
    main_only(&window)?;
    let port = config::load(&config_path(&app)?)?.mcp_server_port;
    let state = state.inner();
    let port = if enabled {
        let port = expose::start(&app, state, port)?;
        remember_expose_port(&app, port);
        port
    } else {
        expose::stop(state);
        port
    };
    Ok(state.status(enabled, port))
}

/// Restart the listener on a new port. If the new port cannot be bound, the
/// previous listener is restored so a working endpoint is not lost.
#[tauri::command]
pub(crate) fn expose_set_port(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, expose::State>,
    port: u16,
) -> Result<expose::Status, String> {
    main_only(&window)?;
    if !(1024..=65535).contains(&port) {
        return Err("端口需在 1024-65535 之间".into());
    }
    let config = config::load(&config_path(&app)?)?;
    if !config.mcp_server_enabled {
        return Err("请先打开 MCP 模式".into());
    }
    let state = state.inner();
    let previous = state.running_port();
    expose::stop(state);
    match expose::start(&app, state, port) {
        Ok(port) => {
            remember_expose_port(&app, port);
            Ok(state.status(true, port))
        }
        Err(error) => {
            if let Some(previous) = previous {
                let _ = expose::start(&app, state, previous);
            }
            Err(error)
        }
    }
}

/// Runs one cheap campus query so the page can say whether the current school
/// session actually works, rather than inferring it from the address alone.
/// The answer is only success or the error; query details belong in the tools.
#[tauri::command]
pub(crate) async fn expose_probe(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<(), String> {
    main_only(&window)?;
    run_school_query(&app, school::Query::Terms, None, false).await?;
    Ok(())
}
