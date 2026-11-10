//! Tauri commands for the timetable domain.

use crate::*;
use std::sync::atomic::Ordering;
use tauri::WebviewWindow;

pub(crate) fn timetable_enabled(app: &tauri::AppHandle) -> Result<(), String> {
    if config::load(&config_path(app)?)?
        .disabled_plugins
        .iter()
        .any(|id| id == plugins::TIMETABLE_ID)
    {
        return Err("请先启用课表插件".into());
    }
    Ok(())
}

/// Shared by the timetable command and the Agent's campus tools.
pub(crate) async fn run_timetable(app: &tauri::AppHandle) -> Result<campus::Timetable, String> {
    timetable_enabled(app)?;
    let state = app.state::<AppState>();
    let generation = state.generation.load(Ordering::SeqCst);
    let (session, account_id) = {
        let _guard = state.auth_lock.lock().await;
        let path = accounts_path(app)?;
        let book = accounts::load(&path)?;
        if let Some(account) = book.active() {
            (
                vault::Session {
                    degree: account.degree.clone(),
                    token: account.token.clone(),
                    mode: account.mode,
                    cookies: Some(account.cookies.clone()),
                },
                Some(account.id.clone()),
            )
        } else if !path.exists() {
            (vault::session()?.ok_or("请先连接校园账户")?, None)
        } else {
            return Err("请先连接校园账户".into());
        }
    };
    let mut timetable = campus::timetable(&session).await?;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账户状态已变化，请重新查询".into());
    }
    let scope = account_id.unwrap_or_else(|| cache::active_scope(app));
    // The cache keeps the school's own values, so a reset can always restore
    // them; local edits are applied to the copy that leaves this function.
    if let Ok(value) = serde_json::to_value(&timetable) {
        cache::write_for(app, &scope, "timetable", &value);
    }
    let plan = timetable_custom::plan(app, &scope, &timetable.semester)?;
    timetable_custom::apply(&mut timetable, &plan);
    Ok(timetable)
}

/// Last synced school timetable for this account, without touching the network.
pub(crate) fn cached_timetable(
    app: &tauri::AppHandle,
    semester: &str,
) -> Result<campus::Timetable, String> {
    let value = cache::read(app, "timetable").ok_or("还没有可用的课表，请先打开课表页同步一次")?;
    let timetable: campus::Timetable =
        serde_json::from_value(value).map_err(|_| "课表缓存无法读取，请重新同步课表")?;
    let semester = semester.trim();
    if !semester.is_empty() && timetable.semester != semester {
        return Err("请先同步这个学期的课表，再修改它的课程".into());
    }
    Ok(timetable)
}

/// Applies local edits to a stored raw timetable for the first paint. A value
/// that does not parse is handed back untouched; the background refresh then
/// replaces it with a real one.
pub(crate) fn merge_cached_timetable(
    app: &tauri::AppHandle,
    value: serde_json::Value,
) -> serde_json::Value {
    let Ok(mut timetable) = serde_json::from_value::<campus::Timetable>(value.clone()) else {
        return value;
    };
    let state = app.state::<AppState>();
    let Ok(_guard) = state.timetable_lock.lock() else {
        return value;
    };
    let scope = cache::active_scope(app);
    match timetable_custom::plan(app, &scope, &timetable.semester) {
        Ok(plan) => {
            timetable_custom::apply(&mut timetable, &plan);
            serde_json::to_value(&timetable).unwrap_or(value)
        }
        // Structured data that fails to load is not worth blocking the page for.
        Err(_) => value,
    }
}

/// The semester an edit targets: the one asked for, or the last synced one.
pub(crate) fn target_semester(
    app: &tauri::AppHandle,
    requested: Option<&str>,
) -> Result<String, String> {
    match requested.map(str::trim).filter(|value| !value.is_empty()) {
        Some(value) => timetable_custom::clean_semester(value),
        None => Ok(cached_timetable(app, "")?.semester),
    }
}

/// Lists local edits for the Agent. The raw timetable supplies the original
/// labels of edited school courses.
pub(crate) fn timetable_customizations(
    app: &tauri::AppHandle,
    semester: Option<String>,
) -> Result<serde_json::Value, String> {
    timetable_enabled(app)?;
    let state = app.state::<AppState>();
    let _guard = state.timetable_lock.lock().map_err(|_| "课表状态不可用")?;
    let semester = target_semester(app, semester.as_deref())?;
    let raw = cached_timetable(app, &semester)?;
    let plan = timetable_custom::plan(app, &cache::active_scope(app), &semester)?;
    Ok(timetable_custom::describe(&raw, &plan))
}

/// One Agent edit. `confirm=false` only reports what would change; the user
/// must have seen that preview before anything is written.
pub(crate) fn apply_timetable_edit(
    app: &tauri::AppHandle,
    request: timetable_custom::Request,
    confirm: bool,
) -> Result<serde_json::Value, String> {
    timetable_enabled(app)?;
    let state = app.state::<AppState>();
    let _guard = state.timetable_lock.lock().map_err(|_| "课表状态不可用")?;
    let request = timetable_custom::Request {
        semester: target_semester(app, Some(&request.semester))?,
        ..request
    };
    let raw = cached_timetable(app, &request.semester)?;
    let scope = cache::active_scope(app);
    if !confirm {
        let plan = timetable_custom::plan(app, &scope, &request.semester)?;
        return timetable_custom::preview(&raw, &plan, &request);
    }
    let plan = timetable_custom::edit(app, &scope, &request)?;
    let mut merged = raw;
    timetable_custom::apply(&mut merged, &plan);
    let course = merged
        .courses
        .iter()
        .find(|course| course.id == request.id)
        .or_else(|| merged.courses.iter().rev().find(|course| course.custom))
        .map(timetable_custom::course_summary);
    Ok(serde_json::json!({
        "applied": true,
        "semester": merged.semester,
        "course": course,
        "courseCount": merged.courses.len(),
        "hiddenCount": merged.hidden.len(),
    }))
}

/// Page command: applies one edit and returns the merged timetable the page
/// should show, using the cached copy instead of another network round trip.
#[tauri::command]
pub(crate) fn edit_timetable(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    request: timetable_custom::Request,
) -> Result<campus::Timetable, String> {
    main_only(&window)?;
    timetable_enabled(&app)?;
    let _guard = state.timetable_lock.lock().map_err(|_| "课表状态不可用")?;
    let request = timetable_custom::Request {
        semester: target_semester(&app, Some(&request.semester))?,
        ..request
    };
    let raw = cached_timetable(&app, &request.semester)?;
    let plan = timetable_custom::edit(&app, &cache::active_scope(&app), &request)?;
    let mut merged = raw;
    timetable_custom::apply(&mut merged, &plan);
    Ok(merged)
}

#[tauri::command]
pub(crate) async fn query_timetable(
    app: tauri::AppHandle,
    window: WebviewWindow,
    plugin_id: String,
) -> Result<campus::Timetable, String> {
    main_only(&window)?;
    if plugin_id != plugins::TIMETABLE_ID {
        return Err("此插件没有课表服务权限".into());
    }
    run_timetable(&app).await
}
