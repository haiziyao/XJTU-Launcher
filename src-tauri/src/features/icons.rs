//! User-supplied entry icons.
//!
//! ```text
//! ~/.xjtu-launcher/resources/icons/
//! ├── icons.json     entry id → image file, relative to this folder
//! ├── home.png
//! └── campus/todo.png
//! ```
//!
//! ```json
//! { "version": 1, "map": { "nav.home": "home.png" } }
//! ```
//!
//! Keys are stable ids (`nav.*`, `plugin.<id>`, `campus.*`), never labels, so
//! renaming an entry does not break a mapping. The app writes the file once as
//! a self-documenting template and never touches it again: editing it by hand
//! is the supported interface. A missing or broken entry only drops that one
//! icon and leaves the others alone; the entry falls back to its built-in
//! glyph, so a typo can never stop the app from rendering.
//!
//! Images are decoded and squared to 96 px on read, which keeps a 4 MB photo
//! from reaching the webview, and normalises formats the webview cannot show.
use base64::{engine::general_purpose::STANDARD, Engine};
use image::{imageops::FilterType, ImageFormat, ImageReader};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Cursor,
    path::{Component, Path},
};
use tauri::AppHandle;

use crate::storage;

const FOLDER: &str = "resources/icons";
const MAP_FILE: &str = "icons.json";
const VERSION: u32 = 1;
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_SIDE: u32 = 4096;
/// Sidebar icons draw at 20 logical pixels; 96 covers 2x and 3x screens.
const OUTPUT_SIDE: u32 = 96;

/// Ids the app understands today. The template lists them with empty values so
/// the file explains itself; empty (like a missing key) means "built-in glyph".
const KNOWN_IDS: [&str; 28] = [
    "nav.home",
    "nav.agent",
    "nav.community",
    "nav.market",
    "nav.settings",
    "plugin.cn.xjtu.campus",
    "plugin.cn.xjtu.campus-reservations",
    "plugin.cn.xjtu.campus-profile",
    "plugin.cn.xjtu.timetable",
    "plugin.cn.xjtu.home-theme",
    "plugin.cn.xjtu.agent-settings",
    "campus.todo",
    "campus.notices",
    "campus.lms",
    "campus.localLibrary",
    "campus.courseFiles",
    "campus.grades",
    "campus.exams",
    "campus.evaluations",
    "campus.rooms",
    "campus.courses",
    "campus.calendar",
    "campus.card",
    "campus.library",
    "campus.venues",
    "campus.attendance",
    "campus.fitness",
    "campus.profile",
];

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IconSet {
    /// Entry id → `data:image/png;base64,…`, already squared.
    pub icons: BTreeMap<String, String>,
    /// One line per entry that was skipped, shown to nobody unless the user
    /// asks; kept so a broken file is diagnosable without a log.
    pub problems: Vec<String>,
}

#[derive(Deserialize)]
struct MapFile {
    #[serde(default)]
    version: u32,
    /// Deliberately no `deny_unknown_fields`: an extra key a later build writes
    /// must not blank every icon in an older one.
    #[serde(default)]
    map: BTreeMap<String, String>,
}

#[derive(Serialize)]
struct MapTemplate {
    version: u32,
    map: BTreeMap<String, &'static str>,
}

pub fn load(app: &AppHandle) -> Result<IconSet, String> {
    let dir = storage::root(app)?.join(FOLDER);
    ensure(&dir)?;
    Ok(read(&dir))
}

/// Creates the folder and an empty template once. An existing file is left
/// byte-for-byte alone: it belongs to the user.
fn ensure(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|_| "无法创建图标文件夹".to_string())?;
    let path = dir.join(MAP_FILE);
    if path.exists() {
        return Ok(());
    }
    let map = KNOWN_IDS
        .iter()
        .map(|id| (id.to_string(), ""))
        .collect::<BTreeMap<_, _>>();
    let mut body = serde_json::to_vec_pretty(&MapTemplate {
        version: VERSION,
        map,
    })
    .map_err(|_| "无法写入图标配置".to_string())?;
    body.push(b'\n');
    fs::write(&path, body).map_err(|_| "无法写入图标配置".to_string())
}

fn read(dir: &Path) -> IconSet {
    let mut set = IconSet::default();
    let Ok(bytes) = fs::read(dir.join(MAP_FILE)) else {
        return set;
    };
    let Ok(file) = serde_json::from_slice::<MapFile>(&bytes) else {
        set.problems
            .push(format!("{MAP_FILE} 不是有效的 JSON，未使用自定义图标"));
        return set;
    };
    if file.version != VERSION {
        set.problems
            .push(format!("{MAP_FILE} 的 version 应为 {VERSION}"));
        return set;
    }
    for (id, name) in file.map {
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        match icon(dir, name) {
            Ok(url) => {
                set.icons.insert(id, url);
            }
            Err(reason) => set.problems.push(format!("{id}：{reason}")),
        }
    }
    set
}

fn icon(dir: &Path, name: &str) -> Result<String, String> {
    // Only plain names and subfolders: `..`, absolute paths and Windows drive
    // prefixes would let the mapping escape the icons folder.
    let relative = Path::new(name);
    if relative.is_absolute()
        || relative
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("路径必须在 icons 文件夹内".into());
    }
    let path = dir.join(relative);
    let meta = fs::symlink_metadata(&path).map_err(|_| "找不到图片".to_string())?;
    if !meta.is_file() || meta.len() > MAX_BYTES {
        return Err(format!(
            "需要不超过 {} MB 的图片文件",
            MAX_BYTES / 1024 / 1024
        ));
    }
    let bytes = fs::read(&path).map_err(|_| "无法读取图片".to_string())?;
    let mut reader = ImageReader::new(Cursor::new(&bytes))
        .with_guessed_format()
        .map_err(|_| "图片格式无效".to_string())?;
    if !matches!(
        reader.format(),
        Some(ImageFormat::Png | ImageFormat::Jpeg | ImageFormat::WebP)
    ) {
        return Err("仅支持 PNG、JPEG 和 WebP".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|_| "图片损坏或尺寸过大".to_string())?;
    // Square it ourselves: the sidebar draws a square, and letting CSS crop a
    // tall image would cut it at an arbitrary place.
    let square = img.resize_to_fill(OUTPUT_SIDE, OUTPUT_SIDE, FilterType::Lanczos3);
    let mut output = Cursor::new(Vec::new());
    square
        .to_rgba8()
        .write_to(&mut output, ImageFormat::Png)
        .map_err(|_| "无法处理图片".to_string())?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(output.into_inner())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgba, RgbaImage};

    fn png(dir: &Path, name: &str) -> Vec<u8> {
        let img = RgbaImage::from_pixel(8, 8, Rgba([200, 40, 90, 255]));
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        img.save(&path).unwrap();
        fs::read(path).unwrap()
    }

    fn write_map(dir: &Path, body: &str) {
        fs::write(dir.join(MAP_FILE), body).unwrap();
    }

    #[test]
    fn first_run_writes_a_template_and_uses_no_icons() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("icons");
        ensure(&dir).unwrap();
        let raw = fs::read_to_string(dir.join(MAP_FILE)).unwrap();
        assert!(raw.contains("\"nav.home\": \"\""));
        let set = read(&dir);
        assert!(set.icons.is_empty());
        assert!(set.problems.is_empty());
    }

    #[test]
    fn existing_file_is_never_rewritten() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("icons");
        fs::create_dir_all(&dir).unwrap();
        write_map(&dir, "{\"version\":1,\"map\":{}}");
        ensure(&dir).unwrap();
        assert_eq!(
            fs::read_to_string(dir.join(MAP_FILE)).unwrap(),
            "{\"version\":1,\"map\":{}}"
        );
    }

    #[test]
    fn mapped_image_becomes_a_square_png_data_url() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("icons");
        fs::create_dir_all(&dir).unwrap();
        png(&dir, "home.png");
        write_map(
            &dir,
            "{\"version\":1,\"map\":{\"nav.home\":\"home.png\",\"nav.agent\":\"\"}}",
        );
        let set = read(&dir);
        assert!(set.problems.is_empty(), "{:?}", set.problems);
        assert_eq!(set.icons.len(), 1);
        let url = &set.icons["nav.home"];
        assert!(url.starts_with("data:image/png;base64,"));
        let decoded = STANDARD.decode(url.rsplit(',').next().unwrap()).unwrap();
        let img = image::load_from_memory(&decoded).unwrap();
        assert_eq!((img.width(), img.height()), (OUTPUT_SIDE, OUTPUT_SIDE));
    }

    #[test]
    fn subfolders_are_allowed() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("icons");
        fs::create_dir_all(&dir).unwrap();
        png(&dir, "campus/todo.png");
        write_map(
            &dir,
            "{\"version\":1,\"map\":{\"campus.todo\":\"campus/todo.png\"}}",
        );
        let set = read(&dir);
        assert!(set.problems.is_empty(), "{:?}", set.problems);
        assert!(set.icons.contains_key("campus.todo"));
    }

    #[test]
    fn paths_outside_the_folder_are_rejected() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("icons");
        fs::create_dir_all(&dir).unwrap();
        write_map(
            &dir,
            "{\"version\":1,\"map\":{\"nav.home\":\"../secret.png\",\"nav.agent\":\"/etc/passwd\"}}",
        );
        let set = read(&dir);
        assert!(set.icons.is_empty());
        assert_eq!(set.problems.len(), 2);
    }

    #[test]
    fn a_broken_entry_only_drops_itself() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("icons");
        fs::create_dir_all(&dir).unwrap();
        png(&dir, "home.png");
        fs::write(dir.join("notes.txt"), "hello").unwrap();
        write_map(
            &dir,
            "{\"version\":1,\"map\":{\"nav.home\":\"home.png\",\"nav.agent\":\"missing.png\",\"nav.settings\":\"notes.txt\"}}",
        );
        let set = read(&dir);
        assert_eq!(set.icons.len(), 1);
        assert_eq!(set.problems.len(), 2);
    }

    #[test]
    fn an_invalid_file_falls_back_to_no_icons() {
        let tmp = tempfile::tempdir().unwrap();
        for body in ["not json", "{\"version\":2,\"map\":{}}"] {
            write_map(tmp.path(), body);
            let set = read(tmp.path());
            assert!(set.icons.is_empty());
            assert_eq!(set.problems.len(), 1);
        }
    }
}
