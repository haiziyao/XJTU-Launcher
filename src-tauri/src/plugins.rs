//! Built-in plugin manifests.
//!
//! The plugin centre is internal: every plugin ships with the app, and the
//! only user decision is whether it is enabled and where its sidebar entry
//! sits. There is no remote catalogue, no package download and no local
//! `plugin.json` import, so this module only parses the manifests compiled
//! into the binary.
use serde::{Deserialize, Serialize};
use url::Url;

pub const TIMETABLE_ID: &str = "cn.xjtu.timetable";
pub const CAMPUS_ID: &str = "cn.xjtu.campus";
pub const AGENT_ID: &str = "cn.xjtu.agent-settings";
pub const HOME_THEME_ID: &str = "cn.xjtu.home-theme";
pub const PPT_CAPTURE_ID: &str = "cn.xjtu.ppt-capture";
pub const DESKTOP_PET_ID: &str = "cn.xjtu.desktop-pet";
pub const FONT_BEAUTIFY_ID: &str = "cn.xjtu.font-beautify";

const BUILTIN_IDS: [&str; 7] = [
    TIMETABLE_ID,
    CAMPUS_ID,
    AGENT_ID,
    HOME_THEME_ID,
    PPT_CAPTURE_ID,
    DESKTOP_PET_ID,
    FONT_BEAUTIFY_ID,
];

pub const CATEGORIES: [&str; 3] = ["campus", "invention", "lab"];

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plugin {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
    /// 校园广场 / 拓展发明 / 创新小港: which section of the plugin centre
    /// lists this plugin.
    #[serde(default = "default_category")]
    pub category: String,
    #[serde(default = "default_author")]
    pub author: String,
    /// Long description shown when a row is expanded.
    #[serde(default)]
    pub details: String,
    /// Whether a fresh install starts with the plugin enabled.
    #[serde(default = "default_true")]
    pub enabled_by_default: bool,
    pub page: Page,
    pub widget: bool,
    /// Whether the plugin gets its own sidebar entry. Built-in pages that are
    /// opened from inside a workflow (for example the replay capture page,
    /// reached from a lesson) set this to false; the manifest default is true.
    #[serde(default = "default_true")]
    pub sidebar: bool,
}

fn default_category() -> String {
    "invention".into()
}

fn default_author() -> String {
    "haiziyao".into()
}

fn default_true() -> bool {
    true
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum Page {
    Timetable,
    Links {
        links: Vec<Link>,
    },
    Campus,
    #[serde(rename = "agentSettings")]
    AgentSettings,
    #[serde(rename = "homeTheme")]
    HomeTheme,
    #[serde(rename = "pptCapture")]
    PptCapture,
    #[serde(rename = "desktopPet")]
    DesktopPet,
    #[serde(rename = "fontBeautify")]
    FontBeautify,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    title: String,
    description: String,
    url: String,
}

#[derive(Serialize)]
pub struct PluginList {
    pub plugins: Vec<Plugin>,
}

pub fn valid_id(id: &str) -> bool {
    (3..=80).contains(&id.len())
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && !id.contains("..")
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
}

pub fn is_builtin(id: &str) -> bool {
    BUILTIN_IDS.contains(&id)
}

pub fn validate_url(text: &str) -> Result<Url, String> {
    let url = Url::parse(text).map_err(|_| "链接格式错误")?;
    if !["https", "http"].contains(&url.scheme())
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("仅支持不含账号密码的 HTTP / HTTPS 链接".into());
    }
    Ok(url)
}

impl Plugin {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != 1
            || [
                "home",
                "agent",
                "market",
                "settings",
                "personal",
                "accounts",
                "community",
            ]
            .contains(&self.id.as_str())
            || !is_builtin(&self.id)
            || self.name.trim().is_empty()
            || self.name.chars().count() > 30
            || self.description.chars().count() > 300
            || self.details.chars().count() > 600
            || self.author.trim().is_empty()
            || self.author.chars().count() > 40
            || !CATEGORIES.contains(&self.category.as_str())
            || self.version.len() > 32
            || self.icon.len() > 30
        {
            return Err("插件清单字段无效或版本不受支持".into());
        }
        if let Page::Links { links } = &self.page {
            if links.len() > 50 {
                return Err("导航插件最多包含 50 个链接".into());
            }
            for link in links {
                if link.title.trim().is_empty()
                    || link.title.chars().count() > 60
                    || link.description.chars().count() > 300
                {
                    return Err("导航文字长度无效".into());
                }
                validate_url(&link.url)?;
            }
        }
        Ok(())
    }
}

fn builtin(json: &str, broken: &str) -> Result<Plugin, String> {
    let plugin: Plugin = serde_json::from_str(json).map_err(|_| broken.to_string())?;
    plugin.validate()?;
    Ok(plugin)
}

/// Every plugin the app ships. The manifests are compiled in, so the list is
/// identical on every launch and cannot be changed by files on disk.
pub fn list() -> Result<PluginList, String> {
    let plugins = vec![
        builtin(
            include_str!("../../plugins/cn.xjtu.timetable/plugin.json"),
            "内置课程表插件损坏",
        )?,
        builtin(
            include_str!("../../plugins/cn.xjtu.campus/plugin.json"),
            "内置畅行校园插件损坏",
        )?,
        builtin(
            include_str!("../../plugins/cn.xjtu.desktop-pet/plugin.json"),
            "内置桌面宠物插件损坏",
        )?,
        builtin(
            include_str!("../../plugins/cn.xjtu.font-beautify/plugin.json"),
            "内置字体美化插件损坏",
        )?,
        builtin(
            include_str!("../../plugins/cn.xjtu.home-theme/plugin.json"),
            "内置主页主题插件损坏",
        )?,
        builtin(
            include_str!("../../plugins/cn.xjtu.agent-settings/plugin.json"),
            "内置 Agent 设置插件损坏",
        )?,
        builtin(
            include_str!("../../plugins/cn.xjtu.ppt-capture/plugin.json"),
            "内置回放截取插件损坏",
        )?,
    ];
    Ok(PluginList { plugins })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_manifest_is_valid_and_unique() {
        let result = list().unwrap();
        assert_eq!(result.plugins.len(), BUILTIN_IDS.len());
        let mut ids: Vec<&str> = result.plugins.iter().map(|p| p.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), result.plugins.len());
        for plugin in &result.plugins {
            assert!(CATEGORIES.contains(&plugin.category.as_str()));
            assert_eq!(plugin.author, "haiziyao");
        }
    }

    #[test]
    fn new_invention_plugins_start_disabled() {
        let result = list().unwrap();
        for id in [DESKTOP_PET_ID, FONT_BEAUTIFY_ID] {
            let plugin = result.plugins.iter().find(|p| p.id == id).unwrap();
            assert_eq!(plugin.category, "invention");
            assert!(!plugin.enabled_by_default);
        }
        let capture = result
            .plugins
            .iter()
            .find(|p| p.id == PPT_CAPTURE_ID)
            .unwrap();
        assert_eq!(capture.category, "lab");
        assert!(!capture.sidebar);
    }

    #[test]
    fn rejects_paths_and_scripts() {
        for id in ["../escape", "/tmp/test", ".hidden", "x/xx", "a..b"] {
            assert!(!valid_id(id));
        }
        for url in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "https://user:pass@xjtu.edu.cn",
        ] {
            assert!(validate_url(url).is_err());
        }
    }

    #[test]
    fn rejects_unknown_ids_and_bad_categories() {
        let mut plugin: Plugin =
            serde_json::from_str(include_str!("../../plugins/cn.xjtu.timetable/plugin.json"))
                .unwrap();
        plugin.id = "local.links".into();
        assert!(plugin.validate().is_err());
        let mut plugin: Plugin =
            serde_json::from_str(include_str!("../../plugins/cn.xjtu.timetable/plugin.json"))
                .unwrap();
        plugin.category = "remote".into();
        assert!(plugin.validate().is_err());
    }
}
