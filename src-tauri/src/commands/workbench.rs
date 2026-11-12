//! Tauri commands for the workbench domain.

use super::school::{require_school_feature, run_lms_homework_checked, school_account};
use crate::*;
use std::path::PathBuf;
use tauri::WebviewWindow;

pub(crate) async fn workbench_root(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(PathBuf, String), String> {
    let (account, _) = school_account(app, state).await?;
    let workspace = storage::file_path(app, "workspace")?;
    Ok((workbench::root(&workspace, &account.id), account.id))
}

pub(crate) fn sync_state_path(app: &tauri::AppHandle, account_id: &str) -> Result<PathBuf, String> {
    storage::path(
        app,
        &format!("state/workbench/{}.json", workbench::owner_key(account_id)),
    )
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncCourse {
    pub(crate) id: u64,
    pub(crate) code: String,
    pub(crate) subject_code: String,
    pub(crate) name: String,
    pub(crate) folder: String,
    pub(crate) assignments: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncState {
    pub(crate) version: u32,
    pub(crate) term: String,
    pub(crate) synced_at: i64,
    pub(crate) courses: Vec<SyncCourse>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            version: 1,
            term: String::new(),
            synced_at: 0,
            courses: Vec::new(),
        }
    }
}

pub(crate) fn load_sync(path: &PathBuf) -> Result<SyncState, String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(SyncState::default())
        }
        Err(_) => return Err("无法读取工作台同步记录".into()),
    };
    let state: SyncState = serde_json::from_slice(&bytes).map_err(|_| "工作台同步记录损坏")?;
    if state.version != 1 {
        return Err("工作台同步记录版本不受支持".into());
    }
    Ok(state)
}

pub(crate) fn save_sync(path: &PathBuf, state: &SyncState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|_| "无法创建工作台同步目录")?;
    }
    let bytes = serde_json::to_vec_pretty(state).map_err(|_| "无法编码工作台同步记录")?;
    let temp = path.with_extension("json.tmp");
    std::fs::write(&temp, bytes).map_err(|_| "无法写入工作台同步记录")?;
    std::fs::rename(&temp, path).map_err(|_| "无法写入工作台同步记录")?;
    Ok(())
}

pub(crate) fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

/// File work (copying, index writes, folder scans) must not hold an async worker.
pub(crate) async fn run_workbench<T: Send + 'static>(
    task: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(task)
        .await
        .map_err(|_| "工作台任务失败".to_string())?
}

#[tauri::command]
pub(crate) async fn workbench_overview(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    let (root, account_id) = workbench_root(&app, &state).await?;
    let store = load_sync(&sync_state_path(&app, &account_id)?)?;
    let folders = run_workbench(move || workbench::tree(&root)).await?;
    Ok(serde_json::json!({
        "currentTerm": store.term,
        "syncedAt": store.synced_at,
        "folders": folders,
        "courses": store.courses,
    }))
}

#[tauri::command]
pub(crate) async fn workbench_folder(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    folder: String,
) -> Result<Vec<workbench::FileNode>, String> {
    main_only(&window)?;
    let (root, _) = workbench_root(&app, &state).await?;
    run_workbench(move || workbench::files(&root, &folder)).await
}

#[tauri::command]
pub(crate) async fn workbench_create_folder(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    parent: String,
    name: String,
) -> Result<workbench::FolderNode, String> {
    main_only(&window)?;
    let (root, _) = workbench_root(&app, &state).await?;
    run_workbench(move || workbench::create_folder(&root, &parent, &name)).await
}

#[tauri::command]
pub(crate) async fn workbench_import(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    folder: String,
    paths: Vec<String>,
) -> Result<Vec<workbench::ImportOutcome>, String> {
    main_only(&window)?;
    if paths.len() > 32 {
        return Err("一次最多添加 32 个文件".into());
    }
    if paths.is_empty() {
        return Ok(Vec::new());
    }
    let (root, _) = workbench_root(&app, &state).await?;
    let lock = state.library_lock.clone();
    run_workbench(move || Ok(workbench::import_files(&root, &folder, &paths, &lock))).await
}

#[tauri::command]
pub(crate) async fn workbench_update(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    path: String,
    input: workbench::BindingInput,
) -> Result<workbench::FileNode, String> {
    main_only(&window)?;
    let (root, _) = workbench_root(&app, &state).await?;
    let lock = state.library_lock.clone();
    run_workbench(move || workbench::set_binding(&root, &path, input, &lock)).await
}

#[tauri::command]
pub(crate) async fn workbench_delete(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    main_only(&window)?;
    let (root, _) = workbench_root(&app, &state).await?;
    let lock = state.library_lock.clone();
    run_workbench(move || workbench::delete_file(&root, &path, &lock)).await
}

#[tauri::command]
pub(crate) async fn workbench_open(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    main_only(&window)?;
    let (root, _) = workbench_root(&app, &state).await?;
    let path = run_workbench(move || workbench::absolute(&root, &path)).await?;
    let path = path.to_str().ok_or("文件路径不是有效的 UTF-8")?;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|_| "无法打开本地文件".into())
}

#[tauri::command]
pub(crate) async fn workbench_reveal(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    main_only(&window)?;
    let (root, _) = workbench_root(&app, &state).await?;
    let path = run_workbench(move || {
        workbench::absolute(&root, &path).or_else(|_| workbench::folder_absolute(&root, &path))
    })
    .await?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|_| "无法在文件管理器中显示".into())
}

/// One 工作台 submission, resolved from workspace paths. The active campus
/// account is checked again just before the school call.
pub(crate) async fn workbench_submit_inner(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    state: &tauri::State<'_, AppState>,
    activity: u64,
    comment: String,
    paths: Vec<String>,
    confirm: bool,
) -> Result<serde_json::Value, String> {
    main_only(window)?;
    if paths.len() > school::lms::MAX_UPLOAD_FILES {
        return Err(format!(
            "一次最多上传 {} 个文件",
            school::lms::MAX_UPLOAD_FILES
        ));
    }
    let (account, _) = school_account(app, state).await?;
    let workspace = storage::file_path(app, "workspace")?;
    let root = workbench::root(&workspace, &account.id);
    let files = run_workbench(move || {
        paths
            .iter()
            .map(|path| -> Result<String, String> {
                let path = workbench::absolute(&root, path)?;
                path.to_str()
                    .map(str::to_string)
                    .ok_or_else(|| "文件路径不是有效的 UTF-8".to_string())
            })
            .collect::<Result<Vec<_>, _>>()
    })
    .await?;
    let request = school::lms::Homework {
        activity,
        comment,
        files,
        draft: false,
    };
    run_lms_homework_checked(app, request, confirm, Some(&account.id)).await
}

/// Preview only: reads the activity and the local files, never posts.
#[tauri::command]
pub(crate) async fn workbench_submit_preview(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    activity: u64,
    comment: String,
    paths: Vec<String>,
) -> Result<serde_json::Value, String> {
    workbench_submit_inner(&app, &window, &state, activity, comment, paths, false).await
}

/// The caller must have shown the preview and asked the user first.
#[tauri::command]
pub(crate) async fn workbench_submit(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    activity: u64,
    comment: String,
    paths: Vec<String>,
) -> Result<serde_json::Value, String> {
    workbench_submit_inner(&app, &window, &state, activity, comment, paths, true).await
}

/// Arguments for the Agent's read-only workbench tool.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkbenchRead {
    pub action: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub query: String,
}

/// Arguments for the Agent's workbench write tool. Every action previews
/// first; `confirm = true` is the only way anything is copied or deleted.
#[derive(Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct WorkbenchEdit {
    pub action: String,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub paths: Vec<String>,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub assignment: String,
    #[serde(default)]
    pub activity_id: Option<u64>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub confirm: bool,
}

/// The workbench is the user's own folder tree, so the Agent may browse all of
/// it: folder metadata, files in one folder, and a text search across both.
pub(crate) async fn run_workbench_read(
    app: &tauri::AppHandle,
    request: WorkbenchRead,
) -> Result<serde_json::Value, String> {
    let state = app.state::<AppState>();
    let (root, account_id) = workbench_root(app, &state).await?;
    match request.action.as_str() {
        "overview" => {
            let store = load_sync(&sync_state_path(app, &account_id)?)?;
            let folders = run_workbench(move || workbench::tree(&root)).await?;
            Ok(serde_json::json!({
                "currentTerm": store.term,
                "syncedAt": store.synced_at,
                "folders": folders,
                "courses": store.courses,
            }))
        }
        "folder" => {
            let folder = request.folder;
            let target = folder.clone();
            run_workbench(move || workbench::files(&root, &target))
                .await
                .map(|files| serde_json::json!({ "folder": folder, "files": files }))
        }
        "search" => {
            let query = request.query;
            let needle = query.clone();
            run_workbench(move || workbench::search(&root, &needle, 100))
                .await
                .map(|files| serde_json::json!({ "query": query, "files": files }))
        }
        other => Err(format!(
            "workbench_files 不支持 action={other}，可选 overview、folder、search"
        )),
    }
}

pub(crate) async fn run_workbench_edit(
    app: &tauri::AppHandle,
    request: WorkbenchEdit,
) -> Result<serde_json::Value, String> {
    let state = app.state::<AppState>();
    let (root, _) = workbench_root(app, &state).await?;
    let lock = state.library_lock.clone();
    let confirm = request.confirm;
    match request.action.as_str() {
        "mkdir" => {
            let parent = request.folder;
            let name = request.name;
            if !confirm {
                workbench::path_of(&root, &parent)?;
                workbench::validate_name(&name)?;
                return Ok(serde_json::json!({
                    "confirmRequired": true,
                    "action": "mkdir",
                    "parent": parent,
                    "name": name,
                    "message": "还没有创建文件夹。向用户说明位置和名称；用户明确同意后再带 confirm=true 调用。",
                }));
            }
            let created =
                run_workbench(move || workbench::create_folder(&root, &parent, &name)).await?;
            Ok(serde_json::json!({ "done": true, "action": "mkdir", "folder": created }))
        }
        "import" => {
            if request.paths.is_empty() {
                return Err("请提供要添加的文件".into());
            }
            if request.paths.len() > 32 {
                return Err("一次最多添加 32 个文件".into());
            }
            let folder = request.folder;
            let paths = request.paths;
            if !confirm {
                let folder_copy = folder.clone();
                let preview =
                    run_workbench(move || workbench::preview_import(&root, &folder_copy, &paths))
                        .await?;
                return Ok(serde_json::json!({
                    "confirmRequired": true,
                    "action": "import",
                    "folder": folder,
                    "files": preview,
                    "message": "还没有复制文件。向用户复述文件与目标文件夹；用户明确同意后再带 confirm=true 调用。",
                }));
            }
            let target = folder.clone();
            let outcomes =
                run_workbench(move || Ok(workbench::import_files(&root, &target, &paths, &lock)))
                    .await?;
            Ok(serde_json::json!({
                "done": true,
                "action": "import",
                "folder": folder,
                "files": outcomes,
            }))
        }
        "update" => {
            if request.path.is_empty() {
                return Err("请提供要更新的文件路径".into());
            }
            let path = request.path;
            let input = workbench::BindingInput {
                assignment: request.assignment,
                activity_id: request.activity_id,
                tags: request.tags,
                note: request.note,
            };
            if !confirm {
                let path_copy = path.clone();
                let input_copy = input.clone();
                let preview = run_workbench(move || {
                    workbench::validate_binding(&root, &path_copy, &input_copy)?;
                    workbench::file(&root, &path_copy)
                })
                .await?;
                return Ok(serde_json::json!({
                    "confirmRequired": true,
                    "action": "update",
                    "path": path,
                    "current": preview,
                    "proposed": {
                        "assignment": input.assignment,
                        "activityId": input.activity_id,
                        "tags": input.tags,
                        "note": input.note,
                    },
                    "message": "还没有保存。向用户复述新的绑定信息；用户明确同意后再带 confirm=true 调用。",
                }));
            }
            let updated =
                run_workbench(move || workbench::set_binding(&root, &path, input, &lock)).await?;
            Ok(serde_json::json!({ "done": true, "action": "update", "file": updated }))
        }
        "delete" => {
            if request.path.is_empty() {
                return Err("请提供要删除的文件路径".into());
            }
            let path = request.path;
            if !confirm {
                let path_copy = path.clone();
                let file = run_workbench(move || workbench::file(&root, &path_copy)).await?;
                return Ok(serde_json::json!({
                    "confirmRequired": true,
                    "action": "delete",
                    "file": file,
                    "message": "还没有删除，删除后无法从应用恢复。向用户确认文件和后果；用户明确同意后再带 confirm=true 调用。",
                }));
            }
            let target = path.clone();
            run_workbench(move || workbench::delete_file(&root, &target, &lock)).await?;
            Ok(serde_json::json!({ "done": true, "action": "delete", "path": path }))
        }
        other => Err(format!(
            "workbench_edit 不支持 action={other}，可选 import、mkdir、update、delete"
        )),
    }
}

/// Refresh the current semester from 思源学堂: create missing course folders and
/// keep each course's assignment list in app state (never as files on disk).
#[tauri::command]
pub(crate) async fn workbench_sync(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    main_only(&window)?;
    require_school_feature(&app, "lms")?;
    let (root, account_id) = workbench_root(&app, &state).await?;
    let (account, _) = school_account(&app, &state).await?;
    let _service_guard = state.school_lock.lock().await;
    let client = school::lms::Client::connect(&account)?;
    let data = client.workbench().await?;
    let term = data["term"].as_str().unwrap_or_default().trim().to_string();
    if term.is_empty() {
        return Err("思源学堂没有返回当前学期，暂时无法建立学期文件夹".into());
    }
    if term.chars().count() > 120 {
        return Err("学期名称过长".into());
    }
    let courses = data["courses"].as_array().cloned().unwrap_or_default();
    let course_count = courses.len();
    let failed = data["failed"].as_u64().unwrap_or(0);
    let synced = run_workbench(move || {
        let mut state = SyncState {
            version: 1,
            term: term.clone(),
            synced_at: now_millis(),
            courses: Vec::new(),
        };
        for course in &courses {
            let meta = workbench::CourseMeta {
                id: course["id"].as_u64().unwrap_or(0),
                code: course["code"].as_str().unwrap_or_default().to_string(),
                subject_code: course["subjectCode"]
                    .as_str()
                    .unwrap_or_default()
                    .to_string(),
                name: course["name"].as_str().unwrap_or_default().to_string(),
                term: term.clone(),
            };
            let folder = workbench::ensure_course_folder(&root, &term, &meta)?;
            state.courses.push(SyncCourse {
                id: meta.id,
                code: meta.code,
                subject_code: meta.subject_code,
                name: meta.name,
                folder,
                assignments: course["assignments"]
                    .as_array()
                    .cloned()
                    .unwrap_or_default(),
            });
        }
        Ok(state)
    })
    .await?;
    save_sync(&sync_state_path(&app, &account_id)?, &synced)?;
    let assignment_count: usize = synced
        .courses
        .iter()
        .map(|course| course.assignments.len())
        .sum();
    Ok(serde_json::json!({
        "term": synced.term,
        "syncedAt": synced.synced_at,
        "courses": course_count,
        "assignments": assignment_count,
        "failed": failed,
    }))
}
