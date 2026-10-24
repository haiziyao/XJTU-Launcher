//! Desktop pet shown while the main window is minimized.
//!
//! The pet lives in its own small, transparent, always-on-top window. It never
//! receives the main window's IPC privileges: only the handful of commands in
//! this module are exposed to it, and they only read its own settings and
//! restore the main window.
use serde::Serialize;
use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{
    AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindow,
    WebviewWindowBuilder,
};

pub const LABEL: &str = "pet";
pub const EVENT_NOTIFICATIONS: &str = "pet-notifications";
pub const EVENT_SETTINGS: &str = "pet-settings";
pub const EVENT_CONFIG_CHANGED: &str = "pet-config-changed";
pub const EVENT_OPEN_SETTINGS: &str = "pet-open-settings";

/// The floating window is a fixed size so a speech bubble and the quick
/// settings card always fit without resizing the window. The dock is anchored
/// to the bottom-left corner of that box.
const WIDTH: f64 = 336.0;
const HEIGHT: f64 = 200.0;
const MARGIN: f64 = 12.0;
const MAX_NOTIFICATIONS: usize = 40;
const MAX_SEEN: usize = 600;
const CAMPUS_INTERVAL: Duration = Duration::from_secs(20 * 60);
const MAX_TEXT: usize = 160;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    pub id: String,
    pub source: String,
    pub text: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub enabled: bool,
    pub character: String,
    pub bar_style: String,
    pub bar_hidden: bool,
    pub bubble: bool,
    pub sources: Vec<String>,
    pub reduce_motion: bool,
    pub accent: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub settings: Settings,
    pub notifications: Vec<Notification>,
}

#[derive(Default)]
pub struct State {
    notifications: Mutex<Vec<Notification>>,
    next_id: AtomicU64,
    /// Cached copy of `pet_desktop_enabled` so the minimize watcher never has
    /// to touch the config file on every tick.
    enabled: AtomicBool,
    /// Shown from the settings page without the main window being minimized.
    preview: AtomicBool,
    /// Last position reported by a drag, flushed once the drag settles.
    pending_position: Mutex<Option<([i32; 2], Instant)>>,
}

fn pet_only(window: &WebviewWindow) -> Result<(), String> {
    if window.label() == LABEL {
        Ok(())
    } else {
        Err("此窗口不能使用桌面宠物接口".into())
    }
}

fn update_config(
    app: &AppHandle,
    edit: impl FnOnce(&mut crate::config::AppConfig),
) -> Result<crate::config::AppConfig, String> {
    let state = app.state::<crate::AppState>();
    let _guard = state.config_lock.lock().map_err(|_| "配置服务繁忙")?;
    let path = crate::config_path(app)?;
    let mut config = crate::config::load(&path)?;
    edit(&mut config);
    config.validate()?;
    crate::config::save(&path, &config)?;
    Ok(config)
}

pub fn settings(app: &AppHandle) -> Result<Settings, String> {
    let config = crate::config::load(&crate::config_path(app)?)?;
    Ok(Settings {
        enabled: config.pet_desktop_enabled,
        character: config.pet_character,
        bar_style: config.pet_bar_style,
        bar_hidden: config.pet_bar_hidden,
        bubble: config.pet_bubble_enabled,
        sources: config.pet_sources,
        reduce_motion: config.reduce_motion,
        accent: config.accent,
    })
}

fn view(app: &AppHandle) -> Result<View, String> {
    let notifications = app
        .state::<State>()
        .notifications
        .lock()
        .map_err(|_| "通知服务繁忙")?
        .clone();
    Ok(View {
        settings: settings(app)?,
        notifications,
    })
}

fn emit_to_pet(app: &AppHandle, event: &str) {
    if let Some(window) = app.get_webview_window(LABEL) {
        let _ = window.emit(event, ());
    }
}

/// Creates the floating window hidden. It is built once so the first minimize
/// is instant; recreating a webview would show a blank frame.
pub fn ensure_window(app: &AppHandle) -> Result<WebviewWindow, String> {
    if let Some(window) = app.get_webview_window(LABEL) {
        return Ok(window);
    }
    let window =
        WebviewWindowBuilder::new(app, LABEL, WebviewUrl::App("index.html?window=pet".into()))
            .title("桌面宠物")
            .inner_size(WIDTH, HEIGHT)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .maximizable(false)
            .minimizable(false)
            .closable(false)
            .focused(false)
            .visible(false)
            .build()
            .map_err(|error| format!("无法创建桌面宠物窗口：{error}"))?;
    #[cfg(target_os = "macos")]
    {
        let _ = window.set_visible_on_all_workspaces(true);
    }
    Ok(window)
}

/// Puts the window at the saved position, or in the bottom-right corner of the
/// current monitor's work area the first time, and keeps it fully on screen.
fn place(app: &AppHandle, window: &WebviewWindow) -> Result<(), String> {
    let config = crate::config::load(&crate::config_path(app)?)?;
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| app.primary_monitor().ok().flatten());
    let Some(monitor) = monitor else {
        if let Some([x, y]) = config.pet_position {
            let _ = window.set_position(PhysicalPosition::new(x, y));
        }
        return Ok(());
    };
    let scale = monitor.scale_factor();
    let area = monitor.work_area();
    let size = window.outer_size().unwrap_or(PhysicalSize::new(
        (WIDTH * scale).round() as u32,
        (HEIGHT * scale).round() as u32,
    ));
    let margin = (MARGIN * scale).round() as i32;
    let (mut x, mut y) = match config.pet_position {
        Some([x, y]) => (x, y),
        None => (
            area.position.x + area.size.width as i32 - size.width as i32 - margin,
            area.position.y + area.size.height as i32 - size.height as i32 - margin,
        ),
    };
    let max_x = (area.position.x + area.size.width as i32 - size.width as i32).max(area.position.x);
    let max_y =
        (area.position.y + area.size.height as i32 - size.height as i32).max(area.position.y);
    x = x.clamp(area.position.x, max_x);
    y = y.clamp(area.position.y, max_y);
    window
        .set_position(PhysicalPosition::new(x, y))
        .map_err(|_| "无法定位桌面宠物")?;
    Ok(())
}

pub fn show(app: &AppHandle) -> Result<(), String> {
    let window = ensure_window(app)?;
    place(app, &window)?;
    window.show().map_err(|_| "无法显示桌面宠物")?;
    emit_to_pet(app, EVENT_SETTINGS);
    Ok(())
}

fn remember_position(app: &AppHandle, window: &WebviewWindow) {
    let state = app.state::<State>();
    let Ok(position) = window.outer_position() else {
        return;
    };
    let _ = update_config(app, |config| {
        config.pet_position = Some([position.x, position.y])
    });
    let _ = state;
}

pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window(LABEL) {
        remember_position(app, &window);
        let _ = window.hide();
    }
}

/// Called for the pet window's `Moved` events. The position is only staged;
/// the watcher flushes it once the drag settles, so a long drag does not write
/// the config on every frame and still saves the final spot.
pub fn remember_moved(app: &AppHandle, position: PhysicalPosition<i32>) {
    let state = app.state::<State>();
    let Ok(mut pending) = state.pending_position.lock() else {
        return;
    };
    *pending = Some(([position.x, position.y], Instant::now()));
}

fn flush_position(app: &AppHandle) {
    let state = app.state::<State>();
    let ready = {
        let Ok(mut pending) = state.pending_position.lock() else {
            return;
        };
        match *pending {
            Some((position, at)) if at.elapsed() > Duration::from_millis(700) => {
                *pending = None;
                Some(position)
            }
            _ => None,
        }
    };
    if let Some(position) = ready {
        let _ = update_config(app, |config| config.pet_position = Some(position));
    }
}

pub fn restore_main(app: &AppHandle) {
    app.state::<State>().preview.store(false, Ordering::Relaxed);
    hide(app);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.unminimize();
        let _ = main.show();
        let _ = main.set_focus();
    }
}

pub fn open_settings(app: &AppHandle) {
    restore_main(app);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit(EVENT_OPEN_SETTINGS, ());
    }
}

/// Adds one notification and wakes the pet bar. `key` deduplicates updates for
/// the same event; without it every call creates a new entry.
pub fn notify(app: &AppHandle, source: &str, text: &str, key: Option<String>) {
    let Ok(config) = crate::config_path(app).and_then(|path| crate::config::load(&path)) else {
        return;
    };
    if !config.pet_desktop_enabled
        || !config.pet_sources.iter().any(|entry| entry == source)
        || !["campus", "agent", "class", "update"].contains(&source)
    {
        return;
    }
    let text: String = text.chars().take(MAX_TEXT).collect();
    if text.trim().is_empty() {
        return;
    }
    let state = app.state::<State>();
    let Ok(mut list) = state.notifications.lock() else {
        return;
    };
    let id = match key {
        Some(key) if !key.is_empty() => {
            if list.iter().any(|entry| entry.id == key) {
                return;
            }
            key
        }
        _ => format!("{source}-{}", state.next_id.fetch_add(1, Ordering::Relaxed)),
    };
    list.push(Notification {
        id,
        source: source.to_string(),
        text,
    });
    if list.len() > MAX_NOTIFICATIONS {
        let drop = list.len() - MAX_NOTIFICATIONS;
        list.drain(0..drop);
    }
    drop(list);
    emit_to_pet(app, EVENT_NOTIFICATIONS);
}

fn enabled_source(app: &AppHandle, source: &str) -> bool {
    crate::config_path(app)
        .and_then(|path| crate::config::load(&path))
        .map(|config| {
            config.pet_desktop_enabled && config.pet_sources.iter().any(|entry| entry == source)
        })
        .unwrap_or(false)
}

fn seen_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|_| "无法定位应用数据目录")?
        .join("pet-seen.json"))
}

fn load_seen(app: &AppHandle) -> Option<Vec<String>> {
    let text = std::fs::read_to_string(seen_path(app).ok()?).ok()?;
    serde_json::from_str::<Vec<String>>(&text).ok()
}

fn save_seen(app: &AppHandle, seen: &[String]) {
    if let Ok(path) = seen_path(app) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(bytes) = serde_json::to_vec(seen) {
            let _ = crate::config::write_atomic(&path, &bytes);
        }
    }
}

async fn poll_campus(app: &AppHandle) {
    let Some(channels) = crate::school::notices::channels()
        .get("channels")
        .and_then(|value| value.as_array())
        .cloned()
    else {
        return;
    };
    let stored = load_seen(app);
    let first_run = stored.is_none();
    let mut order = stored.unwrap_or_default();
    let mut seen: HashSet<String> = order.iter().cloned().collect();
    let mut fetched = 0;
    for channel in channels {
        let Some(id) = channel.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        let Ok(page) = crate::school::notices::list(id, 3).await else {
            continue;
        };
        fetched += 1;
        let name = page
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("校园通知");
        let Some(items) = page.get("items").and_then(|value| value.as_array()) else {
            continue;
        };
        for item in items {
            let link = item
                .get("url")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let title = item
                .get("title")
                .and_then(|value| value.as_str())
                .unwrap_or("");
            if link.is_empty() || title.is_empty() || !seen.insert(link.to_string()) {
                continue;
            }
            order.push(link.to_string());
            if !first_run {
                notify(
                    app,
                    "campus",
                    &format!("{name} · {title}"),
                    Some(format!("campus:{link}")),
                );
            }
        }
    }
    // If every channel failed, leave the seen file alone: saving now would make
    // the next successful poll treat every existing notice as new.
    if fetched == 0 {
        return;
    }
    if order.len() > MAX_SEEN {
        let drop = order.len() - MAX_SEEN;
        order.drain(0..drop);
    }
    save_seen(app, &order);
}

fn watch(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut visible = false;
        loop {
            tokio::time::sleep(Duration::from_millis(250)).await;
            let Some(main) = app.get_webview_window("main") else {
                break;
            };
            let minimized = main.is_minimized().unwrap_or(false);
            flush_position(&app);
            let state = app.state::<State>();
            let enabled = state.enabled.load(Ordering::Relaxed);
            let preview = state.preview.load(Ordering::Relaxed);
            let want = preview || (minimized && enabled);
            if want && !visible {
                if show(&app).is_ok() {
                    visible = true;
                }
            } else if !want && visible {
                hide(&app);
                visible = false;
            }
        }
    });
}

fn poll_loop(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(12)).await;
        loop {
            if enabled_source(&app, "campus") {
                poll_campus(&app).await;
            }
            tokio::time::sleep(CAMPUS_INTERVAL).await;
        }
    });
}

/// Called once from `setup`: creates the hidden window and starts the two
/// background loops. The pet is inert until the user enables it in settings.
pub fn initialize(app: &AppHandle) -> Result<(), String> {
    let config = crate::config::load(&crate::config_path(app)?)?;
    let state = app.state::<State>();
    state
        .enabled
        .store(config.pet_desktop_enabled, Ordering::Relaxed);
    if config.pet_desktop_enabled {
        ensure_window(app)?;
    }
    watch(app.clone());
    poll_loop(app.clone());
    Ok(())
}

/// Mirrors config changes made from the main window, so the watcher and the
/// floating window see them immediately.
pub fn sync_config(app: &AppHandle, config: &crate::config::AppConfig) {
    app.state::<State>()
        .enabled
        .store(config.pet_desktop_enabled, Ordering::Relaxed);
    if config.pet_desktop_enabled {
        let _ = ensure_window(app);
    }
    emit_to_pet(app, EVENT_SETTINGS);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit(EVENT_CONFIG_CHANGED, config.clone());
    }
}

#[tauri::command]
pub fn pet_view(app: AppHandle, window: WebviewWindow) -> Result<View, String> {
    pet_only(&window)?;
    view(&app)
}

#[tauri::command]
pub fn pet_restore(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    pet_only(&window)?;
    restore_main(&app);
    Ok(())
}

#[tauri::command]
pub fn pet_open_settings(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    pet_only(&window)?;
    open_settings(&app);
    Ok(())
}

#[tauri::command]
pub fn pet_set_bar_hidden(
    app: AppHandle,
    window: WebviewWindow,
    hidden: bool,
) -> Result<View, String> {
    pet_only(&window)?;
    let config = update_config(&app, |config| config.pet_bar_hidden = hidden)?;
    emit_to_pet(&app, EVENT_SETTINGS);
    if let Some(main) = app.get_webview_window("main") {
        let _ = main.emit(EVENT_CONFIG_CHANGED, config);
    }
    view(&app)
}

/// Pushes a notification from the main window (for events it already knows,
/// such as plugin updates). Agent and app-update notifications come from Rust.
#[tauri::command]
pub fn pet_push_notification(
    app: AppHandle,
    window: WebviewWindow,
    source: String,
    text: String,
    key: Option<String>,
) -> Result<(), String> {
    crate::main_only(&window)?;
    if text.len() > 1000 || key.as_ref().is_some_and(|value| value.len() > 256) {
        return Err("通知内容过长".into());
    }
    notify(&app, &source, &text, key);
    Ok(())
}

/// Shows the pet from the settings page without minimizing the main window.
#[tauri::command]
pub fn pet_preview(app: AppHandle, window: WebviewWindow) -> Result<(), String> {
    crate::main_only(&window)?;
    app.state::<State>().preview.store(true, Ordering::Relaxed);
    show(&app)
}
