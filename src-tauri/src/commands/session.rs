//! Tauri commands for the session domain.

use super::accounts::account_changed;
use crate::*;
use serde::Serialize;
use std::sync::atomic::Ordering;
use tauri::WebviewWindow;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NetworkCheck {
    /// campus | vpn | manual | neutral
    pub(crate) tone: &'static str,
    pub(crate) message: String,
    /// Whether the account holds a direct CAS session for card/fitness.
    pub(crate) direct_session: bool,
}

pub(crate) enum SessionRefresh {
    Refreshed,
    NeedsManual,
}

/// Re-run the saved account's login with its stored password. Used by the
/// sidebar self-check and when the LMS reports the saved session has expired;
/// the password never leaves this process.
pub(crate) async fn refresh_saved_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<SessionRefresh, String> {
    let path = accounts_path(app)?;
    let (account_id, username, degree, password) = {
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("未绑定校园账号")?;
        let saved = record.password.as_ref().ok_or("此账号未保存密码")?;
        (
            record.id.clone(),
            record.username.clone(),
            record.degree.clone(),
            saved.0.clone(),
        )
    };
    let mut login = auth::Login::begin(degree, device_visitor(app)?).await?;
    login
        .act(auth::Action::Credentials {
            username,
            password,
            remember_password: true,
            captcha: String::new(),
            // An unattended renewal must not change the device-trust choice.
            trust_device: false,
        })
        .await?;
    // The campus CAS can return an intermediate confirmation page with no input;
    // submitting "continue" is what the login dialog's second click does.
    for _ in 0..2 {
        if login.completed.is_some() {
            break;
        }
        let view = login.view();
        if view.phase != auth::Phase::Credentials || !view.can_continue {
            break;
        }
        login
            .act(auth::Action::Continue {
                captcha: String::new(),
            })
            .await?;
    }
    let Some(account) = login.completed.take() else {
        // A captcha, MFA or identity step still needs the login dialog.
        return Ok(SessionRefresh::NeedsManual);
    };
    let _guard = state.auth_lock.lock().await;
    let mut book = accounts::load(&path)?;
    let previous = match book.active() {
        Some(active) if active.id == account_id => active.clone(),
        _ => return Ok(SessionRefresh::NeedsManual),
    };
    // A CAS login response carries no per-service sessions and may omit the
    // saved password and direct session. Keep the stored ones so renewing the
    // main session does not sign the user out of LMS, the profile site or the
    // other campus services.
    let mut account = account;
    if account.sites.is_empty() {
        account.sites = previous.sites;
    }
    if account.direct_cookies.is_none() {
        account.direct_cookies = previous.direct_cookies;
    }
    if account.password.is_none() {
        account.password = previous.password;
    }
    book.upsert(account)?;
    accounts::save(&path, &book)?;
    drop(_guard);
    state.generation.fetch_add(1, Ordering::SeqCst);
    campus_browser::close_all(app)?;
    account_changed(app, "校园会话已刷新");
    Ok(SessionRefresh::Refreshed)
}

/// Signs in to a service in the account's current mode. In WebVPN mode the
/// portal session can expire on its own while each service cookie still looks
/// present; when the login bounces to the portal page, sign in to WebVPN again
/// and retry once. The refreshed portal jar is returned so the caller can keep
/// `account.cookies` current for the other services that share it.
async fn service_login_renewing_portal(
    service: &str,
    mode: network::AccessMode,
    portal: &str,
    username: &str,
    password: &str,
    visitor: &str,
) -> Result<(network::Transport, network::Page, Option<String>), String> {
    let seed = (mode == network::AccessMode::Webvpn).then_some(portal);
    let first = auth::service_login_landing(service, mode, seed, username, password, visitor).await;
    match first {
        Err(error) if error == auth::PORTAL_EXPIRED => {}
        other => {
            let (transport, page) = other?;
            return Ok((transport, page, None));
        }
    }
    let portal = auth::portal_login(username, password, visitor).await?;
    let (transport, page) =
        auth::service_login_landing(service, mode, Some(&portal), username, password, visitor)
            .await?;
    Ok((transport, page, Some(portal)))
}

/// Renew the LMS's own CAS cookie session with the saved password. The main
/// CAS session can still be valid when only the LMS one expired, so this is a
/// service login rather than a full account refresh. Saving the account must
/// not drop its other site sessions; `Book::upsert` preserves them.
pub(crate) async fn renew_lms_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = accounts_path(app)?;
    let (account_id, username, password, mode, portal) = {
        let _guard = state.auth_lock.lock().await;
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("请先登录校园账号")?;
        let saved = record
            .password
            .as_ref()
            .ok_or("此账号未保存密码，无法自动登录思源学堂")?;
        (
            record.id.clone(),
            record.username.clone(),
            zeroize::Zeroizing::new(saved.0.clone()),
            record.mode,
            record.cookies.clone(),
        )
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let visitor = device_visitor(app)?;
    let (transport, _, refreshed) = service_login_renewing_portal(
        school::LMS_LOGIN,
        mode,
        &portal,
        &username,
        &password,
        &visitor,
    )
    .await?;
    let cookies = transport.snapshot()?;
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新打开".into());
    }
    let mut book = accounts::load(&path)?;
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    if let Some(portal) = refreshed {
        account.cookies = portal;
    }
    account.sites.insert(
        "lms".into(),
        accounts::SiteSession {
            cookies: Some(cookies),
            token: None,
            system_type: None,
        },
    );
    accounts::save(&path, &book)?;
    Ok(())
}

/// Services such as the campus card refuse WebVPN sessions, so they get a CAS
/// session of their own. The app warms this up as soon as the account is ready;
/// the self-check and the first direct-only tool also call it as a fallback.
/// A captcha or phone step falls back to the "VPN 下不可用" label. `force`
/// bypasses the "already prepared" shortcut when a stored session has expired.
#[tauri::command]
pub(crate) async fn refresh_direct_session(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
    force: bool,
) -> Result<bool, String> {
    main_only(&window)?;
    renew_direct_session(&app, &state, force).await?;
    Ok(true)
}

/// Shared by the command above, the startup warm-up and the self-check.
pub(crate) async fn renew_direct_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
    force: bool,
) -> Result<(), String> {
    let _flight = state.direct_lock.lock().await;
    let path = accounts_path(app)?;
    let (account_id, username, password) = {
        let _guard = state.auth_lock.lock().await;
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("请先登录校园账号")?;
        // Another call may have finished the same warm-up while this one waited.
        if !force && record.direct_cookies.is_some() {
            return Ok(());
        }
        let saved = record
            .password
            .as_ref()
            .ok_or("此账号未保存密码，无法自动直连")?;
        (
            record.id.clone(),
            record.username.clone(),
            zeroize::Zeroizing::new(saved.0.clone()),
        )
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let visitor = device_visitor(app)?;
    let cookies = auth::direct_login(school::CARD_LOGIN, &username, &password, &visitor).await?;
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新打开".into());
    }
    let mut book = accounts::load(&path)?;
    book.accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?
        .direct_cookies = Some(cookies);
    accounts::save(&path, &book)
}

/// The fitness platform keeps its own H5 session, which the shared direct jar
/// cannot satisfy on its own. Log in through CAS, capture the launch tokens
/// from the callback fragment and exchange them for the legacy PHP session.
#[tauri::command]
pub(crate) async fn refresh_fitness_session(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    main_only(&window)?;
    renew_fitness_session(&app, &state).await?;
    Ok(true)
}

/// Shared by the command above and the query retry: the launch-token exchange
/// is read-only, so an expired stored session can be renewed without asking
/// the user again.
pub(crate) async fn renew_fitness_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = accounts_path(app)?;
    let (account_id, username, password) = {
        let _guard = state.auth_lock.lock().await;
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("请先登录校园账号")?;
        let saved = record
            .password
            .as_ref()
            .ok_or("此账号未保存密码，无法自动登录体测系统")?;
        (
            record.id.clone(),
            record.username.clone(),
            zeroize::Zeroizing::new(saved.0.clone()),
        )
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let visitor = device_visitor(app)?;
    let (transport, mut page) =
        auth::direct_login_landing(school::FITNESS_LOGIN, &username, &password, &visitor).await?;
    // The callback carries the launch tokens in its URL fragment. If the first
    // pass stopped on an intermediate page, ask again now that CAS cookies are
    // in the jar.
    let mut launch = school::sports::Launch::parse(&page.url).ok();
    if launch.is_none() {
        let service = url::Url::parse(school::FITNESS_LOGIN).map_err(|_| "体测登录地址无效")?;
        page = transport
            .page(&service, network::AccessMode::Direct, "undergraduate")
            .await?;
        launch = school::sports::Launch::parse(&page.url).ok();
    }
    let launch = launch.ok_or_else(|| {
        format!(
            "体测系统未返回登录参数（停留在 {}），请稍后重试",
            page.url.host_str().unwrap_or("未知页面")
        )
    })?;
    school::sports::check_login(&transport.client, &launch).await?;
    let cookies = transport.snapshot()?;
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新打开".into());
    }
    let mut book = accounts::load(&path)?;
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    account.sites.insert(
        "fitness".into(),
        accounts::SiteSession {
            cookies: Some(cookies),
            token: Some(launch.encode()),
            system_type: None,
        },
    );
    accounts::save(&path, &book)?;
    Ok(())
}

/// The profile system keeps its own OAuth login and JWT, so it cannot reuse the
/// card session. A captcha or phone step reports an error instead of guessing.
#[tauri::command]
pub(crate) async fn refresh_profile_session(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    main_only(&window)?;
    let path = accounts_path(&app)?;
    let (account_id, username, password) = {
        let _guard = state.auth_lock.lock().await;
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("请先登录校园账号")?;
        let saved = record
            .password
            .as_ref()
            .ok_or("此账号未保存密码，无法自动登录学籍档案")?;
        (
            record.id.clone(),
            record.username.clone(),
            zeroize::Zeroizing::new(saved.0.clone()),
        )
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let visitor = device_visitor(&app)?;
    let (cookies, token, system_type) = auth::hello_login(&username, &password, &visitor).await?;
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新打开".into());
    }
    let mut book = accounts::load(&path)?;
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    account.sites.insert(
        "profile".into(),
        accounts::SiteSession {
            cookies: Some(cookies),
            token: Some(token),
            system_type: Some(system_type),
        },
    );
    accounts::save(&path, &book)?;
    Ok(true)
}

/// 思源学堂 keeps a separate CAS cookie session of its own.
#[tauri::command]
pub(crate) async fn refresh_lms_session(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    main_only(&window)?;
    renew_lms_session(&app, &state).await?;
    Ok(true)
}

/// The library seat system is plain HTTP on the campus network; WebVPN reaches
/// it through the port-aware `http-8086` mapping, so both modes can log in.
#[tauri::command]
pub(crate) async fn refresh_library_session(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    main_only(&window)?;
    let path = accounts_path(&app)?;
    let (account_id, username, password, mode, portal) = {
        let _guard = state.auth_lock.lock().await;
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("请先登录校园账号")?;
        let saved = record
            .password
            .as_ref()
            .ok_or("此账号未保存密码，无法自动登录图书馆")?;
        (
            record.id.clone(),
            record.username.clone(),
            zeroize::Zeroizing::new(saved.0.clone()),
            record.mode,
            record.cookies.clone(),
        )
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let visitor = device_visitor(&app)?;
    let seed = (mode == network::AccessMode::Webvpn).then_some(portal.as_str());
    let cookies = auth::service_login(
        school::LIBRARY_LOGIN,
        mode,
        seed,
        &username,
        &password,
        &visitor,
    )
    .await?;
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新打开".into());
    }
    let mut book = accounts::load(&path)?;
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    account.sites.insert(
        "library".into(),
        accounts::SiteSession {
            cookies: Some(cookies),
            token: None,
            system_type: None,
        },
    );
    accounts::save(&path, &book)?;
    Ok(true)
}

/// The venue system is public plain HTTP with its own OAuth login. Off campus
/// it is reached through the WebVPN `http-8080` mapping, so the account's mode
/// decides both the login and the query addressing.
#[tauri::command]
pub(crate) async fn refresh_venue_session(
    app: tauri::AppHandle,
    window: WebviewWindow,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    main_only(&window)?;
    renew_venue_session(&app, &state).await?;
    Ok(true)
}

/// Shared by the command above and the query retry. The venue login is a
/// read-only OAuth flow, so a query that finds the stored session expired can
/// rebuild it instead of making the user reopen the page after a restart.
pub(crate) async fn renew_venue_session(
    app: &tauri::AppHandle,
    state: &tauri::State<'_, AppState>,
) -> Result<(), String> {
    let path = accounts_path(app)?;
    let (account_id, username, password, mode, portal) = {
        let _guard = state.auth_lock.lock().await;
        let book = accounts::load(&path)?;
        let record = book.active().ok_or("请先登录校园账号")?;
        let saved = record
            .password
            .as_ref()
            .ok_or("此账号未保存密码，无法自动登录场馆系统")?;
        (
            record.id.clone(),
            record.username.clone(),
            zeroize::Zeroizing::new(saved.0.clone()),
            record.mode,
            record.cookies.clone(),
        )
    };
    let generation = state.generation.load(Ordering::SeqCst);
    let visitor = device_visitor(app)?;
    let (transport, mut page, refreshed) = service_login_renewing_portal(
        school::VENUE_LOGIN,
        mode,
        &portal,
        &username,
        &password,
        &visitor,
    )
    .await?;
    // The OAuth first pass sometimes stops on the org.xjtu.edu.cn intermediate
    // page before the venue issues its own session. Now that the CAS cookies
    // are in this jar, asking once more usually lands on the venue.
    if page.url.host_str() != Some("202.117.17.144") {
        let service = url::Url::parse(school::VENUE_LOGIN).map_err(|_| "场馆登录地址无效")?;
        transport.page(&service, mode, "undergraduate").await?;
    }
    // Always load the home page: it is where the venue confirms the OAuth
    // result and fills `userno`. A venue cookie can come from the anonymous
    // landing page, so the cookie jar alone cannot tell a login from a bounce.
    let home = url::Url::parse(school::VENUE_HOME).map_err(|_| "场馆首页地址无效")?;
    page = transport.page(&home, mode, "undergraduate").await?;
    if page.url.host_str() != Some("202.117.17.144") || !school::venue_index_logged_in(&page.html) {
        // Never save a jar that looks logged in but cannot query the venue;
        // the next page load would repeat the same silent failure.
        return Err(format!(
            "场馆系统未完成登录（停留在 {}），请稍后重试",
            page.url.host_str().unwrap_or("未知页面")
        ));
    }
    let cookies = transport.snapshot()?;
    let _guard = state.auth_lock.lock().await;
    if generation != state.generation.load(Ordering::SeqCst) {
        return Err("账号已切换，请重新打开".into());
    }
    let mut book = accounts::load(&path)?;
    let account = book
        .accounts
        .iter_mut()
        .find(|a| a.id == account_id)
        .ok_or("校园账号不存在")?;
    if let Some(portal) = refreshed {
        account.cookies = portal;
    }
    account.sites.insert(
        "venue".into(),
        accounts::SiteSession {
            cookies: Some(cookies),
            token: None,
            system_type: None,
        },
    );
    accounts::save(&path, &book)?;
    Ok(())
}
