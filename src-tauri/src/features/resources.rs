//! Read-only web resources shown in 社区广场 → 资源站点.
//!
//! The catalog is data, not code. The bundled copy lives in
//! `resources/community/` at the repository root and Tauri copies it next to
//! the executable; an optional user copy under
//! `~/.xjtu-launcher/resources/community/` wins when it has a `sites.json`.
//! Icons are always resolved from the catalog's own folder first, then from the
//! bundled folder, so a user catalog can keep using the shipped icons.
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};
use tauri::{AppHandle, Manager, WebviewWindow};

const VERSION: u32 = 1;
const MAX_JSON_BYTES: u64 = 256 * 1024;
const MAX_ICON_BYTES: u64 = 512 * 1024;
const MAX_CATEGORIES: usize = 32;
const MAX_SITES: usize = 500;
const MAX_TAGS: usize = 8;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceSite {
    pub id: String,
    pub title: String,
    pub url: String,
    pub description: String,
    pub categories: Vec<String>,
    pub tags: Vec<String>,
    pub featured: bool,
    /// `data:image/...;base64,…`, or `None` when the icon is missing/broken.
    pub icon: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCategory {
    pub id: String,
    pub title: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCatalog {
    pub version: u32,
    pub updated_at: String,
    pub categories: Vec<ResourceCategory>,
    pub sites: Vec<ResourceSite>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawCatalog {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    updated_at: String,
    #[serde(default)]
    categories: Vec<RawCategory>,
    #[serde(default)]
    sites: Vec<RawSite>,
}

#[derive(Deserialize)]
struct RawCategory {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawSite {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: Option<String>,
    #[serde(default)]
    categories: Vec<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    featured: bool,
}

/// The catalog an ordinary install reads: the user copy when present, else the
/// copy shipped with the app. `fallback` is used only to find icons.
fn roots(app: &AppHandle) -> Result<(PathBuf, Option<PathBuf>), String> {
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|_| "无法定位资源目录".to_string())?
        .join("community");
    let user = crate::storage::root(app)?.join("resources/community");
    if user.join("sites.json").is_file() {
        Ok((user, Some(bundled)))
    } else {
        Ok((bundled, None))
    }
}

pub fn load(root: &Path, fallback: Option<&Path>) -> Result<ResourceCatalog, String> {
    let path = root.join("sites.json");
    let size = fs::metadata(&path)
        .map_err(|_| "无法读取资源站点数据".to_string())?
        .len();
    if size > MAX_JSON_BYTES {
        return Err("资源站点数据超过 256 KB".to_string());
    }
    let bytes = fs::read(&path).map_err(|_| "无法读取资源站点数据".to_string())?;
    let raw: RawCatalog =
        serde_json::from_slice(&bytes).map_err(|_| "资源站点数据格式无效".to_string())?;
    validate(raw, |relative| read_icon(root, fallback, relative))
}

fn validate(
    raw: RawCatalog,
    mut icon: impl FnMut(&str) -> Option<String>,
) -> Result<ResourceCatalog, String> {
    if raw.version != VERSION {
        return Err(format!("资源站点数据的 version 应为 {VERSION}"));
    }
    if raw.categories.len() > MAX_CATEGORIES {
        return Err(format!("资源站点分类不能超过 {MAX_CATEGORIES} 个"));
    }
    if raw.sites.len() > MAX_SITES {
        return Err(format!("资源站点条目不能超过 {MAX_SITES} 个"));
    }

    let mut categories = Vec::with_capacity(raw.categories.len());
    for category in raw.categories {
        if !valid_id(&category.id) {
            return Err("资源站点分类 id 无效".to_string());
        }
        if category.id == "recommended" {
            return Err("分类 id「recommended」为「推荐」标签保留值".to_string());
        }
        if !valid_text(&category.title, 1, 30) {
            return Err(format!("分类「{}」的名称无效", category.id));
        }
        if categories
            .iter()
            .any(|item: &ResourceCategory| item.id == category.id)
        {
            return Err(format!("分类 id「{}」重复", category.id));
        }
        categories.push(ResourceCategory {
            id: category.id,
            title: category.title,
        });
    }

    let mut sites = Vec::with_capacity(raw.sites.len());
    for site in raw.sites {
        if !valid_id(&site.id) {
            return Err("资源站点 id 无效".to_string());
        }
        if !valid_text(&site.title, 1, 60) {
            return Err(format!("资源「{}」的标题无效", site.id));
        }
        if site.description.chars().count() > 200 {
            return Err(format!("资源「{}」的描述不能超过 200 字", site.id));
        }
        let parsed =
            url::Url::parse(&site.url).map_err(|_| format!("资源「{}」的网址无效", site.id))?;
        if !matches!(parsed.scheme(), "http" | "https") {
            return Err(format!("资源「{}」只允许 HTTP / HTTPS 网址", site.id));
        }
        if parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err(format!("资源「{}」的网址包含不允许的账号信息", site.id));
        }
        if site.categories.is_empty() {
            return Err(format!("资源「{}」至少要属于一个分类", site.id));
        }
        if site.categories.len() > categories.len() {
            return Err(format!("资源「{}」的分类无效", site.id));
        }
        for id in &site.categories {
            if !categories.iter().any(|item| &item.id == id) {
                return Err(format!("资源「{}」引用了不存在的分类「{id}」", site.id));
            }
        }
        if site.tags.len() > MAX_TAGS {
            return Err(format!("资源「{}」的标签不能超过 {MAX_TAGS} 个", site.id));
        }
        if site.tags.iter().any(|tag| !valid_text(tag, 1, 16)) {
            return Err(format!("资源「{}」的标签无效", site.id));
        }
        let icon = match site.icon.as_deref() {
            Some(relative) => {
                if !safe_relative(relative) {
                    return Err(format!("资源「{}」的图标路径无效", site.id));
                }
                if icon_mime(Path::new(relative)).is_none() {
                    return Err(format!(
                        "资源「{}」的图标只支持 SVG / PNG / JPEG / WebP",
                        site.id
                    ));
                }
                icon(relative)
            }
            None => None,
        };
        if sites.iter().any(|item: &ResourceSite| item.id == site.id) {
            return Err(format!("资源 id「{}」重复", site.id));
        }
        sites.push(ResourceSite {
            id: site.id,
            title: site.title,
            url: site.url,
            description: site.description,
            categories: site.categories,
            tags: site.tags,
            featured: site.featured,
            icon,
        });
    }

    Ok(ResourceCatalog {
        version: raw.version,
        updated_at: raw.updated_at,
        categories,
        sites,
    })
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn valid_text(value: &str, min: usize, max: usize) -> bool {
    let count = value.chars().count();
    (min..=max).contains(&count) && !value.trim().is_empty()
}

fn safe_relative(relative: &str) -> bool {
    let path = Path::new(relative);
    !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
        && path.components().next().is_some()
}

fn icon_mime(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn read_icon(root: &Path, fallback: Option<&Path>, relative: &str) -> Option<String> {
    let path = Path::new(relative);
    let mime = icon_mime(path)?;
    let candidate = [Some(root), fallback]
        .into_iter()
        .flatten()
        .map(|base| base.join(path))
        .find(|path| path.is_file())?;
    let size = fs::metadata(&candidate).ok()?.len();
    if size > MAX_ICON_BYTES {
        return None;
    }
    let bytes = fs::read(candidate).ok()?;
    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

/// Resource links for 社区广场. Read-only; no network request is made.
#[tauri::command]
pub fn resource_sites(window: WebviewWindow, app: AppHandle) -> Result<ResourceCatalog, String> {
    crate::main_only(&window)?;
    let (root, fallback) = roots(&app)?;
    load(&root, fallback.as_deref())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog(sites: &str) -> String {
        format!(
            r#"{{
              "version": 1,
              "updatedAt": "2026-11-11",
              "categories": [
                {{ "id": "study", "title": "学习" }},
                {{ "id": "life", "title": "生活" }}
              ],
              "sites": {sites}
            }}"#
        )
    }

    fn parse(body: &str) -> Result<ResourceCatalog, String> {
        let raw: RawCatalog = serde_json::from_str(body).map_err(|error| error.to_string())?;
        validate(raw, |_| None)
    }

    #[test]
    fn parses_a_catalog_and_keeps_order() {
        let body = catalog(
            r#"[
              {
                "id": "jwxt",
                "title": "教务系统",
                "url": "https://jwxt.xjtu.edu.cn/",
                "description": "选课与成绩",
                "icon": "icons/jwxt.svg",
                "categories": ["study"],
                "tags": ["选课", "成绩"],
                "featured": true
              },
              {
                "id": "card",
                "title": "一卡通",
                "url": "https://ncard.xjtu.edu.cn/",
                "categories": ["life"]
              }
            ]"#,
        );
        let result = parse(&body).expect("catalog should parse");
        assert_eq!(result.version, 1);
        assert_eq!(result.categories.len(), 2);
        assert_eq!(result.sites.len(), 2);
        assert_eq!(result.sites[0].id, "jwxt");
        assert!(result.sites[0].featured);
        assert_eq!(result.sites[0].tags, vec!["选课", "成绩"]);
        assert!(!result.sites[1].featured);
        assert!(result.sites[1].icon.is_none());
    }

    #[test]
    fn rejects_non_http_urls_and_embedded_credentials() {
        for url in [
            "javascript:alert(1)",
            "ftp://example.com/file",
            "https://user:pass@example.com/",
            "https://user@example.com/",
        ] {
            let body = catalog(&format!(
                r#"[{{ "id": "bad", "title": "坏链接", "url": "{url}", "categories": ["study"] }}]"#
            ));
            assert!(parse(&body).is_err(), "{url} should be rejected");
        }
    }

    #[test]
    fn rejects_icon_paths_that_leave_the_folder() {
        for icon in ["../secret.svg", "/etc/passwd", "icons/../../x.svg"] {
            let body = catalog(&format!(
                r#"[{{
                  "id": "bad",
                  "title": "坏图标",
                  "url": "https://example.com/",
                  "icon": "{icon}",
                  "categories": ["study"]
                }}]"#
            ));
            assert!(parse(&body).is_err(), "{icon} should be rejected");
        }
    }

    #[test]
    fn shipped_catalog_is_valid() {
        let body = include_str!("../../../resources/community/sites.json");
        let raw: RawCatalog = serde_json::from_str(body).expect("shipped catalog should parse");
        let result = validate(raw, |_| None).expect("shipped catalog should validate");
        assert!(!result.categories.is_empty());
        assert!(!result.sites.is_empty());
        assert!(result.sites.iter().any(|site| site.featured));
    }

    #[test]
    fn rejects_unknown_categories_and_bad_versions() {
        let unknown = catalog(
            r#"[{ "id": "x", "title": "X", "url": "https://example.com/", "categories": ["nope"] }]"#,
        );
        assert!(parse(&unknown).is_err());

        let wrong_version = r#"{ "version": 2, "categories": [], "sites": [] }"#;
        assert!(parse(wrong_version).is_err());

        let reserved = r#"{ "version": 1, "categories": [{ "id": "recommended", "title": "推荐" }], "sites": [] }"#;
        assert!(parse(reserved).is_err());
    }
}
