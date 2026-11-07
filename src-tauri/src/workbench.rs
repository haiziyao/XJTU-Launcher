//! 工作台：用户自己的文件夹，加上应用侧的少量元数据。
//!
//! The folder tree on disk is the source of truth: semester folders contain
//! course folders, course folders carry a hidden `.xjtu-course.json` marker so
//! a folder that the user renamed is still the same course, and everything
//! else is a plain folder the user may create in Finder. `index.json` only
//! keeps per-file extras (assignment binding, tags, note); a file that is not
//! in it is still shown.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs,
    path::{Component, Path, PathBuf},
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

/// Files whose course cannot be recovered from the old index start here.
pub const UNFILED: &str = "未归类";
/// Hidden course marker; the folder name is the user's custom alias.
pub const MARKER: &str = ".xjtu-course.json";
pub const MAX_FILE: u64 = 4 * 1024 * 1024 * 1024;
const VERSION: u32 = 2;
const MAX_DEPTH: usize = 6;
const MAX_NODES: usize = 20_000;
const MAX_NAME: usize = 120;
const MAX_NOTE: usize = 2_000;
const MAX_TAGS: usize = 8;
const MAX_TAG: usize = 32;
const MAX_META: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CourseMeta {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub code: String,
    /// Stable course number (`subject_code`); the folder is matched on it so a
    /// rename or a new LMS id in another semester keeps the same folder.
    #[serde(default)]
    pub subject_code: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub term: String,
}

impl CourseMeta {
    fn key(&self) -> String {
        if !self.subject_code.is_empty() {
            self.subject_code.clone()
        } else if !self.code.is_empty() {
            self.code.clone()
        } else {
            format!("id:{}", self.id)
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderNode {
    pub path: String,
    pub name: String,
    pub parent: String,
    /// `term`, `course` or `folder`.
    pub kind: String,
    pub course: Option<CourseMeta>,
    /// Files directly inside, and folders directly inside.
    pub files: usize,
    pub folders: usize,
    pub modified: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileNode {
    pub path: String,
    pub name: String,
    pub folder: String,
    pub size: u64,
    pub modified: i64,
    pub course: Option<CourseMeta>,
    pub assignment: String,
    pub activity_id: Option<u64>,
    pub tags: Vec<String>,
    pub note: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BindingInput {
    #[serde(default)]
    pub assignment: String,
    #[serde(default)]
    pub activity_id: Option<u64>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub path: String,
    pub file: Option<FileNode>,
    pub error: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
struct Index {
    version: u32,
    #[serde(default)]
    files: HashMap<String, FileMeta>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileMeta {
    #[serde(default)]
    assignment: String,
    #[serde(default)]
    activity_id: Option<u64>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    note: String,
    #[serde(default)]
    added_at: i64,
}

/// A stable, path-safe directory name for one campus account.
pub fn owner_key(account_id: &str) -> String {
    uuid::Uuid::parse_str(account_id)
        .map(|id| id.to_string())
        .unwrap_or_else(|_| hex(&Sha256::digest(account_id.as_bytes())))
}

pub fn root(workspace: &Path, account_id: &str) -> PathBuf {
    workspace.join(owner_key(account_id))
}

/// Read the folder tree below the workspace root. Hidden folders and symlinks
/// are skipped; a marker turns a folder into a course, and a folder holding a
/// course turns into a semester.
pub fn tree(root: &Path) -> Result<Vec<FolderNode>, String> {
    migrate(root)?;
    let mut nodes = Vec::new();
    walk(root, root, 0, &mut nodes)?;
    nodes.sort_by(|left, right| {
        left.parent
            .cmp(&right.parent)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(nodes)
}

fn walk(root: &Path, dir: &Path, depth: usize, nodes: &mut Vec<FolderNode>) -> Result<(), String> {
    if depth >= MAX_DEPTH {
        return Ok(());
    }
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("无法读取工作台文件夹".into()),
    };
    let mut children: Vec<(String, PathBuf, fs::Metadata)> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.') {
            continue;
        }
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        match fs::metadata(&path) {
            Ok(metadata) if metadata.is_dir() => children.push((name, path, metadata)),
            _ => {}
        }
    }
    children.sort_by(|left, right| left.0.cmp(&right.0));
    for (name, path, metadata) in children {
        if nodes.len() >= MAX_NODES {
            return Err("工作台文件夹数量已达上限".into());
        }
        let (files, folders) = direct_counts(&path);
        let course = read_marker(&path);
        let kind = if course.is_some() {
            "course"
        } else if has_course(&path, 0) {
            "term"
        } else {
            "folder"
        };
        nodes.push(FolderNode {
            path: relative(root, &path)?,
            name,
            parent: relative(root, path.parent().unwrap_or(root)).unwrap_or_default(),
            kind: kind.to_string(),
            course,
            files,
            folders,
            modified: modified(&metadata),
        });
        walk(root, &path, depth + 1, nodes)?;
    }
    Ok(())
}

fn direct_counts(dir: &Path) -> (usize, usize) {
    let mut files = 0;
    let mut folders = 0;
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.')
                || entry
                    .file_type()
                    .map(|kind| kind.is_symlink())
                    .unwrap_or(false)
            {
                continue;
            }
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                folders += 1;
            } else {
                files += 1;
            }
        }
    }
    (files, folders)
}

fn has_course(dir: &Path, depth: usize) -> bool {
    if depth > MAX_DEPTH {
        return false;
    }
    if read_marker(dir).is_some() {
        return true;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_string();
        !name.starts_with('.')
            && entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
            && has_course(&entry.path(), depth + 1)
    })
}

/// Files directly in one folder, merged with the per-file extras.
pub fn files(root: &Path, folder: &str) -> Result<Vec<FileNode>, String> {
    migrate(root)?;
    let dir = path_of(root, folder)?;
    if !dir.is_dir() {
        return Err("文件夹不存在".into());
    }
    let index = load_index(root)?;
    let course = course_of(root, folder)?;
    let entries = fs::read_dir(&dir).map_err(|_| "无法读取文件夹")?;
    let mut files = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with('.')
            || entry
                .file_type()
                .map(|kind| kind.is_symlink())
                .unwrap_or(false)
        {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) if metadata.is_file() => metadata,
            _ => continue,
        };
        let relative = join_rel(folder, &name);
        let meta = index.files.get(&relative).cloned().unwrap_or_default();
        files.push(FileNode {
            path: relative,
            name,
            folder: folder.to_string(),
            size: metadata.len(),
            modified: modified(&metadata),
            course: course.clone(),
            assignment: meta.assignment,
            activity_id: meta.activity_id,
            tags: meta.tags,
            note: meta.note,
        });
    }
    files.sort_by(|left, right| {
        right
            .modified
            .cmp(&left.modified)
            .then_with(|| left.name.cmp(&right.name))
    });
    Ok(files)
}

/// Nearest course marker at or above `folder`.
pub fn course_of(root: &Path, folder: &str) -> Result<Option<CourseMeta>, String> {
    let mut current = path_of(root, folder)?;
    loop {
        if let Some(course) = read_marker(&current) {
            return Ok(Some(course));
        }
        if current == root || !current.pop() {
            return Ok(None);
        }
    }
}

pub fn create_folder(root: &Path, parent: &str, name: &str) -> Result<FolderNode, String> {
    let name = name.trim();
    validate_name(name)?;
    let parent_path = path_of(root, parent)?;
    if !parent_path.is_dir() {
        return Err("上级文件夹不存在".into());
    }
    let target = parent_path.join(name);
    if target.exists() {
        return Err("同名文件夹已存在".into());
    }
    fs::create_dir(&target).map_err(|_| "无法创建文件夹")?;
    let metadata = fs::metadata(&target).map_err(|_| "无法读取新文件夹")?;
    let (files, folders) = direct_counts(&target);
    Ok(FolderNode {
        path: join_rel(parent, name),
        name: name.to_string(),
        parent: parent.to_string(),
        kind: "folder".into(),
        course: None,
        files,
        folders,
        modified: modified(&metadata),
    })
}

/// Copy picked files into a folder, keeping the original names. A name that is
/// already taken gets ` (2)`, so two photos called `IMG_1.jpg` both survive.
pub fn import_files(
    root: &Path,
    folder: &str,
    paths: &[String],
    lock: &Mutex<()>,
) -> Vec<ImportOutcome> {
    paths
        .iter()
        .map(|path| {
            let source = PathBuf::from(path);
            match import_one(root, folder, &source, lock) {
                Ok(file) => ImportOutcome {
                    path: path.clone(),
                    file: Some(file),
                    error: None,
                },
                Err(error) => ImportOutcome {
                    path: path.clone(),
                    file: None,
                    error: Some(error),
                },
            }
        })
        .collect()
}

fn import_one(
    root: &Path,
    folder: &str,
    source: &Path,
    lock: &Mutex<()>,
) -> Result<FileNode, String> {
    if !source.is_absolute() {
        return Err("请选择有效的本地文件".into());
    }
    let metadata = fs::metadata(source).map_err(|_| "无法读取所选文件".to_string())?;
    if !metadata.is_file() {
        return Err("只能添加文件，不能添加文件夹".into());
    }
    if metadata.len() > MAX_FILE {
        return Err(format!(
            "单个文件不能超过 {} GB",
            MAX_FILE / 1024 / 1024 / 1024
        ));
    }
    let name = source
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or("文件名不是有效的 UTF-8")?
        .trim()
        .to_string();
    validate_name(&name)?;
    let dir = path_of(root, folder)?;
    if !dir.is_dir() {
        return Err("文件夹不存在".into());
    }
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (target, name) = unique_target(&dir, &name);
    fs::copy(source, &target).map_err(|_| "无法复制文件")?;
    let metadata = fs::metadata(&target).map_err(|_| "无法读取本地副本")?;
    let relative = join_rel(folder, &name);
    let mut index = load_index(root)?;
    index.files.insert(
        relative.clone(),
        FileMeta {
            added_at: now_ms(),
            ..FileMeta::default()
        },
    );
    save_index(root, &index)?;
    Ok(FileNode {
        path: relative,
        name,
        folder: folder.to_string(),
        size: metadata.len(),
        modified: modified(&metadata),
        course: course_of(root, folder)?,
        assignment: String::new(),
        activity_id: None,
        tags: Vec::new(),
        note: String::new(),
    })
}

fn unique_target(dir: &Path, name: &str) -> (PathBuf, String) {
    let target = dir.join(name);
    if !target.exists() {
        return (target, name.to_string());
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 2..10_000 {
        let candidate = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let target = dir.join(&candidate);
        if !target.exists() {
            return (target, candidate);
        }
    }
    (dir.join(name), name.to_string())
}

pub fn set_binding(
    root: &Path,
    relative: &str,
    input: BindingInput,
    lock: &Mutex<()>,
) -> Result<FileNode, String> {
    let path = path_of(root, relative)?;
    if !path.is_file() {
        return Err("文件不存在，可能已在访达中移动".into());
    }
    let assignment = input.assignment.trim();
    validate_text(assignment, MAX_NAME, "作业")?;
    validate_text(input.note.trim(), MAX_NOTE, "备注")?;
    if let Some(id) = input.activity_id {
        if !(1..=9_999_999_999).contains(&id) {
            return Err("作业编号无效".into());
        }
    }
    let tags = normalize_tags(input.tags)?;
    let folder = parent_of(relative);
    let course = course_of(root, &folder)?;
    if input.activity_id.is_some() && course.is_none() {
        return Err("绑定作业前，先把文件放进课程文件夹".into());
    }
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let mut index = load_index(root)?;
    let meta = index.files.entry(relative.to_string()).or_default();
    meta.assignment = assignment.to_string();
    meta.activity_id = input.activity_id;
    meta.tags = tags.clone();
    meta.note = input.note.trim().to_string();
    save_index(root, &index)?;
    let metadata = fs::metadata(&path).map_err(|_| "无法读取文件")?;
    Ok(FileNode {
        path: relative.to_string(),
        name: path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
        folder,
        size: metadata.len(),
        modified: modified(&metadata),
        course,
        assignment: assignment.to_string(),
        activity_id: input.activity_id,
        tags,
        note: input.note.trim().to_string(),
    })
}

pub fn delete_file(root: &Path, relative: &str, lock: &Mutex<()>) -> Result<(), String> {
    let path = path_of(root, relative)?;
    if !path.is_file() {
        return Err("文件不存在，可能已在访达中移动".into());
    }
    let _guard = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    fs::remove_file(&path).map_err(|_| "无法删除文件，请检查文件权限")?;
    let mut index = load_index(root)?;
    if index.files.remove(relative).is_some() {
        save_index(root, &index)?;
    }
    Ok(())
}

pub fn absolute(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = path_of(root, relative)?;
    if !path.is_file() {
        return Err("文件不存在，可能已在访达中移动".into());
    }
    Ok(path)
}

pub fn folder_absolute(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let path = path_of(root, relative)?;
    if !path.is_dir() {
        return Err("文件夹不存在".into());
    }
    Ok(path)
}

/// Create the semester/course folder for one synced course, or find the folder
/// the user already has. The folder name is never changed: renaming it in
/// Finder is how the user customises a course name.
pub fn ensure_course_folder(
    root: &Path,
    term: &str,
    course: &CourseMeta,
) -> Result<String, String> {
    validate_name(term)?;
    let term_dir = root.join(term);
    if !term_dir.is_dir() {
        fs::create_dir_all(&term_dir).map_err(|_| "无法创建学期文件夹")?;
    }
    // A course already filed under this semester wins, by stable course number.
    if let Some(existing) = find_course(&term_dir, course)? {
        let mut meta = read_marker(&existing).unwrap_or_else(|| course.clone());
        meta.id = course.id;
        meta.code = course.code.clone();
        meta.subject_code = course.subject_code.clone();
        meta.term = course.term.clone();
        // Keep the original LMS name even if the user renamed the folder.
        meta.name = course.name.clone();
        write_marker(&existing, &meta)?;
        return relative(root, &existing);
    }
    // The user may have moved the folder into another semester by hand; that
    // move wins over the synced term, and no second folder is created.
    if let Some((path, _)) = course_folders(root)?
        .into_iter()
        .find(|(_, meta)| meta.key() == course.key())
    {
        if let Ok(dir) = path_of(root, &path) {
            let mut meta = read_marker(&dir).unwrap_or_else(|| course.clone());
            meta.id = course.id;
            meta.code = course.code.clone();
            meta.subject_code = course.subject_code.clone();
            meta.name = course.name.clone();
            write_marker(&dir, &meta)?;
        }
        return Ok(path);
    }
    let name = unique_folder_name(&term_dir, &course.name);
    let dir = term_dir.join(&name);
    fs::create_dir(&dir).map_err(|_| "无法创建课程文件夹")?;
    let mut meta = course.clone();
    meta.term = term.to_string();
    if let Err(error) = write_marker(&dir, &meta) {
        let _ = fs::remove_dir(&dir);
        return Err(error);
    }
    relative(root, &dir)
}

/// All course folders under the whole workspace, for matching synced courses
/// that were moved to another semester by hand.
pub fn course_folders(root: &Path) -> Result<Vec<(String, CourseMeta)>, String> {
    let mut found = Vec::new();
    for node in tree(root)? {
        if let Some(course) = node.course {
            found.push((node.path, course));
        }
    }
    Ok(found)
}

fn find_course(term_dir: &Path, course: &CourseMeta) -> Result<Option<PathBuf>, String> {
    let entries = match fs::read_dir(term_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(None),
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        if let Some(meta) = read_marker(&entry.path()) {
            if meta.key() == course.key() {
                return Ok(Some(entry.path()));
            }
        }
    }
    Ok(None)
}

fn unique_folder_name(dir: &Path, name: &str) -> String {
    let safe = sanitize(name);
    if !dir.join(&safe).exists() {
        return safe;
    }
    for index in 2..10_000 {
        let candidate = format!("{safe} ({index})");
        if !dir.join(&candidate).exists() {
            return candidate;
        }
    }
    safe
}

fn sanitize(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|value| match value {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            value if value.is_control() => '_',
            value => value,
        })
        .collect();
    let trimmed = cleaned.trim().trim_end_matches('.').trim();
    let limited: String = trimmed.chars().take(MAX_NAME).collect();
    if limited.is_empty() {
        "课程".to_string()
    } else {
        limited
    }
}

fn read_marker(dir: &Path) -> Option<CourseMeta> {
    let bytes = fs::read(dir.join(MARKER)).ok()?;
    if bytes.len() as u64 > MAX_META {
        return None;
    }
    let meta: CourseMeta = serde_json::from_slice(&bytes).ok()?;
    if meta.name.is_empty() && meta.id == 0 {
        return None;
    }
    Some(meta)
}

fn write_marker(dir: &Path, meta: &CourseMeta) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(meta).map_err(|_| "无法编码课程标记")?;
    let temp = dir.join(format!(".{MARKER}.{}.tmp", uuid::Uuid::new_v4().simple()));
    fs::write(&temp, bytes).map_err(|_| "无法写入课程标记")?;
    if fs::rename(&temp, dir.join(MARKER)).is_err() {
        let _ = fs::remove_file(&temp);
        return Err("无法写入课程标记".into());
    }
    Ok(())
}

fn load_index(root: &Path) -> Result<Index, String> {
    let path = root.join("index.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Index {
                version: VERSION,
                files: HashMap::new(),
            })
        }
        Err(_) => return Err("无法读取工作台索引".into()),
    };
    let index: Index = serde_json::from_slice(&bytes).map_err(|_| "工作台索引损坏")?;
    if index.version != VERSION {
        return Err("工作台索引版本不受支持".into());
    }
    Ok(index)
}

fn save_index(root: &Path, index: &Index) -> Result<(), String> {
    fs::create_dir_all(root).map_err(|_| "无法创建工作台目录")?;
    let bytes = serde_json::to_vec_pretty(index).map_err(|_| "无法编码工作台索引")?;
    let temp = root.join(format!(".index-{}.tmp", uuid::Uuid::new_v4().simple()));
    fs::write(&temp, &bytes).map_err(|_| "无法写入工作台索引")?;
    if fs::rename(&temp, root.join("index.json")).is_err() {
        let _ = fs::remove_file(&temp);
        return Err("无法写入工作台索引".into());
    }
    Ok(())
}

/// Version 1 kept hashed copies under `files/` and described them with ids.
/// Move each file to its original name under `未归类/` so nothing is hidden,
/// then keep only the extras. A failed move aborts the whole conversion.
fn migrate(root: &Path) -> Result<(), String> {
    let path = root.join("index.json");
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("无法读取工作台索引".into()),
    };
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| "工作台索引损坏")?;
    if value.get("version").and_then(|value| value.as_u64()) != Some(1) {
        return Ok(());
    }
    let entries = value
        .get("entries")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();
    let unfiled = root.join(UNFILED);
    fs::create_dir_all(&unfiled).map_err(|_| "无法创建未归类文件夹")?;
    let mut files = HashMap::new();
    for entry in entries {
        let Some(id) = entry.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        let name = entry
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("未命名")
            .trim();
        let stored = entry
            .get("storedName")
            .or_else(|| entry.get("stored_name"))
            .and_then(|value| value.as_str())
            .unwrap_or("");
        if name.is_empty() || !valid_stored_name(stored) {
            continue;
        }
        let source = root.join("files").join(stored);
        let target = unfiled.join(name);
        if source.is_file() {
            move_verified(&source, &target)?;
        }
        files.insert(
            join_rel(UNFILED, name),
            FileMeta {
                assignment: entry
                    .get("assignment")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                activity_id: entry.get("activityId").and_then(|value| value.as_u64()),
                tags: entry
                    .get("tags")
                    .and_then(|value| value.as_array())
                    .map(|tags| {
                        tags.iter()
                            .filter_map(|tag| tag.as_str().map(str::to_string))
                            .collect()
                    })
                    .unwrap_or_default(),
                note: entry
                    .get("note")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string(),
                added_at: entry
                    .get("addedAt")
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0),
            },
        );
        let _ = id;
    }
    save_index(
        root,
        &Index {
            version: VERSION,
            files,
        },
    )?;
    let _ = fs::remove_dir(root.join("files"));
    Ok(())
}

fn move_verified(source: &Path, target: &Path) -> Result<(), String> {
    let size = fs::metadata(source).map_err(|_| "无法读取旧文件")?.len();
    let target = if target.exists() {
        let dir = target.parent().unwrap_or(Path::new("."));
        let name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("file");
        unique_target(dir, name).0
    } else {
        target.to_path_buf()
    };
    fs::rename(source, &target).map_err(|_| "无法迁移旧文件")?;
    let moved = fs::metadata(&target).map_err(|_| "迁移后找不到文件")?;
    if moved.len() != size {
        return Err("迁移文件大小不一致".into());
    }
    Ok(())
}

fn valid_stored_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains('/')
        && !name.contains('\\')
        && Path::new(name).file_name().and_then(|value| value.to_str()) == Some(name)
}

/// Resolve a UI path (`学期/课程/文件.pdf`) inside the workspace root. Absolute
/// paths, `..` and hidden components are refused, so a tampered path cannot
/// escape the folder the user chose.
pub fn path_of(root: &Path, relative: &str) -> Result<PathBuf, String> {
    if relative.trim().is_empty() {
        return Ok(root.to_path_buf());
    }
    let path = Path::new(relative);
    if path.is_absolute() {
        return Err("路径无效".into());
    }
    let mut out = root.to_path_buf();
    for component in path.components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_str().ok_or("路径不是有效的 UTF-8")?;
                if part.starts_with('.') {
                    return Err("路径无效".into());
                }
                out.push(part);
            }
            _ => return Err("路径无效".into()),
        }
    }
    Ok(out)
}

fn relative(root: &Path, path: &Path) -> Result<String, String> {
    let stripped = path.strip_prefix(root).map_err(|_| "路径无效")?;
    let parts: Vec<String> = stripped
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();
    Ok(parts.join("/"))
}

fn join_rel(folder: &str, name: &str) -> String {
    if folder.is_empty() {
        name.to_string()
    } else {
        format!("{folder}/{name}")
    }
}

fn parent_of(relative: &str) -> String {
    relative
        .rsplit_once('/')
        .map(|(parent, _)| parent.to_string())
        .unwrap_or_default()
}

fn validate_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("名称不能为空".into());
    }
    if name.starts_with('.') {
        return Err("名称不能以点开头".into());
    }
    if name.contains('/') || name.contains('\\') {
        return Err("名称不能包含斜杠".into());
    }
    validate_text(name, MAX_NAME, "名称")
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

fn modified(metadata: &fs::Metadata) -> i64 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account() -> String {
        uuid::Uuid::new_v4().to_string()
    }

    fn course(id: u64, name: &str) -> CourseMeta {
        CourseMeta {
            id,
            code: format!("202620271{id}"),
            subject_code: format!("COMP{id}"),
            name: name.to_string(),
            term: "2026-2027 秋季学期".into(),
        }
    }

    #[test]
    fn course_folder_keeps_a_renamed_folder() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        fs::create_dir_all(&root).unwrap();
        let rel =
            ensure_course_folder(&root, "2026-2027 秋季学期", &course(1, "计算机组成")).unwrap();
        assert_eq!(rel, "2026-2027 秋季学期/计算机组成");
        let dir = root.join(&rel);
        let renamed = dir.with_file_name("计组");
        fs::rename(&dir, &renamed).unwrap();
        let again =
            ensure_course_folder(&root, "2026-2027 秋季学期", &course(1, "计算机组成")).unwrap();
        assert_eq!(again, "2026-2027 秋季学期/计组");
        assert!(renamed.join(MARKER).is_file());
    }

    #[test]
    fn tree_marks_terms_courses_and_plain_folders() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        fs::create_dir_all(&root).unwrap();
        let course_rel =
            ensure_course_folder(&root, "2026-2027 秋季学期", &course(1, "计算机组成")).unwrap();
        fs::write(root.join(&course_rel).join("a.pdf"), b"x").unwrap();
        fs::create_dir_all(root.join("随便建的")).unwrap();
        let nodes = tree(&root).unwrap();
        let term = nodes
            .iter()
            .find(|node| node.path == "2026-2027 秋季学期")
            .unwrap();
        assert_eq!(term.kind, "term");
        let course = nodes.iter().find(|node| node.path == course_rel).unwrap();
        assert_eq!(course.kind, "course");
        assert_eq!(course.files, 1);
        assert_eq!(course.course.as_ref().unwrap().subject_code, "COMP1");
        let plain = nodes.iter().find(|node| node.path == "随便建的").unwrap();
        assert_eq!(plain.kind, "folder");
    }

    #[test]
    fn import_keeps_names_and_renames_a_collision() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        fs::create_dir_all(root.join("学期/课程")).unwrap();
        let source = data.path().join("作业.pdf");
        fs::write(&source, b"one").unwrap();
        let lock = Mutex::new(());
        let folder = "学期/课程";
        let first = import_files(
            &root,
            folder,
            &[source.to_string_lossy().to_string()],
            &lock,
        );
        assert!(first[0].file.is_some());
        let second = import_files(
            &root,
            folder,
            &[source.to_string_lossy().to_string()],
            &lock,
        );
        let name = second[0].file.as_ref().unwrap().name.clone();
        assert_eq!(name, "作业 (2).pdf");
        assert_eq!(files(&root, folder).unwrap().len(), 2);
    }

    #[test]
    fn binding_needs_a_course_folder_and_round_trips() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        fs::create_dir_all(root.join("随便")).unwrap();
        fs::write(root.join("随便/a.pdf"), b"x").unwrap();
        let lock = Mutex::new(());
        assert!(set_binding(
            &root,
            "随便/a.pdf",
            BindingInput {
                assignment: "第一次作业".into(),
                activity_id: Some(9),
                tags: vec![],
                note: String::new(),
            },
            &lock
        )
        .is_err());
        let course_rel =
            ensure_course_folder(&root, "2026-2027 秋季学期", &course(7, "操作系统")).unwrap();
        fs::write(root.join(&course_rel).join("a.pdf"), b"x").unwrap();
        let file = set_binding(
            &root,
            &format!("{course_rel}/a.pdf"),
            BindingInput {
                assignment: "第一次作业".into(),
                activity_id: Some(9),
                tags: vec!["作业".into()],
                note: "终稿".into(),
            },
            &lock,
        )
        .unwrap();
        assert_eq!(file.activity_id, Some(9));
        assert!(file.course.is_some());
        let listed = files(&root, &course_rel).unwrap();
        assert_eq!(listed[0].assignment, "第一次作业");
    }

    #[test]
    fn paths_cannot_escape_the_root() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        assert!(path_of(&root, "../x").is_err());
        assert!(path_of(&root, "/etc/passwd").is_err());
        assert!(path_of(&root, ".xjtu-course.json").is_err());
        assert!(path_of(&root, "学期/课程/a.pdf").is_ok());
    }

    #[test]
    fn old_flat_library_is_moved_to_unfiled() {
        let data = tempfile::tempdir().unwrap();
        let root = root(data.path(), &account());
        fs::create_dir_all(root.join("files")).unwrap();
        fs::write(root.join("files/abc.pdf"), b"legacy").unwrap();
        fs::write(
            root.join("index.json"),
            r#"{"version":1,"entries":[{"id":"00000000-0000-4000-8000-000000000000","name":"作业.pdf","storedName":"abc.pdf","size":6,"sha256":"00","addedAt":1,"updatedAt":1,"assignment":"第一次作业","note":"旧"}]}"#,
        )
        .unwrap();
        let nodes = tree(&root).unwrap();
        assert!(nodes.iter().any(|node| node.path == "未归类"));
        let listed = files(&root, UNFILED).unwrap();
        assert_eq!(listed[0].name, "作业.pdf");
        assert_eq!(listed[0].assignment, "第一次作业");
        assert!(!root.join("files/abc.pdf").exists());
    }
}
