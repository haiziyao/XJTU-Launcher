//! Tauri commands for the network domain.

use super::session::{refresh_saved_session, renew_direct_session, NetworkCheck, SessionRefresh};
use crate::*;
use std::sync::atomic::Ordering;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) async fn check_network(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<NetworkCheck, String> {
    main_only(&window)?;
    let generation = state.generation.load(Ordering::SeqCst);
    let (session, account) = {
        let _guard = state.auth_lock.lock().await;
        let path = accounts_path(&app)?;
        let book = accounts::load(&path)?;
        if let Some(account) = book.active() {
            (
                Some(vault::Session {
                    degree: account.degree.clone(),
                    token: account.token.clone(),
                    mode: account.mode,
                    cookies: Some(account.cookies.clone()),
                }),
                Some(account.clone()),
            )
        } else if !path.exists() {
            (vault::session()?, None)
        } else {
            (None, None)
        }
    };
    let Some(session) = session else {
        return Ok(NetworkCheck {
            tone: "neutral",
            message: "未绑定校园账号，无法检测校园网络".into(),
            direct_session: false,
        });
    };
    let mut direct_session = account
        .as_ref()
        .is_some_and(|account| account.direct_cookies.is_some());
    let direct = campus::reachable(&session, network::AccessMode::Direct).await;
    let vpn = !direct && campus::reachable(&session, network::AccessMode::Webvpn).await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账户状态已变化，请重新检查".into());
    }
    let (tone, mut message) = if direct {
        ("campus", "校园网环境下，网络可正常访问".to_string())
    } else if vpn {
        ("vpn", "检测到非校园网环境，目前自动连接vpn".to_string())
    } else {
        return Ok(NetworkCheck {
            tone: "neutral",
            message: "校园网和 vpn 均不可用".into(),
            direct_session,
        });
    };
    // A bound account can outlive its VPN session; refresh it on the spot when
    // the saved password allows an unattended login. A session that cannot be
    // renewed turns the whole result red.
    let academic_ok = match &account {
        Some(account) => school::academic_available(account).await,
        None => true,
    };
    if !campus::session_valid(&session).await || !academic_ok {
        match refresh_saved_session(&app, &state).await {
            Ok(SessionRefresh::Refreshed) => message.push_str("，校园会话已刷新"),
            Ok(SessionRefresh::NeedsManual) => {
                return Ok(NetworkCheck {
                    tone: "manual",
                    message: "校园会话已失效，请手动重新登录".into(),
                    direct_session,
                })
            }
            Err(error) => {
                return Ok(NetworkCheck {
                    tone: "manual",
                    message: format!("自动登录失败：{error}，请手动重新登录"),
                    direct_session,
                })
            }
        }
    }
    // Direct-only tools (card, fitness) need their own CAS session. Prepare it
    // here so opening one does not have to wait; failures leave the existing
    // "VPN 下不可用" label untouched.
    if let Some(account) = &account {
        if account.mode == network::AccessMode::Webvpn
            && account.direct_cookies.is_none()
            && account.password.is_some()
            && renew_direct_session(&app, &state, false).await.is_ok()
        {
            direct_session = true;
        }
    }
    Ok(NetworkCheck {
        tone,
        message,
        direct_session,
    })
}
/// Public notice boards shown on the home page. No account or cookies are
/// involved; the closed channel list lives in `school::notices`.
#[tauri::command]
pub(crate) async fn campus_notices(
    window: WebviewWindow,
    channel: String,
    limit: u32,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    school::notices::list(&channel, limit).await
}

#[tauri::command]
pub(crate) async fn open_external(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    url: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    main_only(&window)?;
    let url = plugins::validate_url(&url)?;
    if !network::campus_web_url(&url) {
        return app
            .opener()
            .open_url(url.as_str(), None::<&str>)
            .map_err(|_| "无法打开系统浏览器".into());
    }
    open_campus_url(&app, &state, url).await
}

/// Opens a school URL in the native campus window with the active account.
pub(crate) async fn open_campus_url(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    url: url::Url,
) -> Result<(), String> {
    // Keep account changes serialized with native cookie import and window creation.
    let _guard = state.auth_lock.lock().await;
    let book = accounts::load(&accounts_path(app)?)?;
    let account = book.active().ok_or("请先添加并登录校园账号")?;
    let session = campus_browser::Session {
        account_id: account.id.clone(),
        generation: state.generation.load(Ordering::SeqCst),
        display_name: account
            .name
            .clone()
            .unwrap_or_else(|| account.username.clone()),
        mode: account.mode,
        cookies: account.cookies.clone(),
    };
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        campus_browser::open(&app, session, url).map(|_| ())
    })
    .await
    .map_err(|_| "校园窗口初始化失败")?
}
