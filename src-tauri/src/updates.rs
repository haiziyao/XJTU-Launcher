use serde::Serialize;
use std::{sync::Mutex, time::Duration};
use tauri::{Emitter, WebviewWindow};
use tauri_plugin_updater::UpdaterExt;

const ENDPOINT: Option<&str> = option_env!("XJTU_UPDATE_ENDPOINT");
const PUBLIC_KEY: Option<&str> = option_env!("XJTU_UPDATE_PUBLIC_KEY");
// P0: the mirror has no domain or certificate yet, so it is served over plain
// HTTP (docs/P0.md). Baked in so release builds reach it without the user
// configuring anything; a build variable still takes precedence.
const DEFAULT_ENDPOINT: &str = "http://8.152.100.169/xjtu-launcher/latest.json";

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    current_version: &'static str,
    phase: &'static str,
    version: Option<String>,
    notes: Option<String>,
    received: u64,
    total: Option<u64>,
    message: Option<String>,
}
impl Default for Status {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION"),
            phase: if compiled_configured() {
                "idle"
            } else {
                "unconfigured"
            },
            version: None,
            notes: None,
            received: 0,
            total: None,
            message: None,
        }
    }
}
#[derive(Default)]
pub struct State {
    status: Mutex<Status>,
    update: Mutex<Option<tauri_plugin_updater::Update>>,
    operation: tokio::sync::Mutex<()>,
}
fn builtin_endpoint() -> &'static str {
    ENDPOINT
        .filter(|value| !value.is_empty())
        .unwrap_or(DEFAULT_ENDPOINT)
}
fn compiled_configured() -> bool {
    !builtin_endpoint().is_empty() && PUBLIC_KEY.is_some_and(|v| !v.is_empty())
}
/// The mirror saved in the config wins; otherwise the built-in endpoint is
/// used. The public key always comes from the build.
fn effective_endpoint(app: &tauri::AppHandle) -> Option<String> {
    crate::config_path(app)
        .and_then(|path| crate::config::load(&path))
        .ok()
        .map(|config| config.update_endpoint.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| Some(builtin_endpoint().to_string()))
}
fn configured(app: &tauri::AppHandle) -> bool {
    effective_endpoint(app).is_some() && PUBLIC_KEY.is_some_and(|v| !v.is_empty())
}
fn snapshot(state: &State) -> Result<Status, String> {
    state
        .status
        .lock()
        .map(|s| s.clone())
        .map_err(|_| "更新服务繁忙".into())
}
fn publish(window: &WebviewWindow, state: &State, edit: impl FnOnce(&mut Status)) {
    if let Ok(mut status) = state.status.lock() {
        edit(&mut status);
        let _ = window.emit("app-update", status.clone());
    }
}

#[tauri::command]
pub fn update_status(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<State>,
) -> Result<Status, String> {
    crate::main_only(&window)?;
    let mut status = snapshot(&state)?;
    if !configured(&app) {
        status.phase = "unconfigured";
    } else if status.phase == "unconfigured" {
        status.phase = "idle";
    }
    Ok(status)
}

#[tauri::command]
pub async fn check_app_update(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, State>,
) -> Result<Status, String> {
    crate::main_only(&window)?;
    let _operation = state.operation.try_lock().map_err(|_| "正在处理更新")?;
    if !configured(&app) || snapshot(&state)?.phase == "installed" {
        return snapshot(&state);
    }
    publish(&window, &state, |s| {
        s.phase = "checking";
        s.message = None;
    });
    let result = async {
        let endpoint = effective_endpoint(&app).ok_or("更新地址未配置")?;
        // P0: HTTP is tolerated while the built-in mirror has no certificate;
        // validate_url still rejects credentials and non-HTTP schemes.
        let url = crate::plugins::validate_url(&endpoint)?;
        let updater = app
            .updater_builder()
            .pubkey(PUBLIC_KEY.ok_or("更新公钥未配置")?)
            .endpoints(vec![url])
            .map_err(|_| "更新地址无效")?
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| "更新配置无效")?;
        updater
            .check()
            .await
            .map_err(|_| "无法检查更新，请稍后重试".to_string())
    }
    .await;
    match result {
        Ok(update) => {
            publish(&window, &state, |s| {
                s.phase = if update.is_some() {
                    "available"
                } else {
                    "current"
                };
                s.version = update.as_ref().map(|u| u.version.clone());
                s.notes = update.as_ref().and_then(|u| u.body.clone());
                s.received = 0;
                s.total = None;
            });
            if let Some(available) = update.as_ref() {
                crate::pet_desktop::notify(
                    &app,
                    "update",
                    &format!("应用更新 {} 可用", available.version),
                    Some(format!("app-update-{}", available.version)),
                );
            }
            *state.update.lock().map_err(|_| "更新服务繁忙")? = update;
        }
        Err(error) => publish(&window, &state, |s| {
            s.phase = "error";
            s.message = Some(error);
        }),
    }
    snapshot(&state)
}

#[tauri::command]
pub async fn install_app_update(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, State>,
    app_state: tauri::State<'_, crate::AppState>,
) -> Result<(), String> {
    crate::main_only(&window)?;
    let _operation = state.operation.try_lock().map_err(|_| "正在处理更新")?;
    if snapshot(&state)?.phase == "installed" {
        return Err("更新已安装，请重新启动应用".into());
    }
    let update = state
        .update
        .lock()
        .map_err(|_| "更新服务繁忙")?
        .clone()
        .ok_or("请先检查更新")?;
    publish(&window, &state, |s| {
        s.phase = "downloading";
        s.received = 0;
        s.total = None;
        s.message = None;
    });
    let result = async {
        let bytes = update
            .download(
                |chunk, total| {
                    publish(&window, &state, |s| {
                        s.received += chunk as u64;
                        s.total = total;
                    })
                },
                || {},
            )
            .await
            .map_err(|_| "下载或签名校验失败，请重新检查更新".to_string())?;
        // Hold these locks through installation, including platforms whose installer exits the app.
        let _school = app_state
            .school_lock
            .try_lock()
            .map_err(|_| "校园任务正在执行，请稍后重试更新")?;
        let _auth = app_state
            .auth_lock
            .try_lock()
            .map_err(|_| "账号操作正在执行，请稍后重试更新")?;
        let _market = app_state
            .market_lock
            .try_lock()
            .map_err(|_| "插件正在安装，请稍后重试更新")?;
        publish(&window, &state, |s| s.phase = "installing");
        crate::campus_browser::close_all(&app)?;
        update
            .install(bytes)
            .map_err(|_| "安装更新失败，请稍后重试".to_string())
    }
    .await;
    match result {
        Ok(()) => {
            publish(&window, &state, |s| s.phase = "installed");
            Ok(())
        }
        Err(error) => {
            publish(&window, &state, |s| {
                s.phase = "error";
                s.message = Some(error.clone());
            });
            Err(error)
        }
    }
}

#[tauri::command]
pub fn restart_after_update(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<State>,
    app_state: tauri::State<crate::AppState>,
) -> Result<(), String> {
    crate::main_only(&window)?;
    if snapshot(&state)?.phase != "installed" {
        return Err("尚未完成更新安装".into());
    }
    let _school = app_state
        .school_lock
        .try_lock()
        .map_err(|_| "校园任务正在执行，请稍后重启")?;
    let _auth = app_state
        .auth_lock
        .try_lock()
        .map_err(|_| "账号操作正在执行，请稍后重启")?;
    let _market = app_state
        .market_lock
        .try_lock()
        .map_err(|_| "插件正在安装，请稍后重启")?;
    crate::campus_browser::close_all(&app)?;
    app.restart()
}
