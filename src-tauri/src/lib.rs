mod campus;
mod config;
mod plugins;
mod providers;
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
    providers_lock: Mutex<()>,
    auth_lock: tokio::sync::Mutex<()>,
    generation: AtomicU64,
    auth: auth::Manager,
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

// Compatibility with the old, unapproved settings page. Do not open a remote login UI.
#[tauri::command]
fn begin_login(window: WebviewWindow, degree: String) -> Result<(), String> {
    main_only(&window)?;
    campus::domain(&degree)?;
    Err("校园认证已改为应用自有流程；登录表单待设计确认后接入".into())
}

#[tauri::command]
async fn start_account_login(
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    degree: String,
) -> Result<auth::View, String> {
    main_only(&window)?;
    state.auth.insert(auth::Login::begin(degree).await?)
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
        login.act(action).await?;
    }
    let view = login.view();
    if let Some(account) = login.completed.as_ref() {
        let _guard = state.auth_lock.lock().await;
        if handle.cancelled.load(Ordering::SeqCst) {
            return Err("登录已取消".into());
        }
        let path = accounts_path(&app)?;
        let mut book = accounts::load(&path)?;
        book.upsert(account.clone())?;
        accounts::save(&path, &book)?;
        login.completed = None;
        state.generation.fetch_add(1, Ordering::SeqCst);
        state.auth.cancel(&flow_id)?;
        account_changed(&app, "校园账号已添加");
    }
    Ok(view)
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
    login.captcha().await
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

#[tauri::command]
fn open_external(app: tauri::AppHandle, window: WebviewWindow, url: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    main_only(&window)?;
    plugins::validate_url(&url)?;
    app.opener()
        .open_url(url, None::<&str>)
        .map_err(|_| "无法打开系统浏览器".into())
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_model_providers,
            save_model_provider,
            remove_model_provider,
            select_model_provider,
            read_personal_image,
            load_config,
            save_config,
            list_plugins,
            import_plugin,
            account_status,
            begin_login,
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
            open_external
        ])
        .run(tauri::generate_context!())
        .expect("无法启动 XJTU Launcher");
}
