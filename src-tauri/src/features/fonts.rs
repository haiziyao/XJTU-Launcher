//! App-only font library.
//!
//! The font plugin used to write a family name into CSS and hope the system had
//! that family installed. A missing family silently fell back, which is why
//! 霞鹜文楷 never appeared on Windows. Files here are downloaded from the
//! official release pages, verified against a fixed SHA-256, and loaded into
//! the webview with `FontFace`; nothing is installed into the operating system
//! and no administrator rights are needed.
//!
//! Fonts live in `~/.xjtu-launcher/resources/fonts/` with an `index.json`.
//! Downloads stream to a temporary file with progress events, and the index is
//! only updated after the final font (extracted from the official 7z package
//! for Sarasa Gothic) has passed its hash check.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, State, WebviewWindow};

use crate::{storage, AppState};

const FONT_DIR: &str = "resources/fonts";
const INDEX: &str = "index.json";
const MAX_LOCAL_BYTES: u64 = 100 * 1024 * 1024;
const MAX_FAMILY_CHARS: usize = 64;

/// Where the actual font bytes come from. Sarasa Gothic only publishes 7z
/// bundles per region, so the archive is extracted in-process.
#[derive(Clone, Copy)]
enum Package {
    Font,
    SevenZ {
        entry: &'static str,
        sha256: &'static str,
        size: u64,
    },
}

struct CatalogFont {
    id: &'static str,
    name: &'static str,
    family: &'static str,
    style: &'static str,
    license: &'static str,
    source: &'static str,
    url: &'static str,
    size: u64,
    sha256: &'static str,
    format: &'static str,
    package: Package,
}

/// Fixed catalog. The download command accepts only these ids, so the remote
/// URL and expected hash cannot be supplied by the caller.
const CATALOG: [CatalogFont; 4] = [
    CatalogFont {
        id: "wenkai",
        name: "霞鹜文楷",
        family: "霞鹜文楷",
        style: "楷体",
        license: "OFL 1.1",
        source: "https://github.com/lxgw/LxgwWenKai",
        url: "https://github.com/lxgw/LxgwWenKai/releases/download/v1.530/LXGWWenKai-Regular.ttf",
        size: 25_928_940,
        sha256: "2d73eea94ad808999d90c0000c239a84a33b09d4bc6ab6c5b8bbf1eda8a7fc8b",
        format: "ttf",
        package: Package::Font,
    },
    CatalogFont {
        id: "neo-xihei",
        name: "霞鹜新晰黑",
        family: "霞鹜新晰黑",
        style: "黑体",
        license: "OFL 1.1",
        source: "https://github.com/lxgw/LxgwNeoXiHei",
        url: "https://github.com/lxgw/LxgwNeoXiHei/releases/download/v1.308/LXGWNeoXiHei.ttf",
        size: 7_473_598,
        sha256: "80415d3666d9f51cf42ecf82c5350488e6b3f282ab46a3b90dcaa5e212564ecc",
        format: "ttf",
        package: Package::Font,
    },
    CatalogFont {
        id: "noto-serif",
        name: "思源宋体",
        family: "思源宋体",
        style: "宋体",
        license: "OFL 1.1",
        source: "https://github.com/notofonts/noto-cjk",
        url: "https://raw.githubusercontent.com/notofonts/noto-cjk/f8d157532fbfaeda587e826d4cd5b21a49186f7c/Serif/SubsetOTF/SC/NotoSerifSC-Regular.otf",
        size: 11_625_800,
        sha256: "e8f396decc1f0963a016a989c3d8852e863d1350996f573860a80767c83a1cd3",
        format: "otf",
        package: Package::Font,
    },
    CatalogFont {
        id: "sarasa-gothic",
        name: "更纱黑体",
        family: "更纱黑体",
        style: "黑体",
        license: "OFL 1.1",
        source: "https://github.com/be5invis/Sarasa-Gothic",
        url: "https://github.com/be5invis/Sarasa-Gothic/releases/download/v1.0.43/SarasaGothicSC-TTF-1.0.43.7z",
        size: 62_863_304,
        sha256: "787596fa5483da821e1951bd61b60a0e217426b97bca8a02cd47749f8d2e51eb",
        format: "ttf",
        package: Package::SevenZ {
            entry: "SarasaGothicSC-Regular.ttf",
            sha256: "de65a1791c1dc9971f021545c15653ea46a86a94ba6284bd70f12e7242b39687",
            size: 24_047_784,
        },
    },
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstalledFont {
    pub id: String,
    pub name: String,
    /// The family name handed to `FontFace` and written into plugin settings.
    pub family: String,
    pub file: String,
    pub format: String,
    /// `catalog` or `local`.
    pub source: String,
    pub size: u64,
    pub sha256: String,
    pub added_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogView {
    id: &'static str,
    name: &'static str,
    family: &'static str,
    style: &'static str,
    license: &'static str,
    source_url: &'static str,
    download_url: &'static str,
    size: u64,
    format: &'static str,
    package: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryState {
    catalog: Vec<CatalogView>,
    installed: Vec<InstalledFont>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FontProgress {
    id: String,
    phase: &'static str,
    received: u64,
    total: u64,
}

#[derive(Default, Serialize, Deserialize)]
struct Index {
    #[serde(default)]
    fonts: Vec<InstalledFont>,
}

/// Download tokens and the index write lock. Cancellation is per font id; a
/// second download of the same id is refused rather than started twice.
#[derive(Default)]
pub struct Manager {
    index: Mutex<()>,
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

struct ActiveDownload<'a> {
    manager: &'a Manager,
    id: &'a str,
}

impl Drop for ActiveDownload<'_> {
    fn drop(&mut self) {
        self.manager.finish(self.id);
    }
}

impl Manager {
    fn begin(&self, id: &str) -> Result<Arc<AtomicBool>, String> {
        let mut cancels = lock(&self.cancels);
        if cancels.contains_key(id) {
            return Err("字体正在下载".into());
        }
        let token = Arc::new(AtomicBool::new(false));
        cancels.insert(id.to_string(), token.clone());
        Ok(token)
    }

    fn finish(&self, id: &str) {
        lock(&self.cancels).remove(id);
    }

    fn cancel(&self, id: &str) -> bool {
        match lock(&self.cancels).get(id) {
            Some(token) => {
                token.store(true, Ordering::Relaxed);
                true
            }
            None => false,
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn font_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = storage::path(app, FONT_DIR)?;
    std::fs::create_dir_all(&dir).map_err(|_| "无法创建字体文件夹")?;
    Ok(dir)
}

fn load_index(app: &AppHandle) -> Result<Index, String> {
    let path = font_dir(app)?.join(INDEX);
    match std::fs::read(&path) {
        Ok(bytes) => Ok(serde_json::from_slice(&bytes).unwrap_or_default()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Index::default()),
        Err(_) => Err("无法读取字体索引".into()),
    }
}

fn save_index(app: &AppHandle, index: &Index) -> Result<(), String> {
    let path = font_dir(app)?.join(INDEX);
    let bytes = serde_json::to_vec_pretty(index).map_err(|_| "无法保存字体索引")?;
    // The file is small and rewritten only after a download or removal; writing
    // it directly is simpler than juggling a Windows rename-over-existing.
    std::fs::write(path, bytes).map_err(|_| "无法保存字体索引".into())
}

fn catalog_views() -> Vec<CatalogView> {
    CATALOG
        .iter()
        .map(|font| CatalogView {
            id: font.id,
            name: font.name,
            family: font.family,
            style: font.style,
            license: font.license,
            source_url: font.source,
            download_url: font.url,
            size: font.size,
            format: font.format,
            package: match font.package {
                Package::Font => "font",
                Package::SevenZ { .. } => "7z",
            },
        })
        .collect()
}

/// A file name from the index may only refer to a direct child of the font
/// folder. `..`, separators and dotfiles never pass.
fn safe_file_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('.') && !name.contains('/') && !name.contains('\\')
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn emit_progress(app: &AppHandle, id: &str, phase: &'static str, received: u64, total: u64) {
    let _ = app.emit(
        "font-download",
        FontProgress {
            id: id.to_string(),
            phase,
            received,
            total,
        },
    );
}

#[tauri::command]
pub fn fonts_state(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
) -> Result<LibraryState, String> {
    crate::main_only(&window)?;
    let _guard = lock(&state.fonts.index);
    let mut index = load_index(&app)?;
    let dir = font_dir(&app)?;
    let before = index.fonts.len();
    index
        .fonts
        .retain(|font| safe_file_name(&font.file) && dir.join(&font.file).is_file());
    if index.fonts.len() != before {
        let _ = save_index(&app, &index);
    }
    Ok(LibraryState {
        catalog: catalog_views(),
        installed: index.fonts,
    })
}

#[tauri::command]
pub async fn font_download(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<InstalledFont, String> {
    crate::main_only(&window)?;
    let entry = CATALOG
        .iter()
        .find(|font| font.id == id)
        .ok_or("未知字体")?;

    {
        let _guard = lock(&state.fonts.index);
        if let Some(existing) = load_index(&app)?
            .fonts
            .into_iter()
            .find(|font| font.id == entry.id)
        {
            return Ok(existing);
        }
    }

    let token = state.fonts.begin(entry.id)?;
    let _active = ActiveDownload {
        manager: &state.fonts,
        id: entry.id,
    };
    let dir = font_dir(&app)?;
    download_font(&app, &state.fonts, &dir, entry, &token).await
}

#[tauri::command]
pub fn font_download_cancel(
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<bool, String> {
    crate::main_only(&window)?;
    Ok(state.fonts.cancel(&id))
}

#[tauri::command]
pub async fn font_remove(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), String> {
    crate::main_only(&window)?;
    let removed = {
        let _guard = lock(&state.fonts.index);
        let mut index = load_index(&app)?;
        let Some(position) = index.fonts.iter().position(|font| font.id == id) else {
            return Err("字体未安装".into());
        };
        let font = index.fonts.remove(position);
        save_index(&app, &index)?;
        font
    };

    if safe_file_name(&removed.file) {
        let path = font_dir(&app)?.join(&removed.file);
        match tokio::fs::remove_file(&path).await {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("字体已移除，但文件无法删除".into()),
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn font_data(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    id: String,
) -> Result<tauri::ipc::Response, String> {
    crate::main_only(&window)?;
    let path = {
        let _guard = lock(&state.fonts.index);
        let index = load_index(&app)?;
        let font = index
            .fonts
            .iter()
            .find(|font| font.id == id)
            .ok_or("字体未安装")?;
        if !safe_file_name(&font.file) {
            return Err("字体文件无效".into());
        }
        font_dir(&app)?.join(&font.file)
    };
    if !path.is_file() {
        return Err("字体文件不存在".into());
    }
    let bytes = tokio::fs::read(path)
        .await
        .map_err(|_| "无法读取字体文件")?;
    Ok(tauri::ipc::Response::new(bytes))
}

#[tauri::command]
pub async fn font_add_local(
    app: AppHandle,
    window: WebviewWindow,
    state: State<'_, AppState>,
    path: String,
) -> Result<InstalledFont, String> {
    crate::main_only(&window)?;
    let source = PathBuf::from(path);
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .filter(|value| matches!(value.as_str(), "ttf" | "otf" | "woff" | "woff2"))
        .ok_or("仅支持 .ttf、.otf、.woff、.woff2 字体文件")?;
    let metadata = tokio::fs::metadata(&source)
        .await
        .map_err(|_| "无法读取字体文件")?;
    if !metadata.is_file() {
        return Err("请选择字体文件".into());
    }
    if metadata.len() == 0 {
        return Err("字体文件是空的".into());
    }
    if metadata.len() > MAX_LOCAL_BYTES {
        return Err("字体文件过大（上限 100 MB）".into());
    }
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("本地字体")
        .to_string();

    let bytes = tokio::fs::read(&source)
        .await
        .map_err(|_| "无法读取字体文件")?;
    let has_ttf_names = matches!(extension.as_str(), "ttf" | "otf");
    let (bytes, sha256, parsed_family) =
        tokio::task::spawn_blocking(move || -> Result<_, String> {
            let family = if has_ttf_names {
                font_family(&bytes).map_err(|_| "无法识别字体文件")?
            } else {
                None
            };
            let sha256 = sha256_hex(&bytes);
            Ok((bytes, sha256, family))
        })
        .await
        .map_err(|_| "无法识别字体文件")??;

    let family = sanitize_family(parsed_family.as_deref().unwrap_or(&stem));
    if family.is_empty() {
        return Err("无法确定字体名称".into());
    }

    {
        let _guard = lock(&state.fonts.index);
        if let Some(existing) = load_index(&app)?
            .fonts
            .into_iter()
            .find(|font| font.sha256 == sha256)
        {
            return Ok(existing);
        }
    }

    let dir = font_dir(&app)?;
    let file = format!("local-{}.{}", uuid::Uuid::new_v4().simple(), extension);
    let new_file = file.clone();
    let final_path = dir.join(&file);
    tokio::fs::write(&final_path, &bytes)
        .await
        .map_err(|_| "无法保存字体文件")?;

    let mut font = InstalledFont {
        id: format!("local-{}", uuid::Uuid::new_v4().simple()),
        name: family.clone(),
        family,
        file,
        format: extension,
        source: "local".into(),
        size: bytes.len() as u64,
        sha256,
        added_at: now(),
    };

    let installed = {
        let _guard = lock(&state.fonts.index);
        let mut index = load_index(&app)?;
        if let Some(existing) = index
            .fonts
            .iter()
            .find(|existing| existing.sha256 == font.sha256)
        {
            Ok(existing.clone())
        } else {
            let unique = unique_family(&index, &font.family);
            font.name = unique.clone();
            font.family = unique;
            index.fonts.push(font.clone());
            save_index(&app, &index).map(|()| font)
        }
    };

    match installed {
        Ok(existing) => {
            if existing.file != new_file {
                let _ = tokio::fs::remove_file(&final_path).await;
            }
            Ok(existing)
        }
        Err(error) => {
            let _ = tokio::fs::remove_file(&final_path).await;
            Err(error)
        }
    }
}

async fn download_font(
    app: &AppHandle,
    manager: &Manager,
    dir: &Path,
    entry: &CatalogFont,
    token: &AtomicBool,
) -> Result<InstalledFont, String> {
    let temp = dir.join(format!(
        ".download-{}-{}.part",
        entry.id,
        uuid::Uuid::new_v4().simple()
    ));
    let result = download_and_install(app, manager, dir, entry, token, &temp).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&temp).await;
    }
    result
}

async fn download_and_install(
    app: &AppHandle,
    manager: &Manager,
    dir: &Path,
    entry: &CatalogFont,
    token: &AtomicBool,
    temp: &Path,
) -> Result<InstalledFont, String> {
    download_file(app, entry, token, temp).await?;
    if token.load(Ordering::Relaxed) {
        return Err("已取消下载".into());
    }

    let (file, size, sha256) = match entry.package {
        Package::Font => {
            let file = format!("{}.{}", entry.id, entry.format);
            replace_file(temp, &dir.join(&file)).await?;
            (file, entry.size, entry.sha256.to_string())
        }
        Package::SevenZ {
            entry: archive_entry,
            sha256: expected_sha,
            size: expected_size,
        } => {
            emit_progress(app, entry.id, "extract", 0, expected_size);
            let archive = temp.to_path_buf();
            let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>, String> {
                let mut reader =
                    sevenz_rust2::ArchiveReader::open(&archive, sevenz_rust2::Password::empty())
                        .map_err(|_| "无法打开字体安装包".to_string())?;
                reader
                    .read_file(archive_entry)
                    .map_err(|_| "无法解压字体安装包".to_string())
            })
            .await
            .map_err(|_| "字体解压任务失败")??;

            if token.load(Ordering::Relaxed) {
                return Err("已取消下载".into());
            }
            if bytes.len() as u64 != expected_size {
                return Err("解压后的字体大小不符".into());
            }
            let actual = sha256_hex(&bytes);
            if actual != expected_sha {
                return Err("解压后的字体校验失败".into());
            }
            emit_progress(app, entry.id, "extract", expected_size, expected_size);

            let file = format!("{}.{}", entry.id, entry.format);
            write_atomic(&dir.join(&file), &bytes).await?;
            let _ = tokio::fs::remove_file(temp).await;
            (file, expected_size, expected_sha.to_string())
        }
    };

    let font = InstalledFont {
        id: entry.id.to_string(),
        name: entry.name.to_string(),
        family: entry.family.to_string(),
        file,
        format: entry.format.to_string(),
        source: "catalog".into(),
        size,
        sha256,
        added_at: now(),
    };

    let _guard = lock(&manager.index);
    let mut index = load_index(app)?;
    if let Some(existing) = index.fonts.iter().find(|existing| existing.id == font.id) {
        return Ok(existing.clone());
    }
    index.fonts.push(font.clone());
    save_index(app, &index)?;
    Ok(font)
}

async fn download_file(
    app: &AppHandle,
    entry: &CatalogFont,
    token: &AtomicBool,
    destination: &Path,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(20))
        .timeout(Duration::from_secs(900))
        .user_agent("XJTU-Launcher")
        .build()
        .map_err(|_| "无法创建下载请求")?;
    let mut response = client
        .get(entry.url)
        .send()
        .await
        .map_err(|_| "字体下载失败，请检查网络")?;
    if !response.status().is_success() {
        return Err(format!(
            "字体下载失败（HTTP {}）",
            response.status().as_u16()
        ));
    }

    let mut file = tokio::fs::File::create(destination)
        .await
        .map_err(|_| "无法创建字体临时文件")?;
    let mut hasher = Sha256::new();
    let mut received = 0u64;
    let mut last_emit = Instant::now();
    emit_progress(app, entry.id, "download", 0, entry.size);

    while let Some(chunk) = response.chunk().await.map_err(|_| "字体下载中断，请重试")? {
        if token.load(Ordering::Relaxed) {
            return Err("已取消下载".into());
        }
        received = received.saturating_add(chunk.len() as u64);
        if received > entry.size {
            return Err("字体文件大小不符".into());
        }
        hasher.update(&chunk);
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|_| "无法写入字体文件")?;
        if last_emit.elapsed() >= Duration::from_millis(150) {
            emit_progress(app, entry.id, "download", received, entry.size);
            last_emit = Instant::now();
        }
    }
    tokio::io::AsyncWriteExt::flush(&mut file)
        .await
        .map_err(|_| "无法写入字体文件")?;
    drop(file);

    if received != entry.size {
        return Err("字体文件大小不符".into());
    }
    let actual = hex::encode(hasher.finalize());
    if actual != entry.sha256 {
        return Err("字体文件校验失败".into());
    }
    emit_progress(app, entry.id, "download", entry.size, entry.size);
    Ok(())
}

async fn replace_file(source: &Path, target: &Path) -> Result<(), String> {
    match tokio::fs::remove_file(target).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => return Err("无法替换旧的字体文件".into()),
    }
    tokio::fs::rename(source, target)
        .await
        .map_err(|_| "无法保存字体文件".into())
}

async fn write_atomic(target: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = target.with_file_name(format!(".write-{}.tmp", uuid::Uuid::new_v4().simple()));
    tokio::fs::write(&temp, bytes)
        .await
        .map_err(|_| "无法写入字体文件")?;
    if let Err(error) = replace_file(&temp, target).await {
        let _ = tokio::fs::remove_file(&temp).await;
        return Err(error);
    }
    Ok(())
}

fn font_family(bytes: &[u8]) -> Result<Option<String>, ttf_parser::FaceParsingError> {
    let face = ttf_parser::Face::parse(bytes, 0)?;
    let mut best: Option<(u8, String)> = None;
    for name in face.names() {
        let base = match name.name_id {
            ttf_parser::name_id::TYPOGRAPHIC_FAMILY => 2,
            ttf_parser::name_id::FAMILY => 1,
            _ => continue,
        };
        let Some(value) = name.to_string() else {
            continue;
        };
        let value = sanitize_family(&value);
        if value.is_empty() {
            continue;
        }
        // Prefer a typographic family, then a name written in the font's own
        // (usually Chinese) script over an ASCII alias.
        let rank = base + u8::from(!value.is_ascii());
        if best.as_ref().is_none_or(|(current, _)| rank > *current) {
            best = Some((rank, value));
        }
    }
    Ok(best.map(|(_, value)| value))
}

/// Matches `plugin_settings::valid_family`, so a name that survives import is
/// also accepted when the page saves it back. Always quoted in CSS.
fn sanitize_family(value: &str) -> String {
    let mut result = String::new();
    let mut pending_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            pending_space = !result.is_empty();
            continue;
        }
        if !(ch.is_alphanumeric() || matches!(ch, '-' | '_' | '.')) {
            continue;
        }
        if pending_space && result.chars().count() < MAX_FAMILY_CHARS {
            result.push(' ');
        }
        pending_space = false;
        if result.chars().count() >= MAX_FAMILY_CHARS {
            break;
        }
        result.push(ch);
    }
    result.trim().to_string()
}

fn unique_family(index: &Index, base: &str) -> String {
    if !index.fonts.iter().any(|font| font.family == base) {
        return base.to_string();
    }
    for suffix in 2..=99 {
        let candidate = format!("{base} {suffix}");
        if !index.fonts.iter().any(|font| font.family == candidate) {
            return candidate;
        }
    }
    format!("{base} {}", now())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_complete_and_https_only() {
        assert_eq!(CATALOG.len(), 4);
        for entry in &CATALOG {
            assert!(entry.url.starts_with("https://"));
            assert!(entry.source.starts_with("https://"));
            assert!(entry.size > 0);
            assert_eq!(entry.sha256.len(), 64);
            assert!(entry.sha256.chars().all(|ch| ch.is_ascii_hexdigit()));
            if let Package::SevenZ {
                sha256,
                size,
                entry,
                ..
            } = entry.package
            {
                assert!(entry.ends_with(".ttf"));
                assert_eq!(sha256.len(), 64);
                assert!(size > 0);
            }
        }
        let mut ids = CATALOG.iter().map(|entry| entry.id).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CATALOG.len());
    }

    #[test]
    fn file_names_stay_inside_the_font_folder() {
        assert!(safe_file_name("wenkai.ttf"));
        assert!(safe_file_name("local-abc.otf"));
        assert!(!safe_file_name(""));
        assert!(!safe_file_name("../wenkai.ttf"));
        assert!(!safe_file_name("nested/wenkai.ttf"));
        assert!(!safe_file_name("nested\\wenkai.ttf"));
        assert!(!safe_file_name(".hidden"));
    }

    #[test]
    fn family_names_use_the_same_character_set_as_plugin_settings() {
        assert_eq!(sanitize_family("  霞鹜 文楷  "), "霞鹜 文楷");
        assert_eq!(sanitize_family("Noto Serif SC"), "Noto Serif SC");
        assert_eq!(sanitize_family("A\"B\\C,D"), "ABCD");
        assert_eq!(sanitize_family("   "), "");
    }

    #[test]
    fn duplicate_families_get_a_suffix() {
        let mut index = Index::default();
        index.fonts.push(InstalledFont {
            id: "local-1".into(),
            name: "霞鹜文楷".into(),
            family: "霞鹜文楷".into(),
            file: "a.ttf".into(),
            format: "ttf".into(),
            source: "local".into(),
            size: 1,
            sha256: "a".into(),
            added_at: 0,
        });
        assert_eq!(unique_family(&index, "霞鹜文楷"), "霞鹜文楷 2");
        assert_eq!(unique_family(&index, "思源宋体"), "思源宋体");
    }

    #[test]
    fn sha256_matches_the_known_digest() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
