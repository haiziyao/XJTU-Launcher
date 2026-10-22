//! Agent runtime: conversation storage, model streaming and the tool loop.
pub mod chat;
pub mod memory;
pub mod presets;
pub mod store;
pub mod tools;

use crate::{agent::chat::Chat, agent::store::Conversation, providers, vault};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter, Manager as _};

const TOOL_CACHE: Duration = Duration::from_secs(120);

#[derive(Default)]
pub struct Manager {
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
    /// Conversations with a request in flight; one turn at a time per conversation.
    active: Mutex<HashSet<String>>,
    store_lock: Mutex<()>,
    tools: tokio::sync::Mutex<Option<(Instant, Vec<tools::ToolDef>)>>,
}

fn directory(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .home_dir()
        .map_err(|_| "无法定位对话目录")?
        .join(".xjtu-launcher")
        .join("agent"))
}

pub fn path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(directory(app)?.join("conversations.json"))
}

pub fn presets_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(directory(app)?.join("presets.json"))
}

pub fn memory_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(directory(app)?.join("memory.json"))
}

impl Manager {
    fn register(&self, request_id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Ok(mut cancels) = self.cancels.lock() {
            cancels.insert(request_id.to_string(), flag.clone());
        }
        flag
    }
    pub fn cancel(&self, request_id: &str) {
        if let Ok(cancels) = self.cancels.lock() {
            if let Some(flag) = cancels.get(request_id) {
                flag.store(true, Ordering::Relaxed);
            }
        }
    }
    fn finish(&self, request_id: &str) {
        if let Ok(mut cancels) = self.cancels.lock() {
            cancels.remove(request_id);
        }
    }
    fn claim(&self, conversation_id: &str) -> Result<(), String> {
        let mut active = self.active.lock().map_err(|_| "对话状态繁忙")?;
        if !active.insert(conversation_id.to_string()) {
            return Err("这条对话还在回答，请稍候或先停止".into());
        }
        Ok(())
    }
    fn release(&self, conversation_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(conversation_id);
        }
    }
    pub fn invalidate_tools(&self) {
        if let Ok(mut tools) = self.tools.try_lock() {
            *tools = None;
        }
    }
    pub async fn tool_infos(&self, app: &AppHandle) -> Vec<tools::ToolInfo> {
        tools::info(&self.tool_set(app).await)
    }
    async fn tool_set(&self, app: &AppHandle) -> Vec<tools::ToolDef> {
        let mut cache = self.tools.lock().await;
        if let Some((at, defs)) = cache.as_ref() {
            if at.elapsed() < TOOL_CACHE {
                return defs.clone();
            }
        }
        let defs = tools::build(app).await;
        *cache = Some((Instant::now(), defs.clone()));
        defs
    }
    fn persist(&self, path: &Path, conversation: &Conversation) {
        let _guard = self.store_lock.lock();
        let Ok(mut book) = store::load(path) else {
            return;
        };
        if let Ok(target) = store::find_mut(&mut book, &conversation.id) {
            let mut next = conversation.clone();
            next.updated_at = store::now();
            *target = next;
        }
        let _ = store::save(path, &book);
    }
}

fn provider_for(
    app: &AppHandle,
    provider_id: Option<&str>,
    model: Option<&str>,
) -> Result<chat::Provider, String> {
    let book = providers::load(&crate::providers_path(app)?)?;
    let id = provider_id
        .map(str::to_string)
        .or_else(|| book.default_id.clone())
        .ok_or("请先在 Agent 设置中选择模型服务")?;
    let provider = book
        .providers
        .iter()
        .find(|provider| provider.id == id)
        .ok_or("模型服务不存在，请在 Agent 设置中重新选择")?;
    let api_key = if provider.api_key.trim().is_empty() {
        vault::get("model-api-key")?.unwrap_or_default()
    } else {
        provider.api_key.clone()
    };
    // The chat page sends the model it currently shows; fall back to the
    // provider's stored choice when the request does not carry one.
    let model = model
        .map(str::trim)
        .filter(|model| !model.is_empty() && model.len() <= 200)
        .map(str::to_string)
        .unwrap_or_else(|| provider.model.clone());
    Ok(chat::Provider {
        protocol: provider.protocol.clone(),
        base_url: provider.base_url.clone(),
        model,
        api_key,
    })
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Send {
    pub conversation_id: String,
    pub request_id: String,
    pub content: String,
    pub provider_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub skill_ids: Vec<String>,
}

/// Appends the user turn, then streams the answer on a background task.
pub async fn send(app: AppHandle, input: Send) -> Result<(), String> {
    let Send {
        conversation_id,
        request_id,
        content,
        provider_id,
        model,
        skill_ids,
    } = input;
    let content = content.trim().to_string();
    if content.is_empty() {
        return Err("请输入内容".into());
    }
    let provider = provider_for(&app, provider_id.as_deref(), model.as_deref())?;
    let path = path(&app)?;
    let state = app.state::<crate::AppState>();
    let _guard = state.agent.store_lock.lock().map_err(|_| "对话状态繁忙")?;
    let mut book = store::load(&path)?;
    let conversation = store::find_mut(&mut book, &conversation_id)?;
    store::append(conversation, store::Message::user(content));
    let conversation = conversation.clone();
    store::save(&path, &book)?;
    drop(_guard);
    let presets = presets::load(&presets_path(&app)?)?;
    let memory = memory::prompt_block(&memory::load(&memory_path(&app)?)?);
    let role = conversation
        .role_id
        .as_ref()
        .and_then(|id| presets.roles.iter().find(|role| &role.id == id).cloned());
    let skills = presets.skills.clone();
    let active_skills = skills
        .iter()
        .filter(|skill| skill_ids.iter().any(|id| id == &skill.id))
        .cloned()
        .collect::<Vec<_>>();
    state.agent.claim(&conversation_id)?;
    let tools = state.agent.tool_set(&app).await;
    let cancel = state.agent.register(&request_id);

    tauri::async_runtime::spawn(async move {
        let state = app.state::<crate::AppState>();
        let request = request_id.clone();
        let mut conversation = conversation;
        let result = {
            let mut emit = |event: chat::Event| {
                let _ = app.emit("agent-event", event);
            };
            let mut persist = |conversation: &Conversation| {
                state.agent.persist(&path, conversation);
            };
            let mut chat = Chat {
                app: app.clone(),
                provider,
                tools,
                role,
                skills,
                active_skills,
                memory,
                request_id: request.clone(),
                cancel: cancel.clone(),
                emit: &mut emit,
                persist: &mut persist,
            };
            chat.run(&mut conversation).await
        };
        match result {
            Ok(()) if cancel.load(Ordering::Relaxed) => {
                let _ = app.emit(
                    "agent-event",
                    chat::Event::Cancelled {
                        request_id: request.clone(),
                    },
                );
            }
            Ok(()) => {
                let _ = app.emit(
                    "agent-event",
                    chat::Event::Done {
                        request_id: request.clone(),
                        conversation_id: conversation_id.clone(),
                    },
                );
            }
            Err(error) if error == "__cancelled__" || cancel.load(Ordering::Relaxed) => {
                let _ = app.emit(
                    "agent-event",
                    chat::Event::Cancelled {
                        request_id: request.clone(),
                    },
                );
            }
            Err(error) => {
                let _ = app.emit(
                    "agent-event",
                    chat::Event::Error {
                        request_id: request.clone(),
                        message: error,
                    },
                );
            }
        }
        state.agent.finish(&request);
        state.agent.release(&conversation_id);
    });
    Ok(())
}

/// Removes MCP entries that disappeared from the local configuration.
pub fn invalidate_for_mcp(app: &AppHandle) {
    app.state::<crate::AppState>().agent.invalidate_tools();
}
