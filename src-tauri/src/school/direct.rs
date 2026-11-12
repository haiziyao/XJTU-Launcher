//! Shared HTTP setup for services that only accept a direct CAS session.
//!
//! Several campus systems refuse WebVPN cookies. Their adapters build the same
//! client: saved cookies, a browser user agent (CAS otherwise answers with its
//! login page) and redirects that never leave `xjtu.edu.cn`.
use crate::network::{Transport, USER_AGENT};
use reqwest::{redirect::Policy, Client as Http};
use std::time::Duration;
use url::Url;

fn campus_host(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host == "xjtu.edu.cn" || host.ends_with(".xjtu.edu.cn"))
}

pub(crate) fn client(cookies: &str) -> Result<Http, String> {
    let transport = Transport::new(Some(cookies))?;
    Http::builder()
        .user_agent(USER_AGENT)
        .cookie_provider(transport.cookies.clone())
        .redirect(Policy::custom(|attempt| {
            if campus_host(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "无法创建校园服务连接".into())
}

/// The account's direct session, falling back to the main CAS cookies when a
/// saved direct session does not exist yet.
pub(crate) fn account(account: &crate::accounts::Account) -> Result<Http, String> {
    let saved = account
        .direct_cookies
        .as_deref()
        .unwrap_or(&account.cookies);
    client(saved)
}

/// A per-service session stored under `sites.<key>`.
#[allow(dead_code)] // per-service sessions are stored one adapter at a time
pub(crate) fn site(
    account: &crate::accounts::Account,
    key: &str,
    label: &str,
) -> Result<Http, String> {
    let site = account
        .sites
        .get(key)
        .ok_or_else(|| format!("请先完成{label}登录"))?;
    let cookies = site
        .cookies
        .as_deref()
        .filter(|cookies| !cookies.is_empty())
        .ok_or_else(|| format!("请先完成{label}登录"))?;
    client(cookies)
}

#[cfg(test)]
mod tests {
    use super::campus_host;

    fn url(value: &str) -> url::Url {
        url::Url::parse(value).unwrap()
    }

    #[test]
    fn only_https_campus_hosts_are_followed() {
        assert!(campus_host(&url("https://tyxylp.xjtu.edu.cn/login")));
        assert!(campus_host(&url("https://xjtu.edu.cn/")));
        assert!(!campus_host(&url("http://tyxylp.xjtu.edu.cn/login")));
        assert!(!campus_host(&url("https://example.com/")));
        assert!(!campus_host(&url("https://xjtu.edu.cn.example.com/")));
    }
}
