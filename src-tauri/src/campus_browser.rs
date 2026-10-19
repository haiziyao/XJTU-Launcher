//! Isolated, ephemeral campus webviews. Credentials stay in the native cookie store;
//! remote pages have no launcher capabilities and never receive account passwords.
use crate::network::{self, AccessMode};
use cookie_store::{CookieDomain, CookieExpiration};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tauri::{
    menu::{Menu, MenuItem, Submenu},
    webview::{Cookie, NewWindowResponse},
    AppHandle, Manager, WebviewUrl, WebviewWindow, WebviewWindowBuilder, WindowEvent,
};
use tauri_plugin_opener::OpenerExt;
use url::Url;

const PREFIX: &str = "campus-web-";
pub struct Session {
    pub account_id: String,
    pub generation: u64,
    pub display_name: String,
    pub mode: AccessMode,
    pub cookies: String,
}

fn campus_domain(host: &str) -> bool {
    host == "xjtu.edu.cn" || host.ends_with(".xjtu.edu.cn")
}

fn session_cookies(snapshot: &str) -> Result<Vec<Cookie<'static>>, String> {
    let store = cookie_store::serde::json::load(snapshot.as_bytes())
        .map_err(|_| "校园会话无法读取，请重新登录")?;
    let mut result = Vec::new();
    for stored in store.iter_unexpired() {
        let host = match &stored.domain {
            CookieDomain::HostOnly(host) | CookieDomain::Suffix(host) if campus_domain(host) => {
                host
            }
            _ => continue,
        };
        // Preserve Secure, HttpOnly and SameSite from the original Set-Cookie.
        let mut cookie: Cookie<'static> = std::ops::Deref::deref(stored).clone().into_owned();
        cookie.set_domain(host.clone());
        cookie.set_path(stored.path.to_string());
        // Never reset a relative Max-Age when restoring an older session.
        cookie.set_max_age(None);
        match stored.expires {
            CookieExpiration::AtUtc(expires) => cookie.set_expires(expires),
            CookieExpiration::SessionEnd => cookie.unset_expires(),
        }
        result.push(cookie);
    }
    Ok(result)
}

fn menu_items(app: &AppHandle) -> tauri::Result<Submenu<tauri::Wry>> {
    Submenu::with_items(
        app,
        "校园网页",
        true,
        &[
            &MenuItem::with_id(app, "campus.back", "后退", true, Some("CmdOrCtrl+["))?,
            &MenuItem::with_id(app, "campus.forward", "前进", true, Some("CmdOrCtrl+]"))?,
            &MenuItem::with_id(app, "campus.reload", "刷新", true, Some("CmdOrCtrl+R"))?,
            &MenuItem::with_id(
                app,
                "campus.external",
                "在系统浏览器中打开",
                true,
                None::<&str>,
            )?,
        ],
    )
}

pub fn install_menu(app: &AppHandle) -> tauri::Result<()> {
    // macOS menus belong to the app. Windows gets a menu only on the campus window.
    #[cfg(target_os = "macos")]
    {
        let menu = app.menu().map(Ok).unwrap_or_else(|| Menu::default(app))?;
        menu.append(&menu_items(app)?)?;
        app.set_menu(menu)?;
    }
    let _ = app;
    Ok(())
}

pub fn menu_event(app: &AppHandle, id: &str) {
    if !id.starts_with("campus.") {
        return;
    }
    let Some(window) = app
        .webview_windows()
        .into_values()
        .find(|w| w.label().starts_with(PREFIX) && w.is_focused().unwrap_or(false))
    else {
        return;
    };
    match id {
        "campus.back" => {
            let _ = window.eval("history.back()");
        }
        "campus.forward" => {
            let _ = window.eval("history.forward()");
        }
        "campus.reload" => {
            let _ = window.eval("location.reload()");
        }
        "campus.external" => {
            if let Ok(url) = window.url() {
                if network::campus_web_url(&url) {
                    let _ = app.opener().open_url(url.as_str(), None::<&str>);
                }
            }
        }
        _ => {}
    }
}

/// Called while the account lock is held, before switching/removing/reauthenticating.
pub fn close_all(app: &AppHandle) -> Result<(), String> {
    for window in app
        .webview_windows()
        .into_values()
        .filter(|w| w.label().starts_with(PREFIX))
    {
        window
            .destroy()
            .map_err(|_| "校园网页未能关闭，请关闭后再切换账号")?;
    }
    Ok(())
}

fn open_external(app: &AppHandle, url: &Url) {
    if crate::plugins::validate_url(url.as_str()).is_ok() {
        let _ = app.opener().open_url(url.as_str(), None::<&str>);
    }
}

// Queue navigation outside WebKit/WebView2's navigation delegate to avoid re-entry.
fn navigate_later(app: AppHandle, label: String, url: Url, live: Arc<AtomicBool>) {
    tauri::async_runtime::spawn(async move {
        if live.load(Ordering::SeqCst) {
            if let Some(window) = app.get_webview_window(&label) {
                let _ = window.navigate(url);
            }
        }
    });
}

/// Must run from an async command/worker, never a synchronous UI callback.
pub fn open(app: &AppHandle, session: Session, target: Url) -> Result<WebviewWindow, String> {
    uuid::Uuid::parse_str(&session.account_id).map_err(|_| "校园账号标识无效")?;
    let target = network::browser_address(&target, session.mode)?;
    let label = format!("{PREFIX}{}-{}", session.account_id, session.generation);
    if let Some(window) = app.get_webview_window(&label) {
        window.navigate(target).map_err(|_| "无法打开校园页面")?;
        window.show().map_err(|_| "无法显示校园窗口")?;
        window.set_focus().map_err(|_| "无法激活校园窗口")?;
        return Ok(window);
    }
    let cookies = session_cookies(&session.cookies)?;
    let live = Arc::new(AtomicBool::new(true));
    let ready = Arc::new(AtomicBool::new(false));
    // A fresh WebView2 profile per window prevents two accounts sharing InPrivate state.
    // WKWebView uses its own nonpersistent store via incognito on macOS 12+.
    let profile = Arc::new(
        tempfile::Builder::new()
            .prefix("xjtu-campus-web-")
            .tempdir()
            .map_err(|_| "无法创建独立校园浏览会话")?,
    );
    let title = format!("校园网页 · {}", session.display_name);
    let nav_app = app.clone();
    let nav_label = label.clone();
    let nav_live = live.clone();
    let nav_ready = ready.clone();
    let mode = session.mode;
    let popup_app = app.clone();
    let popup_label = label.clone();
    let popup_live = live.clone();
    let builder = WebviewWindowBuilder::new(
        app,
        &label,
        WebviewUrl::External(Url::parse("about:blank").unwrap()),
    )
    .title(&title)
    .inner_size(1180., 800.)
    .min_inner_size(800., 600.)
    .visible(false)
    .incognito(true)
    .data_directory(profile.path().to_owned())
    .user_agent(network::USER_AGENT)
    .on_navigation(move |url| {
        if !nav_live.load(Ordering::SeqCst) {
            return false;
        }
        if url.as_str() == "about:blank" {
            return !nav_ready.load(Ordering::SeqCst);
        }
        if !nav_ready.load(Ordering::SeqCst) {
            return false;
        }
        if let Ok(mapped) = network::browser_address(url, mode) {
            if mapped == *url {
                true
            } else {
                navigate_later(nav_app.clone(), nav_label.clone(), mapped, nav_live.clone());
                false
            }
        } else {
            open_external(&nav_app, url);
            false
        }
    })
    .on_new_window(move |url, _| {
        if popup_live.load(Ordering::SeqCst) {
            if let Ok(mapped) = network::browser_address(&url, mode) {
                navigate_later(
                    popup_app.clone(),
                    popup_label.clone(),
                    mapped,
                    popup_live.clone(),
                );
            } else {
                open_external(&popup_app, &url);
            }
        }
        NewWindowResponse::Deny
    })
    .on_document_title_changed(move |window, page_title| {
        let short: String = page_title
            .chars()
            .filter(|c| !c.is_control())
            .take(100)
            .collect();
        let _ = window.set_title(&if short.trim().is_empty() {
            title.clone()
        } else {
            format!("{short} — {title}")
        });
    });
    #[cfg(not(target_os = "macos"))]
    let builder = builder.menu(
        Menu::with_items(app, &[&menu_items(app).map_err(|_| "无法创建网页菜单")?])
            .map_err(|_| "无法创建网页菜单")?,
    );
    let window = builder.build().map_err(|_| "无法创建校园网页窗口")?;
    let closed = live.clone();
    window.on_window_event(move |event| {
        let _keep_profile_alive = &profile;
        if matches!(event, WindowEvent::Destroyed) {
            closed.store(false, Ordering::SeqCst);
        }
    });
    let result = (|| {
        // Native setters include HttpOnly cookies and complete before the first request.
        for cookie in cookies {
            window
                .set_cookie(cookie)
                .map_err(|_| "无法恢复校园网页登录状态")?;
        }
        ready.store(true, Ordering::SeqCst);
        window.navigate(target).map_err(|_| "无法打开校园页面")?;
        window.show().map_err(|_| "无法显示校园窗口")?;
        window.set_focus().map_err(|_| "无法激活校园窗口")?;
        Ok(())
    })();
    if let Err(error) = result {
        live.store(false, Ordering::SeqCst);
        let _ = window.destroy();
        return Err(error);
    }
    Ok(window)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn snapshot() -> String {
        let mut store = cookie_store::CookieStore::default();
        let school = Url::parse("https://login.xjtu.edu.cn/cas/login").unwrap();
        store
            .parse(
                "TGC=test-only; Secure; HttpOnly; SameSite=Lax; Path=/cas; Max-Age=600",
                &school,
            )
            .unwrap();
        store.parse("session=local; Secure", &school).unwrap();
        assert!(store.parse("expired=gone; Max-Age=0", &school).is_err());
        store
            .parse(
                "other=private; Secure",
                &Url::parse("https://example.com/").unwrap(),
            )
            .unwrap();
        let mut bytes = Vec::new();
        cookie_store::serde::json::save_incl_expired_and_nonpersistent(&store, &mut bytes).unwrap();
        String::from_utf8(bytes).unwrap()
    }
    #[test]
    fn restores_native_cookie_flags_scope_and_absolute_expiry() {
        let cookies = session_cookies(&snapshot()).unwrap();
        assert_eq!(cookies.len(), 2);
        let cookie = cookies.iter().find(|c| c.name() == "TGC").unwrap();
        assert_eq!(cookie.domain(), Some("login.xjtu.edu.cn"));
        assert_eq!(cookie.path(), Some("/cas"));
        assert_eq!(cookie.secure(), Some(true));
        assert_eq!(cookie.http_only(), Some(true));
        assert_eq!(
            cookie.same_site().map(|s| s.to_string()),
            Some("Lax".into())
        );
        assert!(cookie.max_age().is_none());
        assert!(cookie.expires_datetime().is_some());
        let session = cookies.iter().find(|c| c.name() == "session").unwrap();
        assert_eq!(session.path(), Some("/cas"));
        assert!(session.expires_datetime().is_none());
    }
    #[test]
    fn browser_routing_does_not_expand_backend_authentication_allowlist() {
        for host in [
            "syxt.xjtu.edu.cn",
            "identity1.xjtu.edu.cn",
            "lms.xjtu.edu.cn",
        ] {
            let url = Url::parse(&format!("https://{host}/path?a=1#section")).unwrap();
            assert_eq!(
                network::browser_address(&url, AccessMode::Direct).unwrap(),
                url
            );
            let vpn = network::browser_address(&url, AccessMode::Webvpn).unwrap();
            assert_eq!(vpn.host_str(), Some("webvpn.xjtu.edu.cn"));
            assert_eq!(vpn.query(), url.query());
            assert_eq!(vpn.fragment(), url.fragment());
            assert_eq!(
                network::browser_address(&vpn, AccessMode::Webvpn).unwrap(),
                vpn
            );
            assert!(!network::trusted(&url));
        }
        for invalid in [
            "https://xjtu.edu.cn.evil.test",
            "https://evilxjtu.edu.cn",
            "http://login.xjtu.edu.cn",
            "https://u:p@login.xjtu.edu.cn",
            "https://login.xjtu.edu.cn:8443",
            "file:///etc/passwd",
            "tauri://localhost",
        ] {
            assert!(
                network::browser_address(&Url::parse(invalid).unwrap(), AccessMode::Direct)
                    .is_err()
            );
        }
    }
}
