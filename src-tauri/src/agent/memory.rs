//! Long-term memories in ~/.xjtu-launcher/agent/memory.json.
//! Entries are plain text and are injected into the Agent's system prompt.
use serde::{Deserialize, Serialize};
use std::path::Path;

const MAX_ENTRIES: usize = 200;
const MAX_CONTENT: usize = 1_000;
/// Keeps the injected block small enough for any provider's context window.
const PROMPT_LIMIT: usize = 4_000;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Memory {
    pub id: String,
    pub content: String,
    pub created_at: i64,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Book {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub entries: Vec<Memory>,
}

fn version() -> u32 {
    1
}

impl Default for Book {
    fn default() -> Self {
        Self {
            version: 1,
            entries: Vec::new(),
        }
    }
}

fn validate_content(content: &str) -> Result<String, String> {
    let content = content.trim();
    if content.is_empty() || content.chars().count() > MAX_CONTENT {
        return Err("记忆内容需要 1–1000 个字符".into());
    }
    Ok(content.to_string())
}

fn validate(book: &Book) -> Result<(), String> {
    if book.version != 1 || book.entries.len() > MAX_ENTRIES {
        return Err("记忆数量超限或版本不受支持".into());
    }
    for entry in &book.entries {
        if uuid::Uuid::parse_str(&entry.id).is_err() || validate_content(&entry.content).is_err() {
            return Err("记忆内容无效".into());
        }
    }
    Ok(())
}

pub fn load(path: &Path) -> Result<Book, String> {
    if !path.exists() {
        return Ok(Book::default());
    }
    let bytes = std::fs::read(path).map_err(|_| "无法读取全局记忆".to_string())?;
    let book: Book = serde_json::from_slice(&bytes)
        .map_err(|_| "全局记忆损坏，已保留原文件，请修复后重试".to_string())?;
    validate(&book)?;
    Ok(book)
}

pub fn save(path: &Path, book: &Book) -> Result<(), String> {
    validate(book)?;
    crate::config::write_atomic(
        path,
        &serde_json::to_vec_pretty(book).map_err(|_| "无法序列化全局记忆")?,
    )
}

/// Adds a memory or edits an existing one. Returns the stored entry.
pub fn upsert(book: &mut Book, id: Option<String>, content: &str) -> Result<Memory, String> {
    let content = validate_content(content)?;
    if let Some(id) = id {
        let entry = book
            .entries
            .iter_mut()
            .find(|entry| entry.id == id)
            .ok_or("记忆不存在")?;
        entry.content = content;
        return Ok(entry.clone());
    }
    let entry = Memory {
        id: uuid::Uuid::new_v4().to_string(),
        content,
        created_at: super::store::now(),
    };
    book.entries.push(entry.clone());
    if book.entries.len() > MAX_ENTRIES {
        book.entries.sort_by_key(|entry| entry.created_at);
        let excess = book.entries.len() - MAX_ENTRIES;
        book.entries.drain(..excess);
    }
    Ok(entry)
}

pub fn remove(book: &mut Book, id: &str) -> Result<(), String> {
    let before = book.entries.len();
    book.entries.retain(|entry| entry.id != id);
    if book.entries.len() == before {
        return Err("记忆不存在".into());
    }
    Ok(())
}

/// Prompt fragment; empty when there are no memories.
pub fn prompt_block(book: &Book) -> String {
    let mut block = String::new();
    for entry in &book.entries {
        let line = format!("- {}\n", entry.content.trim().replace('\n', " "));
        if block.chars().count() + line.chars().count() > PROMPT_LIMIT {
            break;
        }
        block.push_str(&line);
    }
    if block.is_empty() {
        return String::new();
    }
    format!("以下是用户同意长期保存的偏好与背景，回答时遵守：\n{block}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_edits_and_removes_entries() {
        let mut book = Book::default();
        let entry = upsert(&mut book, None, "  喜欢简洁回答  ").unwrap();
        assert_eq!(entry.content, "喜欢简洁回答");
        let edited = upsert(&mut book, Some(entry.id.clone()), "喜欢非常简洁的回答").unwrap();
        assert_eq!(edited.id, entry.id);
        assert_eq!(book.entries.len(), 1);
        remove(&mut book, &entry.id).unwrap();
        assert!(book.entries.is_empty());
        assert!(remove(&mut book, &entry.id).is_err());
    }

    #[test]
    fn builds_prompt_block_within_limit() {
        let mut book = Book::default();
        assert!(prompt_block(&book).is_empty());
        upsert(&mut book, None, "记忆一").unwrap();
        let block = prompt_block(&book);
        assert!(block.contains("记忆一"));
        for _ in 0..50 {
            let _ = upsert(&mut book, None, &"很长的记忆".repeat(100));
        }
        assert!(prompt_block(&book).chars().count() <= PROMPT_LIMIT + 60);
    }

    #[test]
    fn rejects_invalid_content() {
        let mut book = Book::default();
        assert!(upsert(&mut book, None, "   ").is_err());
        assert!(upsert(&mut book, None, &"a".repeat(MAX_CONTENT + 1)).is_err());
    }
}
