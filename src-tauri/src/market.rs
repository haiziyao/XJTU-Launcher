//! Versioned, data-only plugin packages. Never shares the campus HTTP client or credentials.
use crate::{
    config,
    plugins::{self, Plugin},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    time::Duration,
};
use url::Url;

pub const DEFAULT_SERVER: Option<&str> = option_env!("XJTU_PLUGIN_SERVER");
const PACKAGE_LIMIT: usize = 65536;

pub fn default_server() -> Option<&'static str> {
    DEFAULT_SERVER.filter(|value| !value.trim().is_empty())
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub min_app_version: String,
    pub download_url: String,
    pub sha256: String,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Catalog {
    pub schema_version: u32,
    pub plugins: Vec<Entry>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Installed {
    pub plugin: Plugin,
    pub source: String,
    pub sha256: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCatalog {
    pub source: Option<String>,
    pub default_source: Option<String>,
    pub updates: Vec<String>,
    pub plugins: Vec<Entry>,
}

pub fn source_url(value: &str) -> Result<Url, String> {
    let mut url = plugins::validate_url(value)?;
    if value.len() > 2048 || url.query().is_some() || url.fragment().is_some() {
        return Err("服务端地址不能包含查询参数或片段".into());
    }
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    if url.scheme() != "https" && !local {
        return Err("服务端请使用 HTTPS；本机调试可使用 HTTP".into());
    }
    if !url.path().ends_with('/') {
        url.set_path(&format!("{}/", url.path()));
    }
    Ok(url)
}

pub fn source(config: &config::AppConfig) -> Result<Option<String>, String> {
    config
        .plugin_server
        .as_deref()
        .or(default_server())
        .map(|s| source_url(s).map(|u| u.to_string()))
        .transpose()
}

fn package_url(source: &Url, entry: &Entry) -> Result<Url, String> {
    let url = source
        .join(&entry.download_url)
        .map_err(|_| "插件下载地址无效")?;
    if url.origin() != source.origin()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err("插件包必须来自当前服务端的同源地址".into());
    }
    Ok(url)
}

fn validate_catalog(catalog: &Catalog, source: &Url) -> Result<(), String> {
    if catalog.schema_version != 1 || catalog.plugins.len() > 256 {
        return Err("插件目录版本不受支持或项目过多".into());
    }
    let mut ids = HashSet::new();
    for entry in &catalog.plugins {
        if !plugins::valid_id(&entry.id)
            || [plugins::TIMETABLE_ID, plugins::CAMPUS_ID].contains(&entry.id.as_str())
            || !ids.insert(&entry.id)
            || entry.name.trim().is_empty()
            || entry.name.chars().count() > 30
            || entry.description.chars().count() > 300
            || entry.sha256.len() != 64
            || !entry.sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("插件目录包含无效或重复项目".into());
        }
        semver::Version::parse(&entry.version).map_err(|_| "插件版本无效")?;
        semver::Version::parse(&entry.min_app_version).map_err(|_| "客户端兼容版本无效")?;
        package_url(source, entry)?;
    }
    Ok(())
}

async fn fetch(
    url: Url,
    limit: usize,
    mut progress: impl FnMut(usize, Option<u64>),
) -> Result<Vec<u8>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法初始化插件下载")?;
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|_| "无法连接插件服务器，请检查地址和网络")?;
    if !response.status().is_success() {
        return Err(format!(
            "插件服务器返回 HTTP {}",
            response.status().as_u16()
        ));
    }
    let total = response.content_length();
    if total.is_some_and(|n| n > limit as u64) {
        return Err("插件响应超过大小限制".into());
    }
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "插件下载中断")? {
        if data.len() + chunk.len() > limit {
            return Err("插件响应超过大小限制".into());
        }
        data.extend_from_slice(&chunk);
        progress(data.len(), total);
    }
    Ok(data)
}

pub async fn catalog(config: &config::AppConfig) -> Result<RemoteCatalog, String> {
    let source = source(config)?;
    let mut result = RemoteCatalog {
        source: source.clone(),
        default_source: default_server().map(str::to_string),
        updates: vec![],
        plugins: vec![],
    };
    if let Some(source) = source {
        let url = source_url(&source)?;
        let data = fetch(
            url.join("catalog.json").map_err(|_| "目录地址无效")?,
            1024 * 1024,
            |_, _| {},
        )
        .await?;
        let catalog: Catalog = serde_json::from_slice(&data).map_err(|_| "服务器目录格式无效")?;
        validate_catalog(&catalog, &url)?;
        result.plugins = catalog.plugins;
    }
    Ok(result)
}

pub fn available_updates(catalog: &RemoteCatalog, installed: &plugins::PluginList) -> Vec<String> {
    catalog
        .plugins
        .iter()
        .filter(|entry| {
            installed.sources.get(&entry.id) == catalog.source.as_ref()
                && installed.plugins.iter().any(|plugin| {
                    plugin.id == entry.id
                        && match (
                            semver::Version::parse(&entry.version),
                            semver::Version::parse(&plugin.version),
                        ) {
                            (Ok(next), Ok(current)) => next > current,
                            _ => false,
                        }
                })
        })
        .map(|entry| entry.id.clone())
        .collect()
}

pub async fn download(
    source: &str,
    entry: &Entry,
    progress: impl FnMut(usize, Option<u64>),
) -> Result<Installed, String> {
    let app = semver::Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| "客户端版本无效")?;
    if app < semver::Version::parse(&entry.min_app_version).map_err(|_| "兼容版本无效")? {
        return Err("请先更新客户端再安装此插件".into());
    }
    let bytes = fetch(
        package_url(&source_url(source)?, entry)?,
        PACKAGE_LIMIT,
        progress,
    )
    .await?;
    validate_package(source, entry, &bytes)
}

fn validate_package(source: &str, entry: &Entry, bytes: &[u8]) -> Result<Installed, String> {
    let digest = hex::encode(Sha256::digest(bytes));
    if !digest.eq_ignore_ascii_case(&entry.sha256) {
        return Err("插件包校验失败，已保留原版本".into());
    }
    let plugin: Plugin =
        serde_json::from_slice(bytes).map_err(|_| "插件包格式无效，仅支持声明式导航插件")?;
    plugin.validate(false)?;
    if plugin.id != entry.id || plugin.version != entry.version || plugin.name != entry.name {
        return Err("插件包与目录声明不一致".into());
    }
    Ok(Installed {
        plugin,
        source: source_url(source)?.to_string(),
        sha256: digest,
    })
}

fn target(directory: &Path, id: &str) -> Result<PathBuf, String> {
    if !plugins::valid_id(id) || [plugins::TIMETABLE_ID, plugins::CAMPUS_ID].contains(&id) {
        return Err("不能移除或覆盖内置插件".into());
    }
    let path = directory.join(id);
    if path.exists()
        && (path
            .symlink_metadata()
            .map_err(|_| "插件目录不可访问")?
            .file_type()
            .is_symlink()
            || !path.is_dir())
    {
        return Err("插件安装路径无效".into());
    }
    Ok(path)
}

pub fn read_installed(path: &Path) -> Result<Installed, String> {
    if path
        .symlink_metadata()
        .map_err(|_| "无法读取插件来源")?
        .file_type()
        .is_symlink()
        || std::fs::metadata(path)
            .map_err(|_| "无法读取插件来源")?
            .len()
            > 128 * 1024
    {
        return Err("插件安装记录无效".into());
    }
    serde_json::from_slice(&std::fs::read(path).map_err(|_| "无法读取插件来源")?)
        .map_err(|_| "插件安装记录损坏".into())
}

pub fn install(directory: &Path, package: Installed) -> Result<(), String> {
    package.plugin.validate(false)?;
    let path = target(directory, &package.plugin.id)?;
    // One atomic file contains both manifest and origin; no half-updated receipt.
    if path.exists() {
        let old = read_installed(&path.join("installed.json"))
            .map_err(|_| "已存在本地插件，请先移除后再安装远端版本")?;
        if old.source != package.source {
            return Err("同名插件来自其他服务端，请先移除后再安装".into());
        }
        let old_version =
            semver::Version::parse(&old.plugin.version).map_err(|_| "已安装版本无效")?;
        let next = semver::Version::parse(&package.plugin.version).map_err(|_| "插件版本无效")?;
        if next <= old_version {
            return Err("已安装相同或更高版本".into());
        }
    }
    let bytes = serde_json::to_vec_pretty(&package).map_err(|_| "无法保存插件")?;
    if path.exists() {
        config::write_atomic(&path.join("installed.json"), &bytes)
    } else {
        std::fs::create_dir_all(directory).map_err(|_| "无法创建插件目录")?;
        let staging = tempfile::Builder::new()
            .prefix(".install-")
            .tempdir_in(directory)
            .map_err(|_| "无法暂存插件")?;
        config::write_atomic(&staging.path().join("installed.json"), &bytes)?;
        std::fs::rename(staging.path(), path).map_err(|_| "无法完成插件安装".to_string())
    }
}

pub fn remove(directory: &Path, id: &str) -> Result<(), String> {
    let path = target(directory, id)?;
    if !path.exists() {
        return Err("插件已经移除".into());
    }
    // Rename out of discovery first; failure never leaves a partially active plugin.
    let trash = tempfile::tempdir_in(directory.parent().ok_or("插件目录无效")?)
        .map_err(|_| "无法准备移除插件")?;
    std::fs::rename(path, trash.path().join(id)).map_err(|_| "无法移除插件")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(version: &str) -> (Entry, Vec<u8>) {
        let mut plugin: Plugin =
            serde_json::from_str(include_str!("../../examples/campus-links/plugin.json")).unwrap();
        plugin.version = version.into();
        let bytes = serde_json::to_vec(&plugin).unwrap();
        (
            Entry {
                id: plugin.id,
                name: plugin.name,
                description: plugin.description,
                version: version.into(),
                min_app_version: "0.1.0".into(),
                download_url: "packages/test.json".into(),
                sha256: hex::encode(Sha256::digest(&bytes)),
            },
            bytes,
        )
    }
    #[test]
    fn validates_source_origin_and_catalog() {
        for value in [
            "file:///tmp",
            "https://user:pw@server",
            "http://remote.test",
            "https://server/?secret=1",
        ] {
            assert!(source_url(value).is_err());
        }
        assert!(source_url("http://127.0.0.1:17321").is_ok());
        let source = source_url("https://server.test/plugins").unwrap();
        let (mut entry, _) = fixture("1.0.0");
        assert!(package_url(&source, &entry).is_ok());
        entry.download_url = "https://other.test/a".into();
        assert!(package_url(&source, &entry).is_err());
        let (entry, _) = fixture("1.0.0");
        assert!(validate_catalog(
            &Catalog {
                schema_version: 1,
                plugins: vec![entry.clone(), entry]
            },
            &source
        )
        .is_err());
    }
    #[test]
    fn installs_updates_and_preserves_version_on_failure() {
        let dir = tempfile::tempdir().unwrap();
        let (entry, bytes) = fixture("1.0.0");
        let package = validate_package("https://server.test", &entry, &bytes).unwrap();
        install(dir.path(), package.clone()).unwrap();
        assert_eq!(plugins::list(dir.path()).unwrap().plugins.len(), 3);
        assert!(install(dir.path(), package).is_err());
        let (entry2, bytes2) = fixture("1.1.0");
        assert!(validate_package("https://server.test", &entry2, &bytes).is_err());
        assert!(install(
            dir.path(),
            validate_package("https://other.test", &entry2, &bytes2).unwrap()
        )
        .is_err());
        assert_eq!(
            read_installed(&dir.path().join(&entry.id).join("installed.json"))
                .unwrap()
                .plugin
                .version,
            "1.0.0"
        );
        install(
            dir.path(),
            validate_package("https://server.test", &entry2, &bytes2).unwrap(),
        )
        .unwrap();
        assert_eq!(
            read_installed(&dir.path().join(&entry.id).join("installed.json"))
                .unwrap()
                .plugin
                .version,
            "1.1.0"
        );
        assert!(remove(dir.path(), "../escape").is_err());
        assert!(remove(dir.path(), plugins::CAMPUS_ID).is_err());
        remove(dir.path(), &entry.id).unwrap();
        assert_eq!(plugins::list(dir.path()).unwrap().plugins.len(), 2);
    }
}
