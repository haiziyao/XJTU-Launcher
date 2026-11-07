//! Two folders, split the way Codex splits settings from its home:
//!
//! ```text
//! ~/.xjtu-launcher/                 settings and app data (like ~/.codex)
//! ├── config.json
//! ├── auth.json
//! ├── storage.json                  which file folder is in use
//! ├── cache/  plugins/  agent/
//! └── state/                        加密账号、模型配置、设备标识……
//!
//! ~/Documents/xjtu-launcher-file/   the user's own files
//! ├── workspace/                    资料库
//! └── download/                     下载
//! ```
//!
//! The hidden home never moves; it sits directly under the user's profile on
//! macOS and Windows. Only the visible `xjtu-launcher-file` folder, which holds
//! the user's own files and nothing else, can be pointed elsewhere in
//! 设置 → 存储. `storage.json` records that choice and lives in the hidden home,
//! never in `AppConfig`: the page saves the whole config on every settings
//! change, so a snapshot taken before a migration would otherwise write the
//! old root back and split the files.
//!
//! Paths are shown as `~/...` so a user name never reaches the screen.
//!
//! Moving the file folder copies first, verifies file and byte counts,
//! switches the recorded root, and only then deletes the old copies. A failure
//! before the switch leaves the old folder untouched.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use tauri::{AppHandle, Manager as _};

/// Hidden folder directly under the user's home, like Codex's `.codex`.
const HOME_DIR: &str = ".xjtu-launcher";
/// Visible folder for the user's own files, by default under Documents.
const FILE_DIR: &str = "xjtu-launcher-file";
const STORE: &str = "storage.json";
const STORE_VERSION: u32 = 2;
/// Staging folder created inside the target while a migration runs.
const STAGING_PREFIX: &str = ".xjtu-migrate-";

#[derive(Clone, Copy)]
struct Item {
    /// Path relative to the folder that holds it.
    name: &'static str,
    label: &'static str,
    directory: bool,
}

/// Only the user's own files move between folders; app data stays in the
/// hidden home and is never touched by a migration.
const FILE_ITEMS: [Item; 2] = [
    Item {
        name: "workspace",
        label: "工作台（资料库）",
        directory: true,
    },
    Item {
        name: "download",
        label: "下载",
        directory: true,
    },
];

/// Names written by builds that kept everything in the Tauri app data folder,
/// and where they live now. Only used once, at startup.
const ADOPT: [(&str, &str); 9] = [
    ("cache", "cache"),
    ("plugins", "plugins"),
    ("agent", "agent"),
    ("campus-accounts.enc", "state/campus-accounts.enc"),
    ("model-profiles.enc", "state/model-profiles.enc"),
    ("device-id", "state/device-id"),
    ("pet-seen.json", "state/pet-seen.json"),
    ("timetable-custom.json", "state/timetable-custom.json"),
    ("mcp.json", "state/mcp.json"),
];

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Store {
    version: u32,
    /// Absolute path chosen by the user; `None` means the default file folder.
    #[serde(default)]
    file_dir: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub name: String,
    pub label: String,
    pub directory: bool,
    pub bytes: u64,
    pub files: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub path: String,
    pub default_path: String,
    pub is_default: bool,
    pub total_bytes: u64,
    pub total_files: u64,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub from: String,
    pub to: String,
    pub total_bytes: u64,
    pub total_files: u64,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub from: String,
    pub to: String,
    pub total_bytes: u64,
    pub total_files: u64,
    /// Labels of the entries that were moved.
    pub moved: Vec<String>,
    /// Non-fatal leftovers, e.g. an old copy that could not be deleted.
    pub warnings: Vec<String>,
}

/// `~/.xjtu-launcher`. Created on demand; the credentials module tightens its
/// permissions to 0700 at startup.
pub fn home(app: &AppHandle) -> Result<PathBuf, String> {
    let home = app
        .path()
        .home_dir()
        .map_err(|_| "无法定位用户主目录")?
        .join(HOME_DIR);
    fs::create_dir_all(&home).map_err(|_| "无法创建 ~/.xjtu-launcher 目录")?;
    Ok(home)
}

/// Settings sit in the launcher folder beside the credentials, not in the
/// system config directory, so every path the interface shows starts with `~`.
pub fn config_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(home(app)?.join("config.json"))
}

/// Shows a path under the user's home as `~/...`. Paths elsewhere stay
/// absolute; the interface must never print the user's account name.
fn display(path: &Path, home: Option<&Path>) -> String {
    if let Some(home) = home {
        if let Ok(rest) = path.strip_prefix(home) {
            if rest.as_os_str().is_empty() {
                return "~".into();
            }
            return format!("~/{}", rest.to_string_lossy().replace('\\', "/"));
        }
    }
    path.to_string_lossy().into_owned()
}

fn user_home(app: &AppHandle) -> Option<PathBuf> {
    app.path().home_dir().ok()
}

/// Turns any path into the `~/...` form the interface shows. Used for paths
/// the user picked in a save dialog as well as the data folder itself.
pub fn display_for(app: &AppHandle, path: &str) -> String {
    display(Path::new(path), user_home(app).as_deref())
}

fn store_path(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(home(app)?.join(STORE))
}

fn load_store(app: &AppHandle) -> Store {
    let Ok(path) = store_path(app) else {
        return Store::default();
    };
    match fs::read(&path) {
        Ok(bytes) => {
            let mut store: Store = serde_json::from_slice(&bytes).unwrap_or_default();
            if store.version == 0 {
                store.version = STORE_VERSION;
            }
            store
        }
        Err(_) => Store::default(),
    }
}

fn save_store(app: &AppHandle, file_dir: Option<&Path>) -> Result<(), String> {
    let store = Store {
        version: STORE_VERSION,
        file_dir: file_dir.map(|path| path.to_string_lossy().into_owned()),
    };
    let bytes = serde_json::to_vec_pretty(&store).map_err(|_| "无法序列化存储设置")?;
    crate::config::write_atomic(&store_path(app)?, &bytes)
}

/// `~/Documents/xjtu-launcher-file`, or a visible folder in the home
/// directory when the system has no Documents folder.
pub fn default_file_root(app: &AppHandle) -> Result<PathBuf, String> {
    if let Ok(documents) = app.path().document_dir() {
        return Ok(documents.join(FILE_DIR));
    }
    Ok(app
        .path()
        .home_dir()
        .map_err(|_| "无法定位用户主目录")?
        .join(FILE_DIR))
}

/// The folder holding the user's own files. A value that is not an absolute
/// path is ignored rather than used, so a damaged store cannot point the app
/// at a relative directory inside the working folder.
pub fn file_root(app: &AppHandle) -> Result<PathBuf, String> {
    let fallback = default_file_root(app)?;
    match load_store(app).file_dir {
        Some(value) => {
            let path = PathBuf::from(&value);
            if value.trim().is_empty() || !path.is_absolute() {
                return Ok(fallback);
            }
            Ok(path)
        }
        None => Ok(fallback),
    }
}

/// The hidden app home `~/.xjtu-launcher`, where settings, credentials,
/// caches, plugins and Agent data live. Never moves.
pub fn root(app: &AppHandle) -> Result<PathBuf, String> {
    home(app)
}

/// A path inside the hidden app home.
pub fn path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let root = root(app)?;
    let path = root.join(name);
    // `state/` is created on demand so a fresh install can write its first
    // small file without every caller remembering to make the folder.
    if let Some(parent) = path.parent() {
        if parent != root.as_path() {
            fs::create_dir_all(parent).map_err(|_| "无法创建数据文件夹")?;
        }
    }
    Ok(path)
}

/// A path inside the visible file folder (`workspace` or `download`). The
/// folder is created on demand so "打开" works on a fresh install.
pub fn file_path(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let root = file_root(app)?;
    fs::create_dir_all(&root).map_err(|_| "无法创建文件文件夹")?;
    Ok(root.join(name))
}

/// Where a save dialog should start. The folder is created on demand; a
/// failure only means the dialog falls back to the system default.
pub fn downloads(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = file_path(app, "download")?;
    fs::create_dir_all(&dir).map_err(|_| "无法创建下载目录")?;
    Ok(dir)
}

/// Only the last path component is kept, so a name from a school server cannot
/// point a save dialog outside the downloads folder.
fn safe_name(name: &str) -> &str {
    Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("下载")
}

/// Turns a suggested file name into a path inside the downloads folder.
pub fn download_target(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    Ok(downloads(app)?.join(safe_name(name)))
}

/// Like [`download_target`], but never returns an existing path: a repeated
/// download becomes `名字 (2).pdf` instead of overwriting the earlier file.
pub fn unique_download_target(app: &AppHandle, name: &str) -> Result<PathBuf, String> {
    let dir = downloads(app)?;
    let name = safe_name(name);
    let target = dir.join(name);
    if !target.exists() {
        return Ok(target);
    }
    let path = Path::new(name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("下载");
    let extension = path.extension().and_then(|value| value.to_str());
    for index in 2..1_000 {
        let candidate = match extension {
            Some(extension) => format!("{stem} ({index}).{extension}"),
            None => format!("{stem} ({index})"),
        };
        let target = dir.join(candidate);
        if !target.exists() {
            return Ok(target);
        }
    }
    Err("下载目录中同名文件太多，请先整理后重试".into())
}

fn measure(path: &Path) -> Result<(u64, u64), String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "无法读取数据文件夹".to_string())?;
    if meta.is_file() {
        return Ok((meta.len(), 1));
    }
    if !meta.is_dir() {
        return Err("数据文件夹中包含无法迁移的符号链接".into());
    }
    let mut bytes = 0;
    let mut files = 0;
    for entry in fs::read_dir(path).map_err(|_| "无法读取数据文件夹".to_string())? {
        let entry = entry.map_err(|_| "无法读取数据文件夹".to_string())?;
        let (child_bytes, child_files) = measure(&entry.path())?;
        bytes += child_bytes;
        files += child_files;
    }
    Ok((bytes, files))
}

fn entries_at(root: &Path) -> Vec<Entry> {
    FILE_ITEMS
        .iter()
        .filter_map(|item| {
            let path = root.join(item.name);
            let (bytes, files) = measure(&path).ok()?;
            Some(Entry {
                name: item.name.into(),
                label: item.label.into(),
                directory: item.directory,
                bytes,
                files,
            })
        })
        .collect()
}

pub fn status(app: &AppHandle) -> Result<Status, String> {
    let root = file_root(app)?;
    let default_path = default_file_root(app)?;
    let home = user_home(app);
    let entries = entries_at(&root);
    Ok(Status {
        is_default: same_path(&root, &default_path),
        path: display(&root, home.as_deref()),
        default_path: display(&default_path, home.as_deref()),
        total_bytes: entries.iter().map(|entry| entry.bytes).sum(),
        total_files: entries.iter().map(|entry| entry.files).sum(),
        entries,
    })
}

fn same_path(left: &Path, right: &Path) -> bool {
    match (fs::canonicalize(left), fs::canonicalize(right)) {
        (Ok(left), Ok(right)) => left == right,
        _ => left == right,
    }
}

/// Resolves a typed path to an existing-or-creatable absolute folder and
/// rejects the cases where a migration would copy data into itself or scatter
/// it across the user's home folder.
fn resolve_target(app: &AppHandle, from: &Path, target: &str) -> Result<PathBuf, String> {
    let raw = target.trim();
    if raw.is_empty() {
        return Err("请选择新的文件文件夹".into());
    }
    // The interface shows folders as `~/...`; accept that form back.
    let user_home_dir = app.path().home_dir().ok();
    let path = match (raw, &user_home_dir) {
        ("~", Some(home)) => home.clone(),
        (value, Some(home)) if value.starts_with("~/") || value.starts_with("~\\") => {
            home.join(&value[2..])
        }
        _ => PathBuf::from(raw),
    };
    if !path.is_absolute() {
        return Err("文件文件夹需要是完整路径".into());
    }
    if path
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("文件文件夹不能包含 ..".into());
    }
    if path.parent().is_none() {
        return Err("不能把整个磁盘根目录作为文件文件夹".into());
    }
    if let Ok(home) = app.path().home_dir() {
        if same_path(&path, &home) {
            return Err("请选择主目录下的子文件夹，而不是主目录本身".into());
        }
    }
    let resolved = canonical_or_join(&path)?;
    if let Ok(settings_home) = home(app) {
        let settings_home = canonical_or_join(&settings_home)?;
        if resolved == settings_home
            || resolved.starts_with(&settings_home)
            || settings_home.starts_with(&resolved)
        {
            return Err("文件文件夹不能放进 ~/.xjtu-launcher".into());
        }
    }
    let current = canonical_or_join(from)?;
    if resolved == current {
        return Err("新位置与当前文件文件夹相同".into());
    }
    // Copying into a folder inside the current root would copy the copy.
    if resolved.starts_with(&current) || current.starts_with(&resolved) {
        return Err("新位置不能与当前文件文件夹互相包含".into());
    }
    if resolved.exists() {
        if !resolved.is_dir() {
            return Err("选择的路径不是文件夹".into());
        }
        if let Some(name) = conflicting_entry(&resolved) {
            return Err(format!("目标文件夹里已经有 {name}，请选择空文件夹或先备份"));
        }
    } else if let Some(parent) = resolved.parent() {
        if !parent.is_dir() {
            return Err("目标文件夹的上一级不存在".into());
        }
    }
    Ok(resolved)
}

fn conflicting_entry(dir: &Path) -> Option<&'static str> {
    FILE_ITEMS
        .iter()
        .find(|item| dir.join(item.name).exists())
        .map(|item| item.label)
}

/// Canonicalises the longest existing ancestor and appends the rest, so a
/// folder that does not exist yet can still be compared with the current root.
fn canonical_or_join(path: &Path) -> Result<PathBuf, String> {
    if let Ok(canonical) = fs::canonicalize(path) {
        return Ok(canonical);
    }
    let mut existing = path.to_path_buf();
    let mut rest = Vec::new();
    while let Some(parent) = existing.parent() {
        if let Some(name) = existing.file_name() {
            rest.push(name.to_os_string());
        }
        existing = parent.to_path_buf();
        if let Ok(canonical) = fs::canonicalize(&existing) {
            let mut resolved = canonical;
            for name in rest.iter().rev() {
                resolved.push(name);
            }
            return Ok(resolved);
        }
    }
    Ok(path.to_path_buf())
}

pub fn preview(app: &AppHandle, target: &str) -> Result<Preview, String> {
    let from = file_root(app)?;
    let to = resolve_target(app, &from, target)?;
    let entries = entries_at(&from);
    if entries.is_empty() {
        return Err("当前文件文件夹还没有可迁移的文件".into());
    }
    let home = user_home(app);
    Ok(Preview {
        from: display(&from, home.as_deref()),
        to: display(&to, home.as_deref()),
        total_bytes: entries.iter().map(|entry| entry.bytes).sum(),
        total_files: entries.iter().map(|entry| entry.files).sum(),
        entries,
    })
}

/// Only the last component, so an error message never prints a full path and
/// therefore never the user's account name.
fn short_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn copy_item(from: &Path, to: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(from).map_err(|_| "无法读取待迁移的数据".to_string())?;
    if meta.is_file() {
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|_| "无法创建目标文件夹")?;
        }
        fs::copy(from, to).map_err(|_| format!("无法复制 {}", short_name(from)))?;
        return Ok(());
    }
    if !meta.is_dir() {
        return Err(format!("{} 是符号链接，无法迁移", short_name(from)));
    }
    fs::create_dir_all(to).map_err(|_| "无法创建目标文件夹")?;
    for entry in fs::read_dir(from).map_err(|_| "无法读取待迁移的数据".to_string())? {
        let entry = entry.map_err(|_| "无法读取待迁移的数据".to_string())?;
        copy_item(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

fn remove_item(path: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(path).map_err(|_| "无法读取旧数据".to_string())?;
    if meta.is_dir() {
        fs::remove_dir_all(path).map_err(|_| "无法删除旧文件夹".to_string())
    } else {
        fs::remove_file(path).map_err(|_| "无法删除旧文件".to_string())
    }
}

/// Copies every owned entry into `to`, verifies the copy, switches the
/// recorded root, then deletes the old copies. `switch` is the step that makes
/// the new root official; a failure there rolls the copies back.
fn migrate_entries(
    from: &Path,
    to: &Path,
    switch: &mut dyn FnMut(&Path) -> Result<(), String>,
) -> Result<Report, String> {
    if let Some(name) = conflicting_entry(to) {
        return Err(format!("目标文件夹里已经有 {name}，请选择空文件夹或先备份"));
    }
    fs::create_dir_all(to).map_err(|_| "无法创建目标文件夹，请检查权限")?;
    let staging = to.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(&staging).map_err(|_| "无法创建目标文件夹，请检查权限")?;

    let existing: Vec<&Item> = FILE_ITEMS
        .iter()
        .filter(|item| from.join(item.name).exists())
        .collect();
    if existing.is_empty() {
        let _ = fs::remove_dir_all(&staging);
        return Err("当前文件文件夹还没有可迁移的文件".into());
    }

    let mut total_bytes = 0;
    let mut total_files = 0;
    let mut moved: Vec<&str> = Vec::new();
    let result = (|| -> Result<(), String> {
        for item in &existing {
            let source = from.join(item.name);
            let target = staging.join(item.name);
            copy_item(&source, &target)?;
            let (source_bytes, source_files) = measure(&source)?;
            let (target_bytes, target_files) = measure(&target)?;
            if (source_bytes, source_files) != (target_bytes, target_files) {
                return Err(format!("{} 复制不完整，已停止迁移", item.label));
            }
            total_bytes += source_bytes;
            total_files += source_files;
        }
        for item in &existing {
            let target = to.join(item.name);
            if target.exists() {
                return Err(format!("目标文件夹里已经有 {}", item.label));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|_| "无法创建目标文件夹")?;
            }
            fs::rename(staging.join(item.name), &target)
                .map_err(|_| format!("无法把 {} 放到新位置", item.label))?;
            moved.push(item.label);
        }
        switch(to)
    })();

    if let Err(error) = result {
        // Undo the copies; the old folder has not been touched yet.
        for item in &existing {
            let _ = remove_item(&to.join(item.name));
        }
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let _ = fs::remove_dir_all(&staging);

    let mut warnings = Vec::new();
    for item in &existing {
        if let Err(error) = remove_item(&from.join(item.name)) {
            warnings.push(format!("{}：{error}；旧文件仍留在原位置", item.label));
        }
    }
    Ok(Report {
        from: from.to_string_lossy().into_owned(),
        to: to.to_string_lossy().into_owned(),
        total_bytes,
        total_files,
        moved: moved.into_iter().map(str::to_string).collect(),
        warnings,
    })
}

pub fn migrate(app: &AppHandle, target: &str) -> Result<Report, String> {
    let from = file_root(app)?;
    let to = resolve_target(app, &from, target)?;
    let default_root = default_file_root(app)?;
    let mut switch = |to: &Path| -> Result<(), String> {
        // The default location stays dynamic, so choosing it again stores
        // nothing rather than pinning the path that a future version changes.
        let value = if same_path(to, &default_root) {
            None
        } else {
            Some(to)
        };
        save_store(app, value)
    };
    let mut report = migrate_entries(&from, &to, &mut switch)?;
    let home = user_home(app);
    report.from = display(Path::new(&report.from), home.as_deref());
    report.to = display(Path::new(&report.to), home.as_deref());
    Ok(report)
}

/// One-time move from older layouts: builds kept everything under the system
/// application folder (`~/Library/Application Support/...` on macOS), and the
/// build before this one briefly kept the 资料库 inside `~/.xjtu-launcher`.
///
/// Settings move into `~/.xjtu-launcher`, app data into its subfolders, and
/// the 资料库 and downloads into the visible file folder. Nothing overwrites
/// an existing destination, so an interrupted move cannot lose either side.
pub fn adopt_legacy(app: &AppHandle) -> Result<(), String> {
    let Ok(home) = home(app) else {
        return Ok(());
    };
    let Ok(old) = app.path().app_config_dir() else {
        return Ok(());
    };
    for name in ["config.json", STORE] {
        adopt_file(&old.join(name), &home.join(name));
    }
    let Ok(files) = file_root(app) else {
        return Ok(());
    };

    // 资料库 used to be `library/`, downloads `downloads/`.
    adopt_entry(&home.join("library"), &files.join("workspace"));
    adopt_entry(&home.join("downloads"), &files.join("download"));
    // Older builds wrote MCP settings straight into the hidden home.
    adopt_entry(&home.join("mcp.json"), &home.join("state/mcp.json"));

    let Ok(old_data) = app.path().app_data_dir() else {
        return Ok(());
    };
    if same_path(&old_data, &home) {
        return Ok(());
    }
    for (legacy, name) in ADOPT {
        adopt_entry(&old_data.join(legacy), &home.join(name));
    }
    adopt_entry(&old_data.join("library"), &files.join("workspace"));
    adopt_entry(&old_data.join("downloads"), &files.join("download"));
    Ok(())
}

/// Copies one small file, checks its length, renames it into place and only
/// then removes the old copy.
fn adopt_file(source: &Path, target: &Path) {
    if !source.is_file() || target.exists() {
        return;
    }
    let Ok(meta) = fs::metadata(source) else {
        return;
    };
    let staging =
        target.with_file_name(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4().simple()));
    if fs::copy(source, &staging).is_err() {
        let _ = fs::remove_file(&staging);
        return;
    }
    let copied = fs::metadata(&staging).is_ok_and(|staged| staged.len() == meta.len());
    if copied && fs::rename(&staging, target).is_ok() {
        let _ = fs::remove_file(source);
    } else {
        let _ = fs::remove_file(&staging);
    }
}

/// Same for one data entry, which may be a folder.
fn adopt_entry(source: &Path, target: &Path) {
    if !source.exists() || target.exists() {
        return;
    }
    let Some(parent) = target.parent() else {
        return;
    };
    if fs::create_dir_all(parent).is_err() {
        return;
    }
    let staging = parent.join(format!("{STAGING_PREFIX}{}", uuid::Uuid::new_v4().simple()));
    let copied = copy_item(source, &staging).and_then(|_| {
        let source_counts = measure(source)?;
        let staging_counts = measure(&staging)?;
        if source_counts == staging_counts {
            Ok(())
        } else {
            Err("复制不完整".to_string())
        }
    });
    if copied.is_ok() && fs::rename(&staging, target).is_ok() {
        let _ = remove_item(source);
    } else {
        let _ = remove_item(&staging);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, text: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, text).unwrap();
    }

    fn fixture(root: &Path) {
        write(&root.join("workspace/u1/index.json"), "[]");
        write(&root.join("workspace/u1/files/x.bin"), "12345");
        write(&root.join("download/a.pdf"), "123");
        write(&root.join("config.json"), "{}");
    }

    #[test]
    fn migration_moves_owned_entries_and_keeps_foreign_files() {
        let temp = tempfile::tempdir().unwrap();
        let from = temp.path().join("old");
        let to = temp.path().join("new");
        fixture(&from);

        let mut switched = false;
        let report = migrate_entries(&from, &to, &mut |path| {
            assert_eq!(path, to);
            switched = true;
            Ok(())
        })
        .unwrap();

        assert!(switched);
        assert_eq!(report.total_files, 3);
        assert!(to.join("workspace/u1/files/x.bin").is_file());
        assert!(to.join("download/a.pdf").is_file());
        // The old folder keeps files the launcher does not own.
        assert!(from.join("config.json").is_file());
        assert!(!from.join("workspace").exists());
        assert!(!from.join("download").exists());
        assert!(!to.join("config.json").exists());
    }

    #[test]
    fn a_failed_switch_keeps_the_old_folder_and_removes_the_copies() {
        let temp = tempfile::tempdir().unwrap();
        let from = temp.path().join("old");
        let to = temp.path().join("new");
        fixture(&from);

        let error = migrate_entries(&from, &to, &mut |_| Err("配置写入失败".into())).unwrap_err();
        assert_eq!(error, "配置写入失败");
        assert!(from.join("workspace/u1/files/x.bin").is_file());
        assert!(from.join("download/a.pdf").is_file());
        assert!(!to.join("workspace").exists());
        assert!(!to.join("download").exists());
    }

    #[test]
    fn a_folder_with_launcher_data_cannot_be_reused_as_target() {
        let temp = tempfile::tempdir().unwrap();
        let from = temp.path().join("old");
        let to = temp.path().join("new");
        fixture(&from);
        write(&to.join("workspace/other.json"), "{}");
        let error = migrate_entries(&from, &to, &mut |_| Ok(())).unwrap_err();
        assert!(error.contains("工作台"), "{error}");
        assert!(from.join("workspace/u1/index.json").is_file());
    }

    #[test]
    fn paths_under_home_are_shown_with_a_tilde() {
        let home = Path::new("/Users/someone");
        assert_eq!(
            display(Path::new("/Users/someone/.xjtu-launcher"), Some(home)),
            "~/.xjtu-launcher"
        );
        assert_eq!(
            display(Path::new("/Users/someone/Documents/data"), Some(home)),
            "~/Documents/data"
        );
        // Anything outside the home folder stays absolute.
        assert_eq!(
            display(Path::new("/Volumes/Data/xjtu"), Some(home)),
            "/Volumes/Data/xjtu"
        );
    }

    #[test]
    fn adoption_moves_flat_state_into_the_state_folder() {
        let temp = tempfile::tempdir().unwrap();
        let old = temp.path().join("old");
        let home = temp.path().join("home");
        write(&old.join("device-id"), "abc");
        write(&old.join("cache/a.json"), "{\"a\":1}");

        adopt_entry(&old.join("device-id"), &home.join("state/device-id"));
        adopt_entry(&old.join("cache"), &home.join("cache"));

        assert_eq!(
            fs::read_to_string(home.join("state/device-id")).unwrap(),
            "abc"
        );
        assert!(home.join("cache/a.json").is_file());
        assert!(!old.join("device-id").exists());
        assert!(!old.join("cache").exists());
    }

    #[test]
    fn download_names_cannot_escape_the_folder() {
        assert_eq!(safe_name("../../etc/passwd"), "passwd");
        assert_eq!(safe_name("a/b/作业.pdf"), "作业.pdf");
        assert_eq!(safe_name("   "), "下载");
        assert_eq!(safe_name("附件.docx"), "附件.docx");
    }
}
