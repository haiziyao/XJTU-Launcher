use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, io::Write, path::Path};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AppConfig {
    pub version: u32,
    #[serde(default)]
    pub sidebar_collapsed: bool,
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32,
    /// User order of the sidebar entries. Unknown ids are ignored and new
    /// entries are appended by the frontend.
    #[serde(default)]
    pub sidebar_order: Vec<String>,
    /// Whether the Agent entry shows in the sidebar. On by default; the
    /// Agent settings page can turn it off without leaving the app.
    #[serde(default = "default_true")]
    pub agent_sidebar_visible: bool,
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
    #[serde(default = "default_pet")]
    pub pet: String,
    pub greeting: String,
    pub widgets: Vec<String>,
    pub plugin_order: Vec<String>,
    pub disabled_plugins: Vec<String>,
    #[serde(default)]
    pub hidden_plugins: Vec<String>,
    /// Internal plugin-centre revision. 0 marks a config written before the
    /// internal plugin centre; `load` then applies the new plugins' defaults
    /// once. The frontend carries the value through every save.
    #[serde(default)]
    pub plugin_layout: u32,
    #[serde(default = "default_show_seconds")]
    pub check_app_updates: bool,
    /// Tool ids pinned to the top of the 畅行校园 page, in display order.
    #[serde(default)]
    pub campus_pinned: Vec<String>,
    /// 畅行校园 section rail collapsed to icons (LAYOUT §3.1).
    #[serde(default)]
    pub campus_rail_collapsed: bool,
    /// Clock style on the home page, managed by the 主页主题 plugin.
    #[serde(default = "default_home_clock")]
    pub home_clock: String,
    /// Home sections the user keeps visible; order comes from `widgets`.
    #[serde(default = "default_home_sections")]
    pub home_sections: Vec<String>,
    /// Local MCP endpoint on the personal page; off until the user turns it on.
    #[serde(default)]
    pub mcp_server_enabled: bool,
    /// Loopback port chosen by the user. Never falls back to another port.
    #[serde(default = "default_mcp_port")]
    pub mcp_server_port: u16,
    /// Built-in tools the user keeps out of that endpoint. Unknown names are
    /// ignored so a config written by an older build still loads.
    #[serde(default)]
    pub mcp_server_disabled_tools: Vec<String>,
    /// Legacy plugin-service setting. Kept only so a config written by an
    /// older build still loads; the plugin centre is internal now, and this
    /// field is never written back.
    #[serde(default, skip_serializing)]
    pub plugin_server: Option<String>,
    /// Legacy automatic plugin-update switch. Never written back.
    #[serde(default = "default_true", skip_serializing)]
    pub check_plugin_updates: bool,
    /// Legacy desktop-pet settings. They moved to
    /// `~/.xjtu-launcher/plugins/settings/cn.xjtu.desktop-pet.json`; kept here
    /// only to migrate old files, and never serialized again.
    #[serde(default = "default_true", skip_serializing)]
    pub pet_desktop_enabled: bool,
    #[serde(default = "default_pet_character", skip_serializing)]
    pub pet_character: String,
    #[serde(default = "default_pet_bar_style", skip_serializing)]
    pub pet_bar_style: String,
    #[serde(default, skip_serializing)]
    pub pet_bar_hidden: bool,
    #[serde(default = "default_true", skip_serializing)]
    pub pet_bubble_enabled: bool,
    #[serde(default = "default_pet_sources", skip_serializing)]
    pub pet_sources: Vec<String>,
    #[serde(default, skip_serializing)]
    pub pet_position: Option<[i32; 2]>,
    /// Give each page-level tab its own rounded backing.
    #[serde(default)]
    pub page_tab_surfaces: bool,
    /// Wallpaper display preset: clear | balanced | vivid. Only meaningful while
    /// a wallpaper is set; the plain page ignores it.
    #[serde(default = "default_wallpaper_display")]
    pub wallpaper_display: String,
    /// Panel opacity (percent) the user chose for each wallpaper preset. A
    /// missing mode falls back to that mode's default on the frontend.
    #[serde(default = "default_wallpaper_panel")]
    pub wallpaper_panel: BTreeMap<String, u8>,
    /// Optional HTTP(S) mirror of the update manifest; empty uses the endpoint
    /// compiled into this build. Plain HTTP is a P0 stopgap until the mirror
    /// gets a domain and certificate; see dev-docs/P0.md.
    #[serde(default)]
    pub update_endpoint: String,
}

fn default_home_clock() -> String {
    "plain".into()
}

pub const WALLPAPER_MODES: [&str; 3] = ["clear", "balanced", "vivid"];

fn default_wallpaper_display() -> String {
    "balanced".into()
}

fn default_wallpaper_panel() -> BTreeMap<String, u8> {
    [("clear", 92), ("balanced", 72), ("vivid", 52)]
        .into_iter()
        .map(|(mode, value)| (mode.to_string(), value))
        .collect()
}

pub const HOME_SECTIONS: [&str; 6] = [
    "greeting",
    "clock",
    "timetable",
    "links",
    "plugins",
    "notices",
];

fn default_home_sections() -> Vec<String> {
    ["greeting", "clock", "timetable", "links", "notices"]
        .iter()
        .map(|id| id.to_string())
        .collect()
}

fn default_sidebar_width() -> u32 {
    140
}

fn default_show_seconds() -> bool {
    true
}

/// The examples include this module without the rest of the crate, so the
/// default lives here rather than in `expose`.
pub const DEFAULT_MCP_PORT: u16 = 39217;

fn default_mcp_port() -> u16 {
    DEFAULT_MCP_PORT
}

fn default_pet() -> String {
    "xixi".into()
}

fn default_true() -> bool {
    true
}

fn default_pet_character() -> String {
    "deepseek-girl".into()
}

fn default_pet_bar_style() -> String {
    "glass-light".into()
}

fn default_pet_sources() -> Vec<String> {
    ["campus", "agent", "update"]
        .iter()
        .map(|id| id.to_string())
        .collect()
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: 1,
            sidebar_collapsed: false,
            sidebar_width: default_sidebar_width(),
            sidebar_order: vec![],
            agent_sidebar_visible: true,
            nickname: String::new(),
            avatar: None,
            wallpaper: None,
            reduce_motion: false,
            show_seconds: true,
            accent: "#1f6fe5".into(),
            pet: default_pet(),
            greeting: "把校园，放在手边。".into(),
            widgets: vec![
                "timetable".into(),
                "links".into(),
                "notices".into(),
                "plugins".into(),
            ],
            plugin_order: vec![],
            // The pet and the font tool are optional inventions; a fresh
            // install turns them on only when the user asks.
            disabled_plugins: vec![
                crate::plugins::DESKTOP_PET_ID.into(),
                crate::plugins::FONT_BEAUTIFY_ID.into(),
            ],
            hidden_plugins: vec![],
            plugin_layout: 1,
            plugin_server: None,
            check_plugin_updates: true,
            check_app_updates: true,
            campus_pinned: Vec::new(),
            campus_rail_collapsed: false,
            home_clock: default_home_clock(),
            home_sections: default_home_sections(),
            mcp_server_enabled: false,
            mcp_server_port: default_mcp_port(),
            mcp_server_disabled_tools: Vec::new(),
            pet_desktop_enabled: true,
            pet_character: default_pet_character(),
            pet_bar_style: default_pet_bar_style(),
            pet_bar_hidden: false,
            pet_bubble_enabled: true,
            pet_sources: default_pet_sources(),
            pet_position: None,
            page_tab_surfaces: false,
            wallpaper_display: default_wallpaper_display(),
            wallpaper_panel: default_wallpaper_panel(),
            update_endpoint: String::new(),
        }
    }
}

// Personal images are stored inline as data URLs produced by `personal_image`.
fn bad_image(value: &Option<String>, formats: &[&str]) -> bool {
    value.as_ref().is_some_and(|s| {
        s.len() > 12 * 1024 * 1024
            || !formats
                .iter()
                .any(|f| s.starts_with(&format!("data:image/{f};base64,")))
    })
}

// Campus tool ids are lowercase slugs, the same shape as plugin ids.
fn valid_tool_name(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

// P0: plain HTTP is accepted only as a temporary stopgap while the mirror has
// no domain or certificate (dev-docs/P0.md). Credentials and malformed URLs are
// rejected before the updater ever sees them. Restore HTTPS-only once the
// mirror is served over TLS.
fn bad_update_endpoint(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    if value.len() > 2048 {
        return true;
    }
    match url::Url::parse(value) {
        Ok(url) => {
            !["http", "https"].contains(&url.scheme())
                || url.host_str().is_none()
                || !url.username().is_empty()
                || url.password().is_some()
        }
        Err(_) => true,
    }
}

// The accent is a #rrggbb colour; older configs still use the original names.
fn bad_accent(value: &str) -> bool {
    const LEGACY: [&str; 4] = ["sage", "blue", "violet", "amber"];
    if LEGACY.contains(&value) {
        return false;
    }
    let bytes = value.as_bytes();
    bytes.len() != 7 || bytes[0] != b'#' || !bytes[1..].iter().all(u8::is_ascii_hexdigit)
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), String> {
        if !(130..=320).contains(&self.sidebar_width)
            || self.nickname.chars().count() > 40
            || bad_image(&self.avatar, &["png"])
            || bad_image(&self.wallpaper, &["png", "jpeg"])
            || self.version != 1
            || bad_accent(&self.accent)
            || !["xixi", "anan", "jiaojiao", "tongtong", "dada", "xuexue"]
                .contains(&self.pet.as_str())
            || self.campus_pinned.len() > 24
            || self.campus_pinned.iter().any(|id| !valid_tool_name(id))
            || !(1024..=65535).contains(&self.mcp_server_port)
            || self.mcp_server_disabled_tools.len() > 64
            || self
                .mcp_server_disabled_tools
                .iter()
                .any(|name| !valid_tool_name(name))
            || {
                let unique: std::collections::HashSet<_> =
                    self.mcp_server_disabled_tools.iter().collect();
                unique.len() != self.mcp_server_disabled_tools.len()
            }
            || !["plain", "bold", "mono", "card"].contains(&self.home_clock.as_str())
            || !WALLPAPER_MODES.contains(&self.wallpaper_display.as_str())
            || self.wallpaper_panel.iter().any(|(mode, value)| {
                !WALLPAPER_MODES.contains(&mode.as_str()) || !(40..=100).contains(value)
            })
            || self.home_sections.len() > HOME_SECTIONS.len()
            || self
                .home_sections
                .iter()
                .any(|id| !HOME_SECTIONS.contains(&id.as_str()))
            || {
                let unique: std::collections::HashSet<_> = self.home_sections.iter().collect();
                unique.len() != self.home_sections.len()
            }
            || bad_update_endpoint(&self.update_endpoint)
            || self.greeting.chars().count() > 60
            || self.widgets.len() > 4
            || self
                .widgets
                .iter()
                .any(|w| !["timetable", "links", "plugins", "notices"].contains(&w.as_str()))
            || self.plugin_layout > 1
            || self.sidebar_order.len() > 256
            || self.plugin_order.len() > 256
            || self.disabled_plugins.len() > 256
            || self.hidden_plugins.len() > 256
            || self
                .sidebar_order
                .iter()
                .chain(&self.plugin_order)
                .chain(&self.disabled_plugins)
                .chain(&self.hidden_plugins)
                .any(|id| !crate::plugins::valid_id(id))
        {
            return Err("配置格式不受支持，请检查版本和字段".into());
        }
        let mut unique = self.widgets.clone();
        unique.sort();
        unique.dedup();
        if unique.len() != self.widgets.len() {
            return Err("主页组件不能重复".into());
        }
        let mut sidebar = self.sidebar_order.clone();
        sidebar.sort();
        sidebar.dedup();
        if sidebar.len() != self.sidebar_order.len() {
            return Err("侧栏顺序不能重复".into());
        }
        Ok(())
    }
}

/// Pre-merge tool names map to the merged tool, so a setting written by an
/// older build keeps the same capability hidden. Order is kept and duplicates
/// are dropped.
fn canonical_tool_names(names: &[String]) -> Vec<String> {
    let mut canonical: Vec<String> = Vec::with_capacity(names.len());
    for name in names {
        let name = crate::tool_names::canonical_name(name);
        if !canonical.iter().any(|existing| existing == name) {
            canonical.push(name.to_string());
        }
    }
    canonical
}

/// One-time defaults for the internal plugin centre. Older configs know only
/// the five original plugins, so the pet and font tool must be added to the
/// disabled list unless the pet was already in use. The marker is stored in
/// the config and carried through saves, so this runs exactly once.
fn migrate_plugin_layout(config: &mut AppConfig) {
    if config.plugin_layout >= 1 {
        return;
    }
    if !config.pet_desktop_enabled
        && !config
            .disabled_plugins
            .iter()
            .any(|id| id == crate::plugins::DESKTOP_PET_ID)
    {
        config
            .disabled_plugins
            .push(crate::plugins::DESKTOP_PET_ID.into());
    }
    if !config
        .disabled_plugins
        .iter()
        .any(|id| id == crate::plugins::FONT_BEAUTIFY_ID)
    {
        config
            .disabled_plugins
            .push(crate::plugins::FONT_BEAUTIFY_ID.into());
    }
    config.plugin_layout = 1;
}

pub fn load(path: &Path) -> Result<AppConfig, String> {
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取本地配置".to_string())?;
    let mut config: AppConfig = serde_json::from_slice(&bytes)
        .map_err(|_| "配置文件损坏，已保留原文件，请修复后重试".to_string())?;
    config.mcp_server_disabled_tools = canonical_tool_names(&config.mcp_server_disabled_tools);
    migrate_plugin_layout(&mut config);
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
        assert_eq!(config.sidebar_width, 140);
        assert_eq!(config.disabled_plugins, vec!["cn.xjtu.timetable"]);
        assert!(config.avatar.is_none());
        assert!(config.wallpaper.is_none());
        assert!(config.show_seconds);
        assert!(!config.reduce_motion);
        assert!(config.hidden_plugins.is_empty());
        assert!(config.plugin_server.is_none());
        assert!(config.check_plugin_updates);
        assert!(config.check_app_updates);
        assert!(!config.mcp_server_enabled);
        assert_eq!(config.mcp_server_port, 39217);
        assert!(config.mcp_server_disabled_tools.is_empty());
    }

    #[test]
    fn sidebar_order_round_trips_and_rejects_duplicates() {
        let config = AppConfig {
            sidebar_order: vec!["settings".into(), "home".into(), "cn.xjtu.timetable".into()],
            ..AppConfig::default()
        };
        config.validate().unwrap();
        let saved = serde_json::to_value(&config).unwrap();
        assert_eq!(saved["sidebarOrder"][0], "settings");

        let duplicate = AppConfig {
            sidebar_order: vec!["home".into(), "home".into()],
            ..AppConfig::default()
        };
        assert!(duplicate.validate().is_err());
        let bad_case = AppConfig {
            sidebar_order: vec!["Home".into()],
            ..AppConfig::default()
        };
        assert!(bad_case.validate().is_err());
    }

    #[test]
    fn fresh_config_keeps_new_invention_plugins_off() {
        let config = AppConfig::default();
        config.validate().unwrap();
        assert_eq!(config.plugin_layout, 1);
        assert!(config
            .disabled_plugins
            .contains(&crate::plugins::DESKTOP_PET_ID.to_string()));
        assert!(config
            .disabled_plugins
            .contains(&crate::plugins::FONT_BEAUTIFY_ID.to_string()));
    }

    #[test]
    fn old_config_migrates_pet_and_font_defaults_once() {
        let old = r##"{"version":1,"accent":"#1f6fe5","greeting":"","widgets":["links"],"pluginOrder":[],"disabledPlugins":[],"petDesktopEnabled":false}"##;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        fs::write(&path, old).unwrap();
        let config = load(&path).unwrap();
        assert_eq!(config.plugin_layout, 1);
        assert!(config
            .disabled_plugins
            .contains(&crate::plugins::DESKTOP_PET_ID.to_string()));
        assert!(config
            .disabled_plugins
            .contains(&crate::plugins::FONT_BEAUTIFY_ID.to_string()));
        // The retired plugin-service fields are read once and never written.
        let saved = serde_json::to_value(&config).unwrap();
        assert!(saved.get("pluginServer").is_none());
        assert!(saved.get("petDesktopEnabled").is_none());
        assert_eq!(saved["pluginLayout"], 1);
    }

    #[test]
    fn an_in_use_desktop_pet_stays_enabled_across_the_upgrade() {
        let old = r##"{"version":1,"accent":"#1f6fe5","greeting":"","widgets":["links"],"pluginOrder":[],"disabledPlugins":[],"petDesktopEnabled":true,"petCharacter":"claude"}"##;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.json");
        fs::write(&path, old).unwrap();
        let config = load(&path).unwrap();
        assert!(!config
            .disabled_plugins
            .contains(&crate::plugins::DESKTOP_PET_ID.to_string()));
        assert!(config
            .disabled_plugins
            .contains(&crate::plugins::FONT_BEAUTIFY_ID.to_string()));
        // Legacy values survive parsing so the pet-settings migration can read them.
        assert!(config.pet_desktop_enabled);
        assert_eq!(config.pet_character, "claude");
    }

    #[test]
    fn wallpaper_display_settings_default_and_reject_bad_values() {
        let config = AppConfig::default();
        config.validate().unwrap();
        assert_eq!(config.wallpaper_display, "balanced");
        assert_eq!(config.wallpaper_panel.get("balanced"), Some(&72));
        let bad = AppConfig {
            wallpaper_display: "unknown".into(),
            ..AppConfig::default()
        };
        assert!(bad.validate().is_err());
        let bad = AppConfig {
            wallpaper_panel: [("vivid".to_string(), 10)].into_iter().collect(),
            ..AppConfig::default()
        };
        assert!(bad.validate().is_err());
        let bad = AppConfig {
            wallpaper_panel: [("neon".to_string(), 80)].into_iter().collect(),
            ..AppConfig::default()
        };
        assert!(bad.validate().is_err());
    }

    #[test]
    fn update_mirror_accepts_http_until_the_p0_cleanup() {
        for good in [
            "https://mirror.example/xjtu/latest.json",
            // P0 stopgap: the mirror is reachable only by IP for now.
            "http://192.0.2.10/xjtu-launcher/latest.json",
        ] {
            let config = AppConfig {
                update_endpoint: good.into(),
                ..AppConfig::default()
            };
            config.validate().unwrap();
        }
        for bad in [
            "http://user:pass@mirror.example/latest.json",
            "https://user:pass@mirror.example/latest.json",
            "ftp://mirror.example/latest.json",
            "not a url",
        ] {
            let config = AppConfig {
                update_endpoint: bad.into(),
                ..AppConfig::default()
            };
            assert!(config.validate().is_err(), "{bad}");
        }
    }

    #[test]
    fn mcp_tool_selection_must_be_slug_list() {
        let mut config = AppConfig {
            mcp_server_disabled_tools: vec!["campus_card".into()],
            ..AppConfig::default()
        };
        config.validate().unwrap();
        config.mcp_server_disabled_tools = vec!["campus_card".into(), "campus_card".into()];
        assert!(config.validate().is_err());
        config.mcp_server_disabled_tools = vec!["Campus Card".into()];
        assert!(config.validate().is_err());
        config.mcp_server_disabled_tools.clear();
        config.mcp_server_enabled = true;
        config.validate().unwrap();
        config.mcp_server_port = 1023;
        assert!(config.validate().is_err());
        config.mcp_server_port = 65535;
        config.validate().unwrap();
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
    fn personal_images_must_be_inline_data_urls() {
        let mut config = AppConfig {
            avatar: Some("data:image/png;base64,AAAA".into()),
            wallpaper: Some("data:image/jpeg;base64,AAAA".into()),
            ..AppConfig::default()
        };
        config.validate().unwrap();
        config.wallpaper = Some("data:image/png;base64,AAAA".into());
        config.validate().unwrap();
        config.avatar = Some("data:image/jpeg;base64,AAAA".into());
        assert!(config.validate().is_err());
        config.avatar = None;
        config.wallpaper = Some("https://example.com/wallpaper.jpg".into());
        assert!(config.validate().is_err());
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
