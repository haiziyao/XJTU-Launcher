//! Tauri commands for the agent domain.

use crate::*;
use tauri::WebviewWindow;

#[tauri::command]
pub(crate) fn agent_list(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Vec<agent::store::Summary>, String> {
    main_only(&window)?;
    Ok(agent::store::summaries(&agent::store::load(&agent::path(
        &app,
    )?)?))
}
#[tauri::command]
pub(crate) fn agent_last_opened(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Option<String>, String> {
    main_only(&window)?;
    Ok(agent::store::last_opened(&agent::store::load(
        &agent::path(&app)?,
    )?))
}
#[tauri::command]
pub(crate) fn agent_set_last_opened(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: Option<String>,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    agent::store::set_last_opened(&mut book, id)?;
    agent::store::save(&path, &book)
}
#[tauri::command]
pub(crate) fn agent_get(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
) -> Result<agent::store::Conversation, String> {
    main_only(&window)?;
    Ok(agent::store::find(&agent::store::load(&agent::path(&app)?)?, &id)?.clone())
}
#[tauri::command]
pub(crate) fn agent_new(
    app: tauri::AppHandle,
    window: WebviewWindow,
    workspace_id: Option<String>,
) -> Result<agent::store::Conversation, String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    let conversation = agent::store::create(&mut book, workspace_id);
    agent::store::save(&path, &book)?;
    Ok(conversation)
}
#[tauri::command]
pub(crate) fn agent_workspaces(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Vec<agent::store::Workspace>, String> {
    main_only(&window)?;
    Ok(agent::store::workspaces(&agent::store::load(
        &agent::path(&app)?,
    )?))
}
#[tauri::command]
pub(crate) fn agent_set_workspace(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
    workspace_id: Option<String>,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    agent::store::set_workspace(&mut book, &id, workspace_id)?;
    agent::store::save(&path, &book)
}
#[tauri::command]
pub(crate) fn agent_create_workspace(
    app: tauri::AppHandle,
    window: WebviewWindow,
    name: String,
) -> Result<agent::store::Workspace, String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    let workspace = agent::store::create_workspace(&mut book, &name)?;
    agent::store::save(&path, &book)?;
    Ok(workspace)
}
#[tauri::command]
pub(crate) fn agent_delete(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    agent::store::remove(&mut book, &id)?;
    agent::store::save(&path, &book)
}
#[tauri::command]
pub(crate) fn agent_rename(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
    title: String,
) -> Result<(), String> {
    main_only(&window)?;
    let title = title.trim();
    if title.is_empty() || title.chars().count() > 60 {
        return Err("标题需要 1–60 个字符".into());
    }
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    let conversation = agent::store::find_mut(&mut book, &id)?;
    conversation.title = title.to_string();
    conversation.updated_at = agent::store::now();
    agent::store::save(&path, &book)
}
#[tauri::command]
pub(crate) async fn agent_send(
    app: tauri::AppHandle,
    window: WebviewWindow,
    input: agent::Send,
) -> Result<(), String> {
    main_only(&window)?;
    agent::send(app, input).await
}
#[tauri::command]
pub(crate) fn agent_cancel(
    window: WebviewWindow,
    state: tauri::State<AppState>,
    request_id: String,
) -> Result<(), String> {
    main_only(&window)?;
    state.agent.cancel(&request_id);
    Ok(())
}
pub(crate) fn drop_by<T>(list: &mut Vec<T>, predicate: impl Fn(&T) -> bool) -> bool {
    match list.iter().position(predicate) {
        Some(index) => {
            list.remove(index);
            true
        }
        None => false,
    }
}

#[tauri::command]
pub(crate) fn agent_presets(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<agent::presets::Presets, String> {
    main_only(&window)?;
    agent::presets::load(&agent::presets_path(&app)?)
}
#[tauri::command]
pub(crate) fn agent_save_preset(
    app: tauri::AppHandle,
    window: WebviewWindow,
    input: agent::presets::Input,
) -> Result<String, String> {
    main_only(&window)?;
    let path = agent::presets_path(&app)?;
    let mut presets = agent::presets::load(&path)?;
    let id = agent::presets::upsert(&mut presets, input)?;
    agent::presets::save(&path, &presets)?;
    Ok(id)
}
#[tauri::command]
pub(crate) fn agent_remove_preset(
    app: tauri::AppHandle,
    window: WebviewWindow,
    kind: String,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::presets_path(&app)?;
    let mut presets = agent::presets::load(&path)?;
    let removed = match kind.as_str() {
        "role" => drop_by(&mut presets.roles, |item| item.id == id),
        "skill" => drop_by(&mut presets.skills, |item| item.id == id),
        "task" => drop_by(&mut presets.tasks, |item| item.id == id),
        _ => return Err("未知的预设类型".into()),
    };
    if !removed {
        return Err("预设不存在".into());
    }
    agent::presets::save(&path, &presets)
}
#[tauri::command]
pub(crate) fn agent_memories(
    app: tauri::AppHandle,
    window: WebviewWindow,
) -> Result<Vec<agent::memory::Memory>, String> {
    main_only(&window)?;
    Ok(agent::memory::load(&agent::memory_path(&app)?)?.entries)
}
#[tauri::command]
pub(crate) fn agent_save_memory(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: Option<String>,
    content: String,
) -> Result<agent::memory::Memory, String> {
    main_only(&window)?;
    let path = agent::memory_path(&app)?;
    let mut book = agent::memory::load(&path)?;
    let entry = agent::memory::upsert(&mut book, id, &content)?;
    agent::memory::save(&path, &book)?;
    Ok(entry)
}
#[tauri::command]
pub(crate) fn agent_remove_memory(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::memory_path(&app)?;
    let mut book = agent::memory::load(&path)?;
    agent::memory::remove(&mut book, &id)?;
    agent::memory::save(&path, &book)
}
#[tauri::command]
pub(crate) fn agent_set_role(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
    role_id: Option<String>,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    let conversation = agent::store::find_mut(&mut book, &id)?;
    conversation.role_id = role_id;
    conversation.updated_at = agent::store::now();
    agent::store::save(&path, &book)
}
#[tauri::command]
pub(crate) fn agent_pin(
    app: tauri::AppHandle,
    window: WebviewWindow,
    id: String,
    pinned: bool,
) -> Result<(), String> {
    main_only(&window)?;
    let path = agent::path(&app)?;
    let mut book = agent::store::load(&path)?;
    let conversation = agent::store::find_mut(&mut book, &id)?;
    conversation.pinned = pinned;
    agent::store::save(&path, &book)
}
#[tauri::command]
pub(crate) async fn agent_tools(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<agent::tools::ToolInfo>, String> {
    main_only(&window)?;
    Ok(state.agent.tool_infos(&app).await)
}
