use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, path::Path};
use url::Url;

pub const TIMETABLE_ID: &str = "cn.xjtu.timetable";
pub const CAMPUS_ID: &str = "cn.xjtu.campus";
pub const AGENT_ID: &str = "cn.xjtu.agent-settings";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plugin {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub icon: String,
    pub page: Page,
    pub widget: bool,
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
    pub warnings: Vec<String>,
    pub directory: String,
    pub sources: std::collections::HashMap<String, String>,
}

pub fn valid_id(id: &str) -> bool {
    (3..=80).contains(&id.len())
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && !id.contains("..")
        && id
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-')
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
    pub fn validate(&self, builtin: bool) -> Result<(), String> {
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
            || !valid_id(&self.id)
            || self.name.trim().is_empty()
            || self.name.chars().count() > 30
            || self.description.chars().count() > 300
            || self.version.len() > 32
            || self.icon.len() > 30
        {
            return Err("插件清单字段无效或版本不受支持".into());
        }
        if [TIMETABLE_ID, CAMPUS_ID, AGENT_ID].contains(&self.id.as_str()) && !builtin {
            return Err("不能覆盖内置插件".into());
        }
        match &self.page {
            Page::Timetable if !builtin => return Err("课表服务目前仅供内置插件使用".into()),
            Page::Campus if !builtin => return Err("畅行校园仅供内置插件使用".into()),
            Page::AgentSettings if !builtin => return Err("Agent 设置仅供内置插件使用".into()),
            Page::Links { links } => {
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
            _ => {}
        }
        Ok(())
    }
}

fn read(path: &Path) -> Result<Plugin, String> {
    if fs::metadata(path).map_err(|_| "找不到 plugin.json")?.len() > 65536 {
        return Err("插件清单不能超过 64 KiB".into());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取插件清单")?;
    serde_json::from_slice(&bytes).map_err(|_| "plugin.json 格式错误或包含未知字段".into())
}

pub fn list(directory: &Path) -> Result<PluginList, String> {
    fs::create_dir_all(directory).map_err(|_| "无法创建插件目录")?;
    let builtin: Plugin =
        serde_json::from_str(include_str!("../../plugins/cn.xjtu.timetable/plugin.json"))
            .map_err(|_| "内置课表插件损坏")?;
    builtin.validate(true)?;
    let campus: Plugin =
        serde_json::from_str(include_str!("../../plugins/cn.xjtu.campus/plugin.json"))
            .map_err(|_| "内置畅行校园插件损坏")?;
    campus.validate(true)?;
    let agent: Plugin = serde_json::from_str(include_str!(
        "../../plugins/cn.xjtu.agent-settings/plugin.json"
    ))
    .map_err(|_| "内置 Agent 设置插件损坏")?;
    agent.validate(true)?;
    let mut result = PluginList {
        plugins: vec![builtin, campus, agent],
        sources: Default::default(),
        warnings: vec![],
        directory: directory.display().to_string(),
    };
    let mut paths: Vec<_> = fs::read_dir(directory)
        .map_err(|_| "无法读取插件目录")?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .collect();
    paths.sort();
    let mut ids = HashSet::from([
        TIMETABLE_ID.to_string(),
        CAMPUS_ID.to_string(),
        AGENT_ID.to_string(),
    ]);
    for path in paths.into_iter().take(256) {
        if !path.is_dir()
            || path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with('.'))
        {
            continue;
        }
        let checked: Result<Plugin, String> = (|| {
            let real = path.canonicalize().map_err(|_| "无法定位插件目录")?;
            let root = directory.canonicalize().map_err(|_| "无法定位插件根目录")?;
            let installed = real.join("installed.json");
            let manifest = real
                .join(if installed.exists() {
                    "installed.json"
                } else {
                    "plugin.json"
                })
                .canonicalize()
                .map_err(|_| "找不到 plugin.json")?;
            if !real.starts_with(&root) || !manifest.starts_with(&real) {
                return Err("不支持目录外的符号链接".into());
            }
            let (plugin, source) = if installed.exists() {
                let package = crate::market::read_installed(&manifest)?;
                (package.plugin, Some(package.source))
            } else {
                (read(&manifest)?, None)
            };
            plugin.validate(false)?;
            if !ids.insert(plugin.id.clone()) {
                return Err("插件 ID 重复".into());
            }
            if path.file_name().and_then(|n| n.to_str()) != Some(&plugin.id) {
                return Err("插件目录与 ID 不一致".into());
            }
            if let Some(source) = source {
                result.sources.insert(plugin.id.clone(), source);
            }
            Ok(plugin)
        })();
        match checked {
            Ok(plugin) => result.plugins.push(plugin),
            Err(error) => result.warnings.push(format!(
                "{}：{}",
                path.file_name().unwrap_or_default().to_string_lossy(),
                error
            )),
        }
    }
    Ok(result)
}

pub fn import(source: &Path, directory: &Path) -> Result<(), String> {
    let plugin = read(&source.join("plugin.json"))?;
    plugin.validate(false)?;
    let target = directory.join(&plugin.id);
    if target.exists() {
        return Err("该插件已存在；请直接修改插件目录中的清单后刷新".into());
    }
    let bytes = serde_json::to_vec_pretty(&plugin).map_err(|_| "无法序列化插件")?;
    crate::config::write_atomic(&target.join("plugin.json"), &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_and_discovers_a_local_plugin() {
        let directory = tempfile::tempdir().unwrap();
        import(Path::new("../examples/campus-links"), directory.path()).unwrap();
        let result = list(directory.path()).unwrap();
        assert_eq!(result.plugins.len(), 4);
        assert!(result.warnings.is_empty());
        assert!(import(Path::new("../examples/campus-links"), directory.path()).is_err());
    }

    #[test]
    fn rejects_paths_scripts_and_builtin_impersonation() {
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
        let plugin: Plugin =
            serde_json::from_str(include_str!("../../plugins/cn.xjtu.timetable/plugin.json"))
                .unwrap();
        assert!(plugin.validate(false).is_err());
    }

    #[test]
    fn bad_plugin_does_not_break_other_plugins() {
        let directory = tempfile::tempdir().unwrap();
        fs::create_dir(directory.path().join("bad")).unwrap();
        fs::write(directory.path().join("bad/plugin.json"), "{}").unwrap();
        let result = list(directory.path()).unwrap();
        assert_eq!(result.plugins.len(), 3);
        assert_eq!(result.warnings.len(), 1);
    }
}
