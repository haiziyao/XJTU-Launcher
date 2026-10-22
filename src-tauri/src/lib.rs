mod campus;
mod campus_browser;
mod config;
mod market;
mod plugins;
mod providers;
mod school;
mod updates;
mod vault;

mod accounts;
mod auth;
mod network;

use serde::Serialize;
use std::{
    path::PathBuf,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex,
    },
};
use tauri::{Emitter, Manager, WebviewWindow};

#[derive(Default)]
struct AppState {
    config_lock: Mutex<()>,
    market_lock: tokio::sync::Mutex<()>,
    providers_lock: Mutex<()>,
    auth_lock: tokio::sync::Mutex<()>,
    generation: AtomicU64,
    auth: auth::Manager,
    school_lock: tokio::sync::Mutex<()>,
    evaluations: school::evaluation::Manager,
}

fn accounts_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位账号目录")?
        .join("campus-accounts.enc"))
}

fn main_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == "main" {
        Ok(())
    } else {
        Err("此窗口不能访问启动器能力".into())
    }
}

fn config_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_config_dir()
        .map_err(|_| "无法定位配置目录")?
        .join("config.json"))
}

fn plugin_directory(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位应用数据目录")?
        .join("plugins"))
}

#[tauri::command]
fn load_config(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
) -> Result<config::AppConfig, String> {
    main_only(&window)?;
    let _guard = state.config_lock.lock().map_err(|_| "配置服务繁忙")?;
    config::load(&config_path(&app)?)
}

#[tauri::command]
fn save_config(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    config: config::AppConfig,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.config_lock.lock().map_err(|_| "配置服务繁忙")?;
    config::save(&config_path(&app)?, &config)
}

#[tauri::command]
fn list_plugins(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<plugins::PluginList, String> {
    main_only(&window)?;
    plugins::list(&plugin_directory(&app)?)
}

#[tauri::command]
fn import_plugin(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    directory: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.config_lock.lock().map_err(|_| "插件服务繁忙")?;
    plugins::import(std::path::Path::new(&directory), &plugin_directory(&app)?)
}

#[tauri::command]
fn plugin_source(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    let config = config::load(&config_path(&app)?)?;
    Ok(
        serde_json::json!({"source": market::source(&config)?, "defaultSource": market::default_server()}),
    )
}

#[tauri::command]
async fn plugin_catalog(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<market::RemoteCatalog, String> {
    main_only(&window)?;
    let mut catalog = market::catalog(&config::load(&config_path(&app)?)?).await?;
    catalog.updates =
        market::available_updates(&catalog, &plugins::list(&plugin_directory(&app)?)?);
    Ok(catalog)
}

#[tauri::command]
async fn install_market_plugin(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    id: String,
    version: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _operation = state
        .market_lock
        .try_lock()
        .map_err(|_| "另一个插件正在安装")?;
    let config = config::load(&config_path(&app)?)?;
    let catalog = market::catalog(&config).await?;
    let source = catalog.source.ok_or("尚未配置插件服务器")?;
    let entry = catalog
        .plugins
        .iter()
        .find(|p| p.id == id && p.version == version)
        .ok_or("目录版本已改变，请刷新后再试")?;
    let package = market::download(&source, entry, |received, total| {
        let _ = window.emit(
            "plugin-download",
            serde_json::json!({"id": id, "received": received, "total": total}),
        );
    })
    .await?;
    let _guard = state.config_lock.lock().map_err(|_| "插件服务繁忙")?;
    if market::source(&config::load(&config_path(&app)?)?)?.as_deref() != Some(&source) {
        return Err("插件源已更改，请重新安装".into());
    }
    market::install(&plugin_directory(&app)?, package)?;
    let _ = window.emit("plugins-changed", ());
    Ok(())
}

#[tauri::command]
fn remove_plugin(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<AppState>,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _operation = state
        .market_lock
        .try_lock()
        .map_err(|_| "请等待插件安装完成")?;
    let _guard = state.config_lock.lock().map_err(|_| "插件服务繁忙")?;
    market::remove(&plugin_directory(&app)?, &id)?;
    let _ = window.emit("plugins-changed", ());
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccountStatus {
    connected: bool,
    degree: Option<String>,
    name: Option<String>,
    student_id: Option<String>,
    avatar: Option<String>,
    api_key_set: bool,
}

#[tauri::command]
async fn account_status(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<AccountStatus, String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    let path = accounts_path(&app)?;
    let book = accounts::load(&path)?;
    let active = book.active();
    let legacy = if !path.exists() {
        vault::session()?
    } else {
        None
    };
    Ok(AccountStatus {
        connected: active.is_some() || legacy.is_some(),
        degree: active
            .map(|a| a.degree.clone())
            .or_else(|| legacy.map(|s| s.degree)),
        name: active.and_then(|a| a.name.clone()),
        student_id: active.and_then(|a| a.student_id.clone()),
        avatar: active.and_then(|a| a.avatar.clone()),
        api_key_set: vault::api_key_set()?,
    })
}

#[derive(Clone, Serialize)]
struct AuthEvent {
    state: &'static str,
    message: String,
}
fn account_changed(app: &tauri::AppHandle, message: &str) {
    let _ = app.emit_to(
        "main",
        "campus-auth",
        AuthEvent {
            state: "success",
            message: message.into(),
        },
    );
}

#[tauri::command]
async fn start_account_login(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    degree: String,
    request_id: String,
) -> Result<auth::View, String> {
    main_only(&window)?;
    campus::domain(&degree)?;
    let token = state.auth.reserve(&request_id)?;
    let result = auth::Login::begin(degree).await;
    state.auth.complete(&request_id, &token, result)
}

#[tauri::command]
async fn advance_account_login(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    flow_id: String,
    action: auth::Action,
) -> Result<auth::View, String> {
    main_only(&window)?;
    let handle = state.auth.get(&flow_id)?;
    let mut login = handle.login.lock().await;
    if handle.cancelled.load(Ordering::SeqCst) {
        return Err("登录已取消".into());
    }
    // Retry a failed local persistence operation without submitting school credentials again.
    if login.completed.is_none() {
        let action = match action {
            auth::Action::SavedCredentials {
                account_id,
                captcha,
                remember_password,
            } => {
                let _guard = state.auth_lock.lock().await;
                let book = accounts::load(&accounts_path(&app)?)?;
                let record = book
                    .accounts
                    .iter()
                    .find(|a| a.id == account_id)
                    .ok_or("校园账号不存在")?;
                if record.degree != login.degree() {
                    return Err("所选账号身份与当前登录不匹配".into());
                }
                let password = record
                    .password
                    .as_ref()
                    .ok_or("此账号未保存密码，请输入密码")?;
                auth::Action::Credentials {
                    username: record.username.clone(),
                    password: password.0.clone(),
                    remember_password,
                    captcha,
                }
            }
            other => other,
        };
        login.act(action).await?;
    }
    if handle.cancelled.load(Ordering::SeqCst) {
        return Err("登录已取消".into());
    }
    if let Some(account) = login.completed.as_ref() {
        let _guard = state.auth_lock.lock().await;
        if handle.cancelled.load(Ordering::SeqCst) {
            return Err("登录已取消".into());
        }
        let path = accounts_path(&app)?;
        let mut book = accounts::load(&path)?;
        book.upsert(account.clone())?;
        campus_browser::close_all(&app)?;
        accounts::save(&path, &book)?;
        login.completed = None;
        state.generation.fetch_add(1, Ordering::SeqCst);
        state.auth.cancel(&flow_id)?;
        account_changed(&app, "校园账号已添加");
    }
    Ok(login.view())
}

#[tauri::command]
async fn account_login_captcha(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    flow_id: String,
) -> Result<String, String> {
    main_only(&window)?;
    let handle = state.auth.get(&flow_id)?;
    let login = handle.login.lock().await;
    if handle.cancelled.load(Ordering::SeqCst) {
        return Err("登录已取消".into());
    }
    let image = login.captcha().await?;
    if handle.cancelled.load(Ordering::SeqCst) {
        return Err("登录已取消".into());
    }
    Ok(image)
}

#[tauri::command]
async fn account_login_state(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    flow_id: String,
) -> Result<auth::View, String> {
    main_only(&window)?;
    let handle = state.auth.get(&flow_id)?;
    let login = handle.login.lock().await;
    if handle.cancelled.load(Ordering::SeqCst) {
        return Err("登录已取消".into());
    }
    Ok(login.view())
}

#[tauri::command]
async fn forget_account_password(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    state.auth.cancel_all();
    let path = accounts_path(&app)?;
    let mut book = accounts::load(&path)?;
    book.forget_password(&account_id)?;
    accounts::save(&path, &book)?;
    account_changed(&app, "已删除保存的密码");
    Ok(())
}

#[tauri::command]
async fn cancel_account_login(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    flow_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    state.auth.cancel(&flow_id)
}

#[tauri::command]
async fn list_accounts(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<accounts::AccountList, String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    Ok(accounts::load(&accounts_path(&app)?)?.view())
}

#[tauri::command]
async fn switch_account(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    let path = accounts_path(&app)?;
    let mut book = accounts::load(&path)?;
    book.select(&account_id)?;
    campus_browser::close_all(&app)?;
    accounts::save(&path, &book)?;
    state.generation.fetch_add(1, Ordering::SeqCst);
    account_changed(&app, "已切换校园账号");
    Ok(())
}

#[tauri::command]
async fn remove_account(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    account_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    let path = accounts_path(&app)?;
    let mut book = accounts::load(&path)?;
    let was_active = book.active_id.as_deref() == Some(&account_id);
    book.remove(&account_id)?;
    if was_active {
        campus_browser::close_all(&app)?;
    }
    state.auth.cancel_all();
    accounts::save(&path, &book)?;
    if was_active {
        state.generation.fetch_add(1, Ordering::SeqCst);
    }
    account_changed(&app, "已移除校园账号");
    Ok(())
}

#[tauri::command]
async fn set_account_avatar(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    account_id: String,
    path: Option<String>,
) -> Result<accounts::Profile, String> {
    main_only(&window)?;
    let avatar = path
        .map(|p| accounts::avatar(std::path::Path::new(&p)))
        .transpose()?;
    let _guard = state.auth_lock.lock().await;
    let file = accounts_path(&app)?;
    let mut book = accounts::load(&file)?;
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    account.avatar = avatar;
    let profile = accounts::Profile::from(&*account);
    accounts::save(&file, &book)?;
    account_changed(&app, "头像已更新");
    Ok(profile)
}

#[tauri::command]
async fn logout(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    state.auth.cancel_all();
    campus_browser::close_all(&app)?;
    let path = accounts_path(&app)?;
    let mut book = accounts::load(&path)?;
    if let Some(id) = book.active_id.clone() {
        book.remove(&id)?;
        accounts::save(&path, &book)?;
    }
    vault::delete("campus-session")?;
    state.generation.fetch_add(1, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn save_api_key(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    key: String,
) -> Result<(), String> {
    main_only(&window)?;
    let _guard = state.auth_lock.lock().await;
    vault::save_api_key(&key)
}

#[tauri::command]
async fn query_timetable(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    plugin_id: String,
) -> Result<campus::Timetable, String> {
    main_only(&window)?;
    if plugin_id != plugins::TIMETABLE_ID {
        return Err("此插件没有课表服务权限".into());
    }
    if config::load(&config_path(&app)?)?
        .disabled_plugins
        .contains(&plugin_id)
    {
        return Err("请先启用课表插件".into());
    }
    let generation = state.generation.load(Ordering::SeqCst);
    let session = {
        let _guard = state.auth_lock.lock().await;
        let path = accounts_path(&app)?;
        let book = accounts::load(&path)?;
        if let Some(account) = book.active() {
            vault::Session {
                degree: account.degree.clone(),
                token: account.token.clone(),
                mode: account.mode,
                cookies: Some(account.cookies.clone()),
            }
        } else if !path.exists() {
            vault::session()?.ok_or("请先连接校园账户")?
        } else {
            return Err("请先连接校园账户".into());
        }
    };
    let timetable = campus::timetable(&session).await?;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账户状态已变化，请重新查询".into());
    }
    Ok(timetable)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkCheck {
    /// campus | vpn | manual | neutral
    tone: &'static str,
    message: String,
}

enum SessionRefresh {
    Refreshed,
    NeedsManual,
}

/// Re-run the saved account's login with its stored password. Only used by the
/// explicit sidebar self-check; the password never leaves this process.
async fn refresh_saved_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<SessionRefresh, String> {
    let path = accounts_path(app)?;
    let (account_id, username, degree, password) = {
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("未绑定校园账号")?;
        let saved = record.password.as_ref().ok_or("此账号未保存密码")?;
        (
            record.id.clone(),
            record.username.clone(),
            record.degree.clone(),
            saved.0.clone(),
        )
    };
    let mut login = auth::Login::begin(degree).await?;
    login
        .act(auth::Action::Credentials {
            username,
            password,
            remember_password: true,
            captcha: String::new(),
        })
        .await?;
    let Some(account) = login.completed.take() else {
        // The school login page asked for a captcha (or another verification step).
        return Ok(SessionRefresh::NeedsManual);
    };
    let _guard = state.auth_lock.lock().await;
    let mut book = accounts::load(&path)?;
    if book.active().map(|a| a.id.as_str()) != Some(account_id.as_str()) {
        return Ok(SessionRefresh::NeedsManual);
    }
    book.upsert(account)?;
    accounts::save(&path, &book)?;
    drop(_guard);
    state.generation.fetch_add(1, Ordering::SeqCst);
    campus_browser::close_all(app)?;
    account_changed(app, "校园会话已刷新");
    Ok(SessionRefresh::Refreshed)
}

/// Sidebar self-check: try the campus host through a direct connection first,
/// then through WebVPN, so the pet can report which network is in effect.
#[tauri::command]
async fn check_network(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<NetworkCheck, String> {
    main_only(&window)?;
    let generation = state.generation.load(Ordering::SeqCst);
    let session = {
        let _guard = state.auth_lock.lock().await;
        let path = accounts_path(&app)?;
        let book = accounts::load(&path)?;
        if let Some(account) = book.active() {
            Some(vault::Session {
                degree: account.degree.clone(),
                token: account.token.clone(),
                mode: account.mode,
                cookies: Some(account.cookies.clone()),
            })
        } else if !path.exists() {
            vault::session()?
        } else {
            None
        }
    };
    let Some(session) = session else {
        return Ok(NetworkCheck {
            tone: "neutral",
            message: "未绑定校园账号，无法检测校园网络".into(),
        });
    };
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
        });
    };
    // A bound account can outlive its VPN session; refresh it on the spot when
    // the saved password allows an unattended login. A session that cannot be
    // renewed turns the whole result red.
    if !campus::session_valid(&session).await {
        match refresh_saved_session(&app, &state).await {
            Ok(SessionRefresh::Refreshed) => message.push_str("，校园会话已刷新"),
            Ok(SessionRefresh::NeedsManual) => {
                return Ok(NetworkCheck {
                    tone: "manual",
                    message: "校园会话已失效，请手动重新登录".into(),
                })
            }
            Err(error) => {
                return Ok(NetworkCheck {
                    tone: "manual",
                    message: format!("自动登录失败：{error}，请手动重新登录"),
                })
            }
        }
    }
    Ok(NetworkCheck { tone, message })
}#[tauri::command]
async fn open_external(
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
    // Keep account changes serialized with native cookie import and window creation.
    let _guard = state.auth_lock.lock().await;
    let book = accounts::load(&accounts_path(&app)?)?;
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
    tauri::async_runtime::spawn_blocking(move || {
        campus_browser::open(&app, session, url).map(|_| ())
    })
    .await
    .map_err(|_| "校园窗口初始化失败")?
}

#[tauri::command]
fn school_catalog(window: WebviewWindow) -> Result<Vec<school::Feature>, String> {
    main_only(&window)?;
    Ok(school::catalog())
}

fn require_school_feature(app: &tauri::AppHandle, feature: &str) -> Result<(), String> {
    if config::load(&config_path(app)?)?
        .disabled_plugins
        .contains(&format!("cn.xjtu.{feature}"))
    {
        return Err("请先启用此校园插件".into());
    }
    Ok(())
}

async fn school_account(
    app: &tauri::AppHandle,
    state: &AppState,
) -> Result<(accounts::Account, u64), String> {
    let _guard = state.auth_lock.lock().await;
    let book = accounts::load(&accounts_path(app)?)?;
    Ok((
        book.active().ok_or("请先登录校园账号")?.clone(),
        state.generation.load(Ordering::SeqCst),
    ))
}

async fn save_school_session(
    app: &tauri::AppHandle,
    state: &AppState,
    account_id: &str,
    generation: u64,
    cookies: String,
) -> Result<(), String> {
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新查询".into());
    }
    let path = accounts_path(app)?;
    let mut book = accounts::load(&path)?;
    if book.active_id.as_deref() != Some(account_id) {
        return Err("账号已切换，请重新查询".into());
    }
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    account.cookies = cookies;
    accounts::save(&path, &book)
}

#[tauri::command]
async fn school_query(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    query: school::Query,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    require_school_feature(&app, query.feature())?;
    query.validate()?;
    let _service_guard = state.school_lock.lock().await;
    if matches!(query, school::Query::Calendar) {
        return school::query(None, query).await.map(|(data, _)| data);
    }
    let (account, generation) = school_account(&app, &state).await?;
    let account_id = account.id.clone();
    let (data, cookies) = school::query(Some(account), query).await?;
    if let Some(cookies) = cookies {
        save_school_session(&app, &state, &account_id, generation, cookies).await?;
    }
    Ok(data)
}

#[tauri::command]
async fn school_prepare_evaluations(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    input: school::evaluation::Prepare,
) -> Result<school::evaluation::Preview, String> {
    main_only(&window)?;
    require_school_feature(&app, "evaluations")?;
    let _service_guard = state.school_lock.lock().await;
    let (account, generation) = school_account(&app, &state).await?;
    let account_id = account.id.clone();
    let (preview, cookies) = state
        .evaluations
        .prepare(account, generation, input)
        .await?;
    save_school_session(&app, &state, &account_id, generation, cookies).await?;
    Ok(preview)
}

#[tauri::command]
async fn school_submit_evaluations(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    preview_id: String,
) -> Result<Vec<school::evaluation::Outcome>, String> {
    main_only(&window)?;
    require_school_feature(&app, "evaluations")?;
    let _service_guard = state.school_lock.lock().await;
    // Serialize account changes with writes to the school. Cancellation remains independent.
    let _account_guard = state.auth_lock.lock().await;
    let path = accounts_path(&app)?;
    let mut book = accounts::load(&path)?;
    let account = book.active().ok_or("请先登录校园账号")?.clone();
    let account_id = account.id.clone();
    let generation = state.generation.load(Ordering::SeqCst);
    let (results, cookies) = state
        .evaluations
        .submit(&preview_id, account, generation, |progress| {
            let _ = window.emit("school-evaluation-progress", progress);
        })
        .await?;
    // A local save failure must not turn already accepted submissions into retryable failures.
    let saved = if let Some(cookies) = cookies {
        if let Some(account) = book.accounts.iter_mut().find(|a| a.id == account_id) {
            account.cookies = cookies;
        }
        accounts::save(&path, &book).is_ok()
    } else {
        false
    };
    if !saved {
        let _ = window.emit(
            "school-session-save-failed",
            "评教结果已返回，但本地会话保存失败",
        );
    }
    Ok(results)
}

#[tauri::command]
fn school_cancel_evaluations(
    window: WebviewWindow,
    state: tauri::State<AppState>,
    preview_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    state.evaluations.cancel(&preview_id)
}

fn providers_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位模型配置目录")?
        .join("model-profiles.enc"))
}
#[tauri::command]
fn list_model_providers(
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
async fn list_provider_models(
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
fn save_model_provider(
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
fn remove_model_provider(
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
fn select_model_provider(
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
fn read_personal_image(
    window: WebviewWindow,
    path: String,
    kind: String,
) -> Result<String, String> {
    use base64::Engine;
    main_only(&window)?;
    if kind == "avatar" {
        return accounts::avatar(std::path::Path::new(&path));
    }
    if kind != "wallpaper" {
        return Err("图片类型无效".into());
    }
    let meta = std::fs::metadata(&path).map_err(|_| "无法读取图片")?;
    if !meta.is_file() || meta.len() > 10 * 1024 * 1024 {
        return Err("请选择不超过 10 MB 的图片".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(
        std::fs::read(path).map_err(|_| "无法读取图片")?,
    ))
    .with_guessed_format()
    .map_err(|_| "图片格式无效")?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP)
    ) {
        return Err("仅支持 PNG、JPEG 和 WebP".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|_| "图片损坏或尺寸过大")?
        .thumbnail(1920, 1200);
    let mut output = std::io::Cursor::new(Vec::new());
    img.write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| "无法处理图片")?;
    let value = format!(
        "data:image/png;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(output.into_inner())
    );
    if value.len() > 12 * 1024 * 1024 {
        return Err("图片处理后仍过大，请选择较小图片".into());
    }
    Ok(value)
}

pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .manage(updates::State::default())
        .setup(|app| {
            vault::initialize(app.path().home_dir()?.join(".xjtu-launcher"))?;
            campus_browser::install_menu(app.handle())?;
            Ok(())
        })
        .on_menu_event(|app, event| campus_browser::menu_event(app, event.id().as_ref()))
        .on_window_event(|window, event| {
            if window.label() == "main" && matches!(event, tauri::WindowEvent::Destroyed) {
                let _ = campus_browser::close_all(window.app_handle());
            }
        })
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            plugin_source,
            plugin_catalog,
            install_market_plugin,
            remove_plugin,
            updates::update_status,
            updates::check_app_update,
            updates::install_app_update,
            updates::restart_after_update,
            school_catalog,
            school_query,
            school_prepare_evaluations,
            school_submit_evaluations,
            school_cancel_evaluations,
            list_model_providers,
            list_provider_models,
            save_model_provider,
            remove_model_provider,
            select_model_provider,
            read_personal_image,
            load_config,
            save_config,
            list_plugins,
            import_plugin,
            account_status,
            account_login_state,
            forget_account_password,
            start_account_login,
            advance_account_login,
            account_login_captcha,
            cancel_account_login,
            list_accounts,
            switch_account,
            remove_account,
            set_account_avatar,
            logout,
            save_api_key,
            query_timetable,
            check_network,
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 XJTU Launcher");
}
