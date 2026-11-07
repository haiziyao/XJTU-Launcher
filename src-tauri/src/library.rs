//! Local file library (资料库). Files are copied into a per-account directory
//! under the app data folder and described by `index.json`. Nothing here talks
//! to the network: remote sync and sharing are deliberately not implemented.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};
use uuid::Uuid;

const VERSION: u32 = 1;
const MAX_ENTRIES: usize = 20_000;
const MAX_INDEX: u64 = 16 * 1024 * 1024;
const MAX_NAME: usize = 200;
const MAX_FIELD: usize = 120;
const MAX_NOTE: usize = 2_000;
const MAX_TAGS: usize = 8;
const MAX_TAG: usize = 32;
/// One local copy must fit in the app-managed folder; the same file is also
/// copied out for a school upload, so very large videos are not the target.
pub const MAX_FILE: u64 = 4 * 1024 * 1024 * 1024;
const BUFFER: usize = 64 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    /// Display name, including the original extension.
    pub name: String,
    /// File name inside `files/`; a content hash plus the original extension.
    pub stored_name: String,
    pub size: u64,
    pub sha256: String,
    pub added_at: i64,
    pub updated_at: i64,
    #[serde(default)]
    pub course: String,
    #[serde(default)]
    pub assignment: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EditInput {
    pub name: String,
    #[serde(default)]
    pub course: String,
    #[serde(default)]
    pub assignment: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    /// The path the user picked, echoed so a batch can report each file.
    pub path: String,
    pub entry: Option<Entry>,
    pub duplicate: bool,
    pub error: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Index {
    version: u32,
    entries: Vec<Entry>,
}

/// A stable, path-safe directory name for one campus account. The account id is
/// a UUID and survives re-login; a value that is not a UUID is hashed instead of
/// being written into a path.
pub fn owner_key(account_id: &str) -> String {
    Uuid::parse_str(account_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|_| hex(&Sha256::digest(account_id.as_bytes())))
}

/// `<文件文件夹>/workspace/<account>` — the user's own files, one folder per
/// account so switching accounts cannot show another account's files.
pub fn root(workspace: &Path, account_id: &str) -> PathBuf {
    workspace.join(owner_key(account_id))
}

/// Read the index without touching the files. A missing index is an empty
/// library; a damaged one is an error, so a write never overwrites metadata.
pub fn list(root: &Path) -> Result<Vec<Entry>, String> {
    let mut entries = load_index(root)?.entries;
    entries.sort_by(|left, right| {
        right
            .added_at
            .cmp(&left.added_at)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(entries)
}

pub fn import_all(root: &Path, paths: &[String], lock: &Mutex<()>) -> Vec<ImportResult> {
    paths
        .iter()
        .map(|path| {
            let source = PathBuf::from(path);
            match import(root, &source, lock) {
                Ok((entry, duplicate)) => ImportResult {
                    path: path.clone(),
                    entry: Some(entry),
                    duplicate,
                    error: None,
                },
                Err(error) => ImportResult {
                    path: path.clone(),
                    entry: None,
                    duplicate: false,
                    error: Some(error),
                },
            }
        })
        .collect()
}

fn import(root: &Path, source: &Path, lock: &Mutex<()>) -> Result<(Entry, bool), String> {
    if !source.is_absolute() {
        return Err("请选择有效的本地文件".into());
    }
    let metadata = fs::metadata(source).map_err(|_| "无法读取所选文件".to_string())?;
    if !metadata.is_file() {
        return Err("资料库只能添加文件，不能添加文件夹".into());
    }
    if metadata.len() > MAX_FILE {
        return Err(format!(
            "单个文件不能超过 {} GB",
            MAX_FILE / 1024 / 1024 / 1024
        ));
    }
    let name = display_name(source)?;
    let extension = extension(source);
    let files = root.join("files");
    fs::create_dir_all(&files).map_err(|_| "无法创建资料库目录")?;
    let temp = files.join(format!(".{}.part", Uuid::new_v4().simple()));
    let (sha256, size) = match copy_and_hash(source, &temp) {
        Ok(value) => value,
        Err(error) => {
            let _ = fs::remove_file(&temp);
            return Err(error);
        }
    };
    let result = commit(root, &name, &extension, sha256, size, temp.clone(), lock);
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn commit(
    root: &Path,
    name: &str,
    extension: &str,
    sha256: String,
    size: u64,
    temp: PathBuf,
    lock: &Mutex<()>,
) -> Result<(Entry, bool), String> {
    let guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut index = load_index(root)?;
    if let Some(existing) = index.entries.iter().find(|entry| entry.sha256 == sha256) {
        // The same bytes already have a record. Restore the stored copy if it
        // was removed outside the app; the record itself stays unchanged.
        let path = root.join("files").join(&existing.stored_name);
        if !path.exists() {
            fs::create_dir_all(root.join("files")).map_err(|_| "无法创建资料库目录")?;
            fs::rename(&temp, &path).map_err(|_| "无法保存本地副本")?;
        } else {
            let _ = fs::remove_file(&temp);
        }
        let entry = existing.clone();
        drop(guard);
        return Ok((entry, true));
    }
    if index.entries.len() >= MAX_ENTRIES {
        let _ = fs::remove_file(&temp);
        return Err("资料库文件数量已达上限".into());
    }
    let stored_name = stored_name(&sha256, extension);
    let destination = root.join("files").join(&stored_name);
    fs::create_dir_all(root.join("files")).map_err(|_| "无法创建资料库目录")?;
    fs::rename(&temp, &destination).map_err(|_| "无法保存本地副本")?;
    let now = now_ms();
    let entry = Entry {
        id: Uuid::new_v4().to_string(),
        name: name.to_string(),
        stored_name,
        size,
        sha256,
        added_at: now,
        updated_at: now,
        course: String::new(),
        assignment: String::new(),
        tags: Vec::new(),
        note: String::new(),
    };
    index.entries.push(entry.clone());
    if let Err(error) = save_index(root, &index) {
        // The copy was not recorded, so it must not take up space silently.
        let _ = fs::remove_file(&destination);
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    drop(guard);
    Ok((entry, false))
}

pub fn update(root: &Path, id: &str, input: EditInput, lock: &Mutex<()>) -> Result<Entry, String> {
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut index = load_index(root)?;
    let entry = index
        .entries
        .iter_mut()
        .find(|entry| entry.id == id)
        .ok_or("资料库中找不到这个文件")?;
    let name = input.name.trim();
    validate_text(name, MAX_NAME, "文件名")?;
    if name.is_empty() {
        return Err("文件名不能为空".into());
    }
    let course = input.course.trim();
    let assignment = input.assignment.trim();
    let note = input.note.trim();
    validate_text(course, MAX_FIELD, "课程")?;
    validate_text(assignment, MAX_FIELD, "作业")?;
    validate_text(note, MAX_NOTE, "备注")?;
    entry.name = name.to_string();
    entry.course = course.to_string();
    entry.assignment = assignment.to_string();
    entry.note = note.to_string();
    entry.tags = normalize_tags(input.tags)?;
    entry.updated_at = now_ms();
    let updated = entry.clone();
    save_index(root, &index)?;
    drop(_guard);
    Ok(updated)
}

pub fn delete(root: &Path, id: &str, lock: &Mutex<()>) -> Result<(), String> {
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut index = load_index(root)?;
    let position = index
        .entries
        .iter()
        .position(|entry| entry.id == id)
        .ok_or("资料库中找不到这个文件")?;
    let entry = index.entries.remove(position);
    let path = root.join("files").join(&entry.stored_name);
    match fs::remove_file(&path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        // The record stays on disk when the copy could not be removed.
        Err(_) => return Err("无法删除本地文件，请检查文件权限".into()),
    }
    save_index(root, &index)
}

/// Resolve one recorded entry to the local copy, refusing relative or escaped
/// stored names even if the index was edited by hand.
pub fn entry_path(root: &Path, id: &str) -> Result<PathBuf, String> {
    let index = load_index(root)?;
    let entry = index
        .entries
        .iter()
        .find(|entry| entry.id == id)
        .ok_or("资料库中找不到这个文件")?;
    let path = root.join("files").join(&entry.stored_name);
    if !path.is_file() {
        return Err("本地副本已丢失，可以重新添加这个文件".into());
    }
    Ok(path)
}

pub fn export(root: &Path, id: &str, target: &Path) -> Result<u64, String> {
    if !target.is_absolute() {
        return Err("请选择有效的保存位置".into());
    }
    let source = entry_path(root, id)?;
    if source == target {
        return Err("不能覆盖资料库中的原文件".into());
    }
    fs::copy(&source, target).map_err(|_| "无法保存文件，请检查目标位置".into())
}

fn load_index(root: &Path) -> Result<Index, String> {
    let path = root.join("index.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Index {
                version: VERSION,
                entries: Vec::new(),
            })
        }
        Err(_) => return Err("无法读取资料库索引".into()),
    };
    if bytes.len() as u64 > MAX_INDEX {
        return Err("资料库索引过大".into());
    }
    let index: Index = serde_json::from_slice(&bytes).map_err(|_| "资料库索引损坏")?;
    if index.version != VERSION {
        return Err("资料库索引版本不受支持".into());
    }
    validate_index(&index)?;
    Ok(index)
}

fn save_index(root: &Path, index: &Index) -> Result<(), String> {
    validate_index(index)?;
    fs::create_dir_all(root).map_err(|_| "无法创建资料库目录")?;
    let bytes = serde_json::to_vec_pretty(index).map_err(|_| "无法编码资料库索引")?;
    let temp = root.join(format!(".index-{}.tmp", Uuid::new_v4().simple()));
    fs::write(&temp, &bytes).map_err(|_| "无法写入资料库索引")?;
    if fs::rename(&temp, root.join("index.json")).is_err() {
        let _ = fs::remove_file(&temp);
        return Err("无法写入资料库索引".into());
    }
    Ok(())
}

fn validate_index(index: &Index) -> Result<(), String> {
    if index.entries.len() > MAX_ENTRIES {
        return Err("资料库索引过大".into());
    }
    let mut ids = std::collections::HashSet::new();
    let mut stored = std::collections::HashSet::new();
    for entry in &index.entries {
        if Uuid::parse_str(&entry.id).is_err()
            || !ids.insert(&entry.id)
            || !stored.insert(&entry.stored_name)
            || !valid_stored_name(&entry.stored_name)
            || entry.size > MAX_FILE
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            || entry.name.is_empty()
            || entry.name.chars().count() > MAX_NAME
            || entry.course.chars().count() > MAX_FIELD
            || entry.assignment.chars().count() > MAX_FIELD
            || entry.note.chars().count() > MAX_NOTE
            || entry.tags.len() > MAX_TAGS
            || entry.tags.iter().any(|tag| {
                tag.is_empty() || tag.chars().count() > MAX_TAG || tag.chars().any(char::is_control)
            })
        {
            return Err("资料库索引损坏".into());
        }
        validate_text(&entry.name, MAX_NAME, "文件名")?;
        validate_text(&entry.course, MAX_FIELD, "课程")?;
        validate_text(&entry.assignment, MAX_FIELD, "作业")?;
        validate_text(&entry.note, MAX_NOTE, "备注")?;
    }
    Ok(())
}

fn validate_text(value: &str, max: usize, label: &str) -> Result<(), String> {
    if value.chars().any(char::is_control) {
        return Err(format!("{label}不能包含控制字符"));
    }
    if value.chars().count() > max {
        return Err(format!("{label}不能超过 {max} 个字符"));
    }
    Ok(())
}

fn normalize_tags(tags: Vec<String>) -> Result<Vec<String>, String> {
    let mut unique: Vec<String> = Vec::new();
    for tag in tags {
        let tag = tag.trim();
        if tag.is_empty() {
            continue;
        }
        validate_text(tag, MAX_TAG, "标签")?;
        if !unique.iter().any(|item| item == tag) {
            unique.push(tag.to_string());
        }
    }
    if unique.len() > MAX_TAGS {
        return Err(format!("最多 {MAX_TAGS} 个标签"));
    }
    Ok(unique)
}

fn valid_stored_name(name: &str) -> bool {
    let (hash, extension) = name.split_once('.').unwrap_or((name, ""));
    hash.len() == 64
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && !extension.contains('/')
        && !extension.contains('\\')
        && extension.len() <= 12
        && extension.bytes().all(|byte| byte.is_ascii_alphanumeric())
        && Path::new(name).file_name().and_then(|value| value.to_str()) == Some(name)
}

fn display_name(path: &Path) -> Result<String, String> {
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("文件名不是有效的 UTF-8")?;
    validate_text(name, MAX_NAME, "文件名")?;
    if name.is_empty() {
        return Err("文件名不能为空".into());
    }
    Ok(name.to_string())
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .chars()
        .filter(|value| value.is_ascii_alphanumeric())
        .take(12)
        .collect::<String>()
        .to_ascii_lowercase()
}

fn stored_name(sha256: &str, extension: &str) -> String {
    if extension.is_empty() {
        sha256.to_string()
    } else {
        format!("{sha256}.{extension}")
    }
}

fn copy_and_hash(source: &Path, temp: &Path) -> Result<(String, u64), String> {
    let input = File::open(source).map_err(|_| "无法读取所选文件")?;
    let output = File::create(temp).map_err(|_| "无法创建本地副本")?;
    let mut reader = BufReader::with_capacity(BUFFER, input);
    let mut writer = BufWriter::with_capacity(BUFFER, output);
    let mut hasher = Sha256::new();
    let mut size = 0u64;
    let mut buffer = [0u8; BUFFER];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|_| "读取文件时出错".to_string())?;
        if read == 0 {
            break;
        }
        size += read as u64;
        if size > MAX_FILE {
            return Err(format!(
                "单个文件不能超过 {} GB",
                MAX_FILE / 1024 / 1024 / 1024
            ));
        }
        hasher.update(&buffer[..read]);
        writer
            .write_all(&buffer[..read])
            .map_err(|_| "写入本地副本时出错".to_string())?;
    }
    writer.flush().map_err(|_| "写入本地副本时出错")?;
    Ok((hex(&hasher.finalize()), size))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> String {
        Uuid::new_v4().to_string()
    }

    fn write(path: &Path, bytes: &[u8]) {
        fs::write(path, bytes).unwrap();
    }

    #[test]
    fn owner_keys_are_safe_and_distinct() {
        let id = account();
        assert_eq!(owner_key(&id), owner_key(&id));
        assert_ne!(owner_key(&id), owner_key(&account()));
        let fallback = owner_key("../../账号 A");
        assert_eq!(fallback.len(), 64);
        assert!(!fallback.contains('/') && !fallback.contains('\\'));
    }

    #[test]
    fn import_deduplicates_by_content_and_restores_a_lost_copy() {
        let data = tempfile::tempdir().unwrap();
        let source = data.path().join("作业.pdf");
        write(&source, b"same bytes");
        let root = root(data.path(), &account());
        let lock = Mutex::new(());
        let (first, duplicate) = import(&root, &source, &lock).unwrap();
        assert!(!duplicate);
        assert_eq!(first.name, "作业.pdf");
        assert_eq!(list(&root).unwrap().len(), 1);
        let (second, duplicate) = import(&root, &source, &lock).unwrap();
        assert!(duplicate);
        assert_eq!(first.id, second.id);
        assert_eq!(list(&root).unwrap().len(), 1);
        fs::remove_file(root.join("files").join(&first.stored_name)).unwrap();
        let (restored, duplicate) = import(&root, &source, &lock).unwrap();
        assert!(duplicate);
        assert!(root.join("files").join(&restored.stored_name).is_file());
    }

    #[test]
    fn metadata_round_trips_and_delete_removes_the_copy() {
        let data = tempfile::tempdir().unwrap();
        let source = data.path().join("report.docx");
        write(&source, b"document");
        let root = root(data.path(), &account());
        let lock = Mutex::new(());
        let (entry, _) = import(&root, &source, &lock).unwrap();
        let updated = update(
            &root,
            &entry.id,
            EditInput {
                name: "操作系统实验报告.docx".into(),
                course: "操作系统".into(),
                assignment: "第 6 次作业".into(),
                tags: vec!["作业".into(), "作业".into(), "  ".into(), "实验".into()],
                note: "最终版".into(),
            },
            &lock,
        )
        .unwrap();
        assert_eq!(updated.name, "操作系统实验报告.docx");
        assert_eq!(updated.tags, vec!["作业", "实验"]);
        let reloaded = list(&root).unwrap();
        assert_eq!(reloaded[0].course, "操作系统");
        delete(&root, &entry.id, &lock).unwrap();
        assert!(list(&root).unwrap().is_empty());
        assert!(!root.join("files").join(&entry.stored_name).exists());
    }

    #[test]
    fn export_writes_a_copy_and_rejects_the_source_path() {
        let data = tempfile::tempdir().unwrap();
        let source = data.path().join("notes.txt");
        write(&source, b"hello");
        let root = root(data.path(), &account());
        let lock = Mutex::new(());
        let (entry, _) = import(&root, &source, &lock).unwrap();
        let stored = entry_path(&root, &entry.id).unwrap();
        assert!(export(&root, &entry.id, &stored).is_err());
        let target = data.path().join("copy.txt");
        assert_eq!(export(&root, &entry.id, &target).unwrap(), 5);
        assert_eq!(fs::read(&target).unwrap(), b"hello");
    }

    #[test]
    fn a_hand_edited_index_cannot_escape_the_library_folder() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        fs::create_dir_all(root.join("files")).unwrap();
        fs::write(
            root.join("index.json"),
            r#"{"version":1,"entries":[{"id":"00000000-0000-4000-8000-000000000000","name":"x","storedName":"../../secret","size":1,"sha256":"0000000000000000000000000000000000000000000000000000000000000000","addedAt":1,"updatedAt":1}]}"#,
        )
        .unwrap();
        assert!(list(&root).is_err());
    }

    #[test]
    fn import_rejects_folders_relative_paths_and_oversized_names() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        let lock = Mutex::new(());
        assert!(import(&root, Path::new("relative.txt"), &lock).is_err());
        assert!(import(&root, data.path(), &lock).is_err());
        let long = data.path().join(format!("{}.txt", "a".repeat(MAX_NAME)));
        write(&long, b"x");
        assert!(import(&root, &long, &lock).is_err());
    }
}
