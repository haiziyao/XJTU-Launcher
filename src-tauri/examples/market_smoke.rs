//! Exercises the actual catalog/downloader against the bundled local server, in a temporary directory.
#![allow(dead_code)]
#[path = "../src/config.rs"]
mod config;
#[path = "../src/market.rs"]
mod market;
#[path = "../src/plugins.rs"]
mod plugins;
#[path = "../src/tool_names.rs"]
mod tool_names;
fn main() {
    tauri::async_runtime::block_on(async {
        let config = config::AppConfig {
            plugin_server: Some("http://127.0.0.1:17321".into()),
            ..Default::default()
        };
        let catalog = market::catalog(&config).await.expect("catalog");
        let entry = catalog.plugins.first().expect("sample package");
        let source = catalog.source.as_deref().unwrap();
        let package = market::download(source, entry, |_, _| {})
            .await
            .expect("download and digest");
        let directory = tempfile::tempdir().unwrap();
        market::install(directory.path(), package).expect("install");
        let installed = plugins::list(directory.path()).unwrap();
        assert_eq!(installed.plugins.len(), 3);
        assert_eq!(
            installed.sources.get(&entry.id).map(String::as_str),
            Some(source)
        );
        let mut tampered = entry.clone();
        tampered.sha256 = "0".repeat(64);
        assert!(market::download(source, &tampered, |_, _| {})
            .await
            .is_err());
        assert_eq!(plugins::list(directory.path()).unwrap().plugins.len(), 3);
        market::remove(directory.path(), &entry.id).expect("remove");
        assert_eq!(plugins::list(directory.path()).unwrap().plugins.len(), 2);
        println!("PASS: real HTTP catalog, package download, digest validation, atomic install, origin persistence, failed download preservation, removal");
    });
}
