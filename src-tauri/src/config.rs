use serde::{Deserialize, Serialize};
use std::{fs, io::Write, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub sidebar_collapsed: bool,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    #[serde(default)]
    pub nickname: String,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub wallpaper: Option<String>,
    #[serde(default)]
    pub reduce_motion: bool,
    #[serde(default = "default_show_seconds")]
    pub show_seconds: bool,
    pub accent: String,
    pub greeting: String,
    pub widgets: Vec<String>,
    pub plugin_order: Vec<String>,
    pub disabled_plugins: Vec<String>,
    #[serde(default)]
    pub hidden_plugins: Vec<String>,
    #[serde(default)]
    pub plugin_server: Option<String>,
    #[serde(default = "default_show_seconds")]
    pub check_plugin_updates: bool,
    #[serde(default = "default_show_seconds")]
    pub check_app_updates: bool,
}

fn default_sidebar_width() -> u32 {
    196
}

fn default_show_seconds() -> bool {
    true
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            sidebar_collapsed: false,
            sidebar_width: default_sidebar_width(),
            nickname: String::new(),
            avatar: None,
            wallpaper: None,
            reduce_motion: false,
            show_seconds: true,
            accent: "blue".into(),
            greeting: "把校园，放在手边。".into(),
            widgets: vec!["timetable".into(), "links".into(), "plugins".into()],
            plugin_order: vec![],
            disabled_plugins: vec![],
            hidden_plugins: vec![],
            plugin_server: None,
            check_plugin_updates: true,
            check_app_updates: true,
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(176..=320).contains(&self.sidebar_width)
            || self.nickname.chars().count() > 40
            || [&self.avatar, &self.wallpaper].iter().any(|v| {
                v.as_ref().is_some_and(|s| {
                    s.len() > 12 * 1024 * 1024 || !s.starts_with("data:image/png;base64,")
                })
            })
            || self.version != 1
            || !["sage", "blue", "violet", "amber"].contains(&self.accent.as_str())
            || self.greeting.chars().count() > 60
            || self.widgets.len() > 3
            || self
                .widgets
                .iter()
                .any(|w| !["timetable", "links", "plugins"].contains(&w.as_str()))
            || self.plugin_order.len() > 256
            || self.disabled_plugins.len() > 256
            || self.hidden_plugins.len() > 256
            || self
                .plugin_order
                .iter()
                .chain(&self.disabled_plugins)
                .chain(&self.hidden_plugins)
                .any(|id| !crate::plugins::valid_id(id))
        {
            return Err("配置格式不受支持，请检查版本和字段".into());
        }
        if let Some(source) = &self.plugin_server {
            crate::market::source_url(source)?;
        }
        let mut unique = self.widgets.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != self.widgets.len() {
            return Err("主页组件不能重复".into());
        }
        Ok(())
    }
}

pub fn load(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取本地配置".to_string())?;
    let config: AppConfig = serde_json::from_slice(&bytes)
        .map_err(|_| "配置文件损坏，已保留原文件，请修复后重试".to_string())?;
    config.validate()?;
    Ok(config)
}

// Persist replaces atomically on macOS and Windows. A failed write leaves the old file intact.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("无效的存储目录")?;
    fs::create_dir_all(parent).map_err(|_| "无法创建本地数据目录")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|_| "无法创建临时配置")?;
    temporary.write_all(bytes).map_err(|_| "无法写入本地文件")?;
    temporary
        .as_file()
        .sync_all()
        .map_err(|_| "无法保存本地文件")?;
    temporary.persist(path).map_err(|_| "无法替换本地文件")?;
    Ok(())
}

pub fn save(path: &Path, config: &AppConfig) -> Result<(), String> {
    config.validate()?;
    write_atomic(
        path,
        &serde_json::to_vec_pretty(config).map_err(|_| "无法序列化配置")?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_config_keeps_sidebar_and_plugins_when_personal_fields_are_added() {
        let old = r#"{"version":1,"sidebarCollapsed":true,"accent":"blue","greeting":"","widgets":["links"],"pluginOrder":[],"disabledPlugins":["cn.xjtu.timetable"]}"#;
        let config: AppConfig = serde_json::from_str(old).unwrap();
        config.validate().unwrap();
        assert!(config.sidebar_collapsed);
        assert_eq!(config.sidebar_width, 196);
        assert_eq!(config.disabled_plugins, vec!["cn.xjtu.timetable"]);
        assert!(config.avatar.is_none());
        assert!(config.wallpaper.is_none());
        assert!(config.show_seconds);
        assert!(!config.reduce_motion);
        assert!(config.hidden_plugins.is_empty());
        assert!(config.plugin_server.is_none());
        assert!(config.check_plugin_updates);
        assert!(config.check_app_updates);
    }

    #[test]
    fn persists_edits_and_replaces_previous_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        let mut config = AppConfig::default();
        save(&path, &config).unwrap();
        config.accent = "blue".into();
        config.widgets.reverse();
        config.sidebar_width = 280;
        save(&path, &config).unwrap();
        assert_eq!(load(&path).unwrap(), config);
    }

    #[test]
    fn corrupt_file_is_not_silently_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        fs::write(&path, b"broken").unwrap();
        assert!(load(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
    }

    #[test]
    fn rejects_unknown_fields_and_duplicate_widgets() {
        let config = AppConfig {
            widgets: vec!["links".into(), "links".into()],
            ..AppConfig::default()
        };
        assert!(config.validate().is_err());
        let mut json = serde_json::to_value(AppConfig::default()).unwrap();
        json["apiKey"] = "never store here".into();
        assert!(serde_json::from_value::<AppConfig>(json).is_err());
    }
}
