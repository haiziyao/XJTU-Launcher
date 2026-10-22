//! Agent conversations in ~/.xjtu-launcher/agent/conversations.json.
//! Messages are plain text plus tool calls; API keys never enter this file.
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_CONVERSATIONS: usize = 100;
const MAX_MESSAGES: usize = 2000;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    /// Raw JSON arguments exactly as returned by the model.
    pub arguments: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Message {
    /// user | assistant | tool
    pub role: String,
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub error: bool,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: text.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
            error: false,
        }
    }
    pub fn assistant(text: String, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".into(),
            content: text,
            tool_calls: (!tool_calls.is_empty()).then_some(tool_calls),
            tool_call_id: None,
            name: None,
            error: false,
        }
    }
    pub fn tool(call: &ToolCall, content: impl Into<String>, error: bool) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(call.id.clone()),
            name: Some(call.name.clone()),
            error,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    /// Workspace this conversation belongs to, if any.
    #[serde(default)]
    pub workspace_id: Option<String>,
    #[serde(default)]
    pub messages: Vec<Message>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub title: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub pinned: bool,
    pub role_id: Option<String>,
    pub workspace_id: Option<String>,
    pub messages: usize,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Store {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub conversations: Vec<Conversation>,
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
}

fn version() -> u32 {
    1
}

impl Default for Store {
    fn default() -> Self {
        Self {
            version: 1,
            conversations: Vec::new(),
            workspaces: Vec::new(),
        }
    }
}

pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

pub fn load(path: &Path) -> Result<Store, String> {
    if !path.exists() {
        return Ok(Store::default());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取对话记录".to_string())?;
    let store: Store = serde_json::from_slice(&bytes)
        .map_err(|_| "对话记录损坏，已保留原文件，请修复后重试".to_string())?;
    if store.version != 1 {
        return Err("对话记录版本不受支持".into());
    }
    Ok(store)
}

pub fn save(path: &Path, store: &Store) -> Result<(), String> {
    crate::config::write_atomic(
        path,
        &serde_json::to_vec_pretty(store).map_err(|_| "无法序列化对话记录")?,
    )
}

/// Adds a conversation and returns a copy; callers should keep using `find_mut`.
pub fn create(store: &mut Store, workspace_id: Option<String>) -> Conversation {
    let workspace_id =
        workspace_id.filter(|id| store.workspaces.iter().any(|workspace| &workspace.id == id));
    let conversation = Conversation {
        id: uuid::Uuid::new_v4().to_string(),
        title: "新对话".into(),
        created_at: now(),
        updated_at: now(),
        role_id: None,
        pinned: false,
        workspace_id,
        messages: Vec::new(),
    };
    store.conversations.push(conversation.clone());
    if store.conversations.len() > MAX_CONVERSATIONS {
        store
            .conversations
            .sort_by_key(|conversation| conversation.updated_at);
        let excess = store.conversations.len() - MAX_CONVERSATIONS;
        store.conversations.drain(..excess);
    }
    conversation
}

pub fn summaries(store: &Store) -> Vec<Summary> {
    let mut list: Vec<Summary> = store
        .conversations
        .iter()
        .map(|conversation| Summary {
            id: conversation.id.clone(),
            title: conversation.title.clone(),
            created_at: conversation.created_at,
            updated_at: conversation.updated_at,
            pinned: conversation.pinned,
            role_id: conversation.role_id.clone(),
            workspace_id: conversation.workspace_id.clone(),
            messages: conversation.messages.len(),
        })
        .collect();
    list.sort_by_key(|summary| {
        (
            std::cmp::Reverse(summary.pinned),
            std::cmp::Reverse(summary.updated_at),
        )
    });
    list
}

/// Workspaces are just named groups of conversations, stored beside them.
pub fn create_workspace(store: &mut Store, name: &str) -> Result<Workspace, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 40 {
        return Err("工作区名称需要 1–40 个字符".into());
    }
    if store.workspaces.len() >= 32 {
        return Err("工作区数量已达上限".into());
    }
    let workspace = Workspace {
        id: uuid::Uuid::new_v4().to_string(),
        name: name.to_string(),
        created_at: now(),
    };
    store.workspaces.push(workspace.clone());
    Ok(workspace)
}

pub fn workspaces(store: &Store) -> Vec<Workspace> {
    let mut list = store.workspaces.clone();
    list.sort_by_key(|workspace| workspace.created_at);
    list
}

pub fn find_mut<'a>(store: &'a mut Store, id: &str) -> Result<&'a mut Conversation, String> {
    store
        .conversations
        .iter_mut()
        .find(|conversation| conversation.id == id)
        .ok_or("对话不存在".into())
}

pub fn find<'a>(store: &'a Store, id: &str) -> Result<&'a Conversation, String> {
    store
        .conversations
        .iter()
        .find(|conversation| conversation.id == id)
        .ok_or("对话不存在".into())
}

pub fn remove(store: &mut Store, id: &str) -> Result<(), String> {
    let before = store.conversations.len();
    store
        .conversations
        .retain(|conversation| conversation.id != id);
    if store.conversations.len() == before {
        return Err("对话不存在".into());
    }
    Ok(())
}

/// Adds a message, trims the oldest history and keeps the title in sync.
pub fn append(conversation: &mut Conversation, message: Message) {
    if conversation.title == "新对话"
        && message.role == "user"
        && !message.content.trim().is_empty()
    {
        let title: String = message.content.trim().chars().take(24).collect();
        conversation.title = if message.content.trim().chars().count() > 24 {
            format!("{title}…")
        } else {
            title
        };
    }
    conversation.messages.push(message);
    if conversation.messages.len() > MAX_MESSAGES {
        let excess = conversation.messages.len() - MAX_MESSAGES;
        conversation.messages.drain(..excess);
    }
    conversation.updated_at = now();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_titles_and_trims_history() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conversations.json");
        let mut store = load(&path).unwrap();
        let created = create(&mut store, None);
        let conversation = find_mut(&mut store, &created.id).unwrap();
        append(conversation, Message::user("帮我看看明天的课"));
        assert_eq!(conversation.title, "帮我看看明天的课");
        let call = ToolCall {
            id: "1".into(),
            name: "campus".into(),
            arguments: "{}".into(),
        };
        let conversation = find_mut(&mut store, &created.id).unwrap();
        append(
            conversation,
            Message::assistant("".into(), vec![call.clone()]),
        );
        append(conversation, Message::tool(&call, "ok", false));
        save(&path, &store).unwrap();
        let reloaded = load(&path).unwrap();
        let saved = find(&reloaded, &created.id).unwrap();
        assert_eq!(saved.messages.len(), 3);
        assert_eq!(saved.messages[1].tool_calls.as_ref().unwrap()[0].id, "1");
        let summaries = summaries(&reloaded);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].messages, 3);
    }

    #[test]
    fn rejects_unknown_conversations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("conversations.json");
        let mut store = load(&path).unwrap();
        assert!(remove(&mut store, "missing").is_err());
        assert!(find(&store, "missing").is_err());
    }
}
