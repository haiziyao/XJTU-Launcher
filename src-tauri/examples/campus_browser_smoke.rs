//! Run manually on a desktop: cargo run --example campus_browser_smoke
//! Exercises real native cookie stores with synthetic cookies only. No user vault is read.
#![allow(dead_code)] // Reuse production modules without exposing them as public library APIs.
#[path = "../src/campus.rs"]
mod campus;
#[path = "../src/campus_browser.rs"]
mod campus_browser;
#[path = "../src/config.rs"]
mod config;
#[path = "../src/market.rs"]
mod market;
#[path = "../src/network.rs"]
mod network;
#[path = "../src/plugins.rs"]
mod plugins;
#[path = "../src/vault.rs"]
mod vault;

use tauri::Manager;
use url::Url;

fn session(value: &str) -> campus_browser::Session {
    let mut jar = cookie_store::CookieStore::default();
    jar.parse(
        &format!("launcher-native-probe={value}; Path=/cas; Secure; HttpOnly; SameSite=Lax"),
        &Url::parse("https://login.xjtu.edu.cn/cas/login").unwrap(),
    )
    .unwrap();
    let mut bytes = Vec::new();
    cookie_store::serde::json::save_incl_expired_and_nonpersistent(&jar, &mut bytes).unwrap();
    campus_browser::Session {
        account_id: uuid::Uuid::new_v4().to_string(),
        generation: 1,
        display_name: "本地窗口验证".into(),
        mode: network::AccessMode::Direct,
        cookies: String::from_utf8(bytes).unwrap(),
    }
}
fn check(app: &tauri::AppHandle) -> Result<(), String> {
    campus_browser::install_menu(app).map_err(|_| "native menu")?;
    let url = Url::parse("https://login.xjtu.edu.cn/cas/login").unwrap();
    let a = campus_browser::open(app, session("first"), url.clone())?;
    let b = campus_browser::open(app, session("second"), url.clone())?;
    for (window, expected) in [(&a, "first"), (&b, "second")] {
        let cookies = window
            .cookies_for_url(url.clone())
            .map_err(|_| "native cookie read")?;
        let c = cookies
            .iter()
            .find(|c| c.name() == "launcher-native-probe")
            .ok_or("missing imported cookie")?;
        if c.value() != expected
            || c.secure() != Some(true)
            || c.http_only() != Some(true)
            || c.path() != Some("/cas")
        {
            return Err("cookie isolation or flags mismatch".into());
        }
    }
    println!("PASS: native cookie import, HttpOnly/Secure/path, independent account stores");
    campus_browser::close_all(app)?;
    for _ in 0..50 {
        if app.webview_windows().is_empty() {
            println!("PASS: account switch closes all campus windows");
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Err("campus windows did not close".into())
}
fn main() {
    let mut context = tauri::generate_context!();
    context.config_mut().app.windows.clear();
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn_blocking(move || {
                let result = check(&app);
                let _ = campus_browser::close_all(&app);
                let code = if let Err(error) = result {
                    eprintln!("FAIL: {error}");
                    1
                } else {
                    0
                };
                app.exit(code);
            });
            Ok(())
        })
        .build(context)
        .expect("native smoke app build")
        .run(|_, event| {
            // Keep the event loop alive until explicit exit after lifecycle checks.
            if let tauri::RunEvent::ExitRequested {
                code: None, api, ..
            } = event
            {
                api.prevent_exit();
            }
        });
}
