//! Tauri commands for the school domain.

use super::session::renew_lms_session;
use super::workbench::{load_sync, run_workbench, sync_state_path};
use crate::*;
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use tauri::{Emitter, WebviewWindow};

#[tauri::command]
pub(crate) fn school_catalog(window: WebviewWindow) -> Result<Vec<school::Feature>, String> {
    main_only(&window)?;
    Ok(school::catalog())
}

pub(crate) fn require_school_feature(app: &tauri::AppHandle, feature: &str) -> Result<(), String> {
    if config::load(&config_path(app)?)?
        .disabled_plugins
        .contains(&format!("cn.xjtu.{feature}"))
    {
        return Err("请先启用此校园插件".into());
    }
    Ok(())
}

pub(crate) async fn school_account(
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

pub(crate) async fn save_school_session(
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

/// Shared by the school_query command and the Agent's campus tools.
pub(crate) async fn run_school_query(
    app: &tauri::AppHandle,
    query: school::Query,
    cache_key: Option<&str>,
    force: bool,
) -> Result<serde_json::Value, String> {
    require_school_feature(app, query.feature())?;
    query.validate()?;
    let state = app.state::<AppState>();
    let _service_guard = state.school_lock.lock().await;
    if matches!(
        query,
        school::Query::Calendar
            | school::Query::LibraryFloors
            | school::Query::NoticeChannels
            | school::Query::Notices { .. }
    ) {
        let scope = cache::active_scope(app);
        if !force {
            if let (Some(key), Some(ttl)) = (cache_key, query.cache_ttl()) {
                if let Some(data) =
                    cache::read_fresh(app, &scope, key, std::time::Duration::from_secs(ttl))
                {
                    return Ok(data);
                }
            }
        }
        let data = school::query(None, query).await.map(|(data, _)| data)?;
        if let Some(key) = cache_key {
            // Public data still goes under the current account so reads line up.
            cache::write_for(app, &scope, key, &data);
        }
        return Ok(data);
    }
    let (mut account, mut generation) = school_account(app, &state).await?;
    let account_id = account.id.clone();
    if !force {
        if let (Some(key), Some(ttl)) = (cache_key, query.cache_ttl()) {
            if let Some(data) =
                cache::read_fresh(app, &account_id, key, std::time::Duration::from_secs(ttl))
            {
                return Ok(data);
            }
        }
    }
    let mut outcome = school::query(Some(account.clone()), query.clone()).await;
    if outcome
        .as_ref()
        .err()
        .is_some_and(|error| is_lms_login_error(error))
    {
        // The LMS session can expire independently of the timetable session,
        // so renew it here instead of failing with a misleading parse error.
        renew_lms_session(app, &state).await?;
        let (next, next_generation) = school_account(app, &state).await?;
        if next.id != account_id {
            return Err("校园账号已切换，请重新查询".into());
        }
        account = next;
        generation = next_generation;
        outcome = school::query(Some(account), query).await;
    }
    let (data, cookies) = outcome?;
    if let Some(cookies) = cookies {
        save_school_session(app, &state, &account_id, generation, cookies).await?;
    }
    if let Some(key) = cache_key {
        // Under the account this query ran for, even if the active one changed.
        cache::write_for(app, &account_id, key, &data);
    }
    Ok(data)
}

/// Homework submission is the one Agent write path. `confirm = false` only
/// previews; files are uploaded and the submission posted only when true.
pub(crate) async fn run_lms_homework(
    app: &tauri::AppHandle,
    request: school::lms::Homework,
    confirm: bool,
) -> Result<serde_json::Value, String> {
    run_lms_homework_checked(app, request, confirm, None).await
}

/// True for the two ways the LMS can lack a usable session: the saved cookie
/// expired, or no LMS site session was saved at all.
pub(crate) fn is_lms_login_error(error: &str) -> bool {
    error == school::lms::LOGIN_EXPIRED || error == school::lms::LOGIN_REQUIRED
}

pub(crate) async fn lms_homework_once(
    client: &school::lms::Client,
    request: &school::lms::Homework,
    confirm: bool,
) -> Result<serde_json::Value, String> {
    if confirm {
        client.homework_submit(request).await
    } else {
        client.homework_preview(request).await
    }
}

/// `expected_account` guards file uploads resolved under one campus account:
/// if the active account changed in the meantime, the request is refused
/// instead of sending the previous account's files through the new session.
pub(crate) async fn run_lms_homework_checked(
    app: &tauri::AppHandle,
    request: school::lms::Homework,
    confirm: bool,
    expected_account: Option<&str>,
) -> Result<serde_json::Value, String> {
    require_school_feature(app, "lms")?;
    request.validate()?;
    let state = app.state::<AppState>();
    let _service_guard = state.school_lock.lock().await;
    let (mut account, _) = school_account(app, &state).await?;
    if expected_account.is_some_and(|expected| account.id != expected) {
        return Err("校园账号已切换，请重新选择文件".into());
    }
    let client = match school::lms::Client::connect(&account) {
        Ok(client) => client,
        Err(error) if is_lms_login_error(&error) => {
            renew_lms_session(app, &state).await?;
            let (next, _) = school_account(app, &state).await?;
            if expected_account.is_some_and(|expected| next.id != expected) {
                return Err("校园账号已切换，请重新选择文件".into());
            }
            account = next;
            school::lms::Client::connect(&account)?
        }
        Err(error) => return Err(error),
    };
    let mut data = match lms_homework_once(&client, &request, confirm).await {
        Err(error) if is_lms_login_error(&error) => {
            // Expiry is detected before any file is uploaded, so renewing and
            // running the preview/submit again cannot duplicate a submission.
            renew_lms_session(app, &state).await?;
            let (next, _) = school_account(app, &state).await?;
            if expected_account.is_some_and(|expected| next.id != expected) {
                return Err("校园账号已切换，请重新选择文件".into());
            }
            let client = school::lms::Client::connect(&next)?;
            account = next;
            lms_homework_once(&client, &request, confirm).await?
        }
        other => other?,
    };
    // The school call is over; release the shared school lock before local
    // copying so the rest of the app is not blocked behind a large file.
    drop(_service_guard);
    // Only a final submission archives. Drafts, previews and failures keep the
    // workbench untouched, and a local archive problem never changes the
    // submission result or makes the caller retry.
    if confirm && !request.draft {
        let archive = archive_homework(app, &account, &request, &data).await;
        if let Some(map) = data.as_object_mut() {
            map.insert("archive".into(), archive);
        }
    }
    Ok(data)
}

/// Copy the submitted files into the synced course folder and bind them to the
/// assignment. This runs after the school accepted the submission, so every
/// failure is a warning in the returned value, never an error.
pub(crate) async fn archive_homework(
    app: &tauri::AppHandle,
    account: &accounts::Account,
    request: &school::lms::Homework,
    result: &serde_json::Value,
) -> serde_json::Value {
    if request.files.is_empty() {
        return serde_json::json!({ "skipped": "没有附件需要归档" });
    }
    let enabled = config::load(&config_path(app).unwrap_or_default())
        .map(|config| config.workbench_auto_archive)
        .unwrap_or(true);
    if !enabled {
        return serde_json::json!({ "skipped": "自动归档已关闭" });
    }
    let workspace = match storage::file_path(app, "workspace") {
        Ok(workspace) => workspace,
        Err(error) => return serde_json::json!({ "warnings": [error] }),
    };
    let root = workbench::root(&workspace, &account.id);
    let course_id = result.get("courseId").and_then(serde_json::Value::as_u64);
    let folder = match archive_folder(app, account, root.clone(), course_id).await {
        Ok(Some(folder)) => folder,
        Ok(None) => {
            return serde_json::json!({
                "warnings": ["没有在思源学堂找到这门课程，文件没有归档；先在工作台同步本学期的课程后再试"],
            })
        }
        Err(error) => return serde_json::json!({ "warnings": [error] }),
    };
    let title = result
        .get("title")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let activity = request.activity;
    let files = request.files.clone();
    let lock = app.state::<AppState>().library_lock.clone();
    let target = folder.clone();
    let assignment = title.clone();
    let report = run_workbench(move || {
        Ok(workbench::archive_files(
            &root,
            &target,
            &files,
            activity,
            &assignment,
            &lock,
        ))
    })
    .await;
    match report {
        Ok(report) => serde_json::json!({
            "folder": folder,
            "assignment": title,
            "files": report.files,
            "warnings": report.warnings,
        }),
        Err(error) => serde_json::json!({ "warnings": [error] }),
    }
}

/// The current course folder for an LMS course. The sync state answers this
/// without a network call; a course that was never synced is looked up in the
/// LMS and its folder is created once.
pub(crate) async fn archive_folder(
    app: &tauri::AppHandle,
    account: &accounts::Account,
    root: PathBuf,
    course_id: Option<u64>,
) -> Result<Option<String>, String> {
    let Some(course_id) = course_id else {
        return Ok(None);
    };
    let path = sync_state_path(app, &account.id)?;
    if let Ok(state) = load_sync(&path) {
        if let Some(course) = state.courses.iter().find(|course| course.id == course_id) {
            let meta = workbench::CourseMeta {
                id: course.id,
                code: course.code.clone(),
                subject_code: course.subject_code.clone(),
                name: course.name.clone(),
                term: state.term.clone(),
            };
            let root = root.clone();
            return run_workbench(move || {
                workbench::ensure_course_folder(&root, &state.term, &meta)
            })
            .await
            .map(Some);
        }
    }
    // Not synced yet: ask the LMS for the course list (one request).
    let client = school::lms::Client::connect(account)?;
    let value = client.courses().await?;
    let course = value["courses"]
        .as_array()
        .and_then(|courses| {
            courses
                .iter()
                .find(|course| course["id"].as_u64() == Some(course_id))
        })
        .cloned();
    let Some(course) = course else {
        return Ok(None);
    };
    let term = course["termLabel"]
        .as_str()
        .unwrap_or_default()
        .trim()
        .to_string();
    if term.is_empty() {
        return Ok(None);
    }
    let meta = workbench::CourseMeta {
        id: course_id,
        code: course["code"].as_str().unwrap_or_default().to_string(),
        subject_code: course["subjectCode"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        name: course["name"].as_str().unwrap_or_default().to_string(),
        term: term.clone(),
    };
    let root = root.clone();
    run_workbench(move || workbench::ensure_course_folder(&root, &term, &meta))
        .await
        .map(Some)
}

/// Agent-facing 课程资料 download. The file lands in the visible download
/// folder under a sanitized, unique name; the model cannot choose a path. The
/// body comes from the public media host, so the school lock is not held while
/// streaming.
pub(crate) async fn download_lms_file(
    app: &tauri::AppHandle,
    upload_id: u64,
    name: String,
) -> Result<serde_json::Value, String> {
    require_school_feature(app, "lms")?;
    let name = name.trim();
    if name.is_empty() {
        return Err("请提供附件的文件名".into());
    }
    let state = app.state::<AppState>();
    let (account, _) = school_account(app, &state).await?;
    let path = storage::unique_download_target(app, name)?;
    let size = school::lms::Client::connect(&account)?
        .save_file(upload_id, &path)
        .await?;
    let display = path
        .to_str()
        .map(|value| storage::display_for(app, value))
        .unwrap_or_default();
    Ok(serde_json::json!({
        "path": display,
        "name": path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
        "size": size,
    }))
}

/// Build an evaluation preview for the Agent. It queries the school but never
/// submits anything; the returned `id` can be submitted later.
pub(crate) async fn prepare_evaluations(
    app: &tauri::AppHandle,
    request: school::evaluation::Prepare,
) -> Result<serde_json::Value, String> {
    require_school_feature(app, "evaluations")?;
    let state = app.state::<AppState>();
    let _service_guard = state.school_lock.lock().await;
    let (account, generation) = school_account(app, &state).await?;
    let account_id = account.id.clone();
    let (preview, cookies) = state
        .evaluations
        .prepare(account, generation, request)
        .await?;
    save_school_session(app, &state, &account_id, generation, cookies).await?;
    serde_json::to_value(preview).map_err(|_| "无法序列化评教预览".into())
}

/// Submit a preview that the Agent already showed to the user. The caller must
/// only reach this after an explicit confirmation.
pub(crate) async fn submit_evaluations(
    app: &tauri::AppHandle,
    preview_id: String,
) -> Result<serde_json::Value, String> {
    require_school_feature(app, "evaluations")?;
    let state = app.state::<AppState>();
    let _service_guard = state.school_lock.lock().await;
    let _account_guard = state.auth_lock.lock().await;
    let path = accounts_path(app)?;
    let mut book = accounts::load(&path)?;
    let account = book.active().ok_or("请先登录校园账号")?.clone();
    let account_id = account.id.clone();
    let generation = state.generation.load(Ordering::SeqCst);
    let (outcomes, cookies) = state
        .evaluations
        .submit(&preview_id, account, generation, |_| {})
        .await?;
    // A local save failure must not turn an accepted submission into a retry.
    if let Some(cookies) = cookies {
        if let Some(account) = book.accounts.iter_mut().find(|a| a.id == account_id) {
            account.cookies = cookies;
        }
        let _ = accounts::save(&path, &book);
    }
    serde_json::to_value(serde_json::json!({"outcomes": outcomes}))
        .map_err(|_| "无法序列化评教结果".into())
}

#[tauri::command]
pub(crate) async fn school_query(
    app: tauri::AppHandle,
    window: WebviewWindow,
    query: school::Query,
    cache_key: Option<String>,
    force: bool,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    run_school_query(&app, query, cache_key.as_deref(), force).await
}

#[tauri::command]
pub(crate) async fn school_prepare_evaluations(
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
pub(crate) async fn school_submit_evaluations(
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
pub(crate) fn school_cancel_evaluations(
    window: WebviewWindow,
    state: tauri::State<AppState>,
    preview_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    state.evaluations.cancel(&preview_id)
}
