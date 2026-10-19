//! Campus transport: isolated cookies, bounded trusted redirects, direct/WebVPN routing.
use aes::cipher::{AsyncStreamCipher, KeyIvInit};
use reqwest::{header, Client, Response, StatusCode};
use reqwest_cookie_store::{CookieStore, CookieStoreMutex};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use url::Url;

const VPN_HOST: &str = "webvpn.xjtu.edu.cn";
const VPN_KEY: &[u8; 16] = b"wrdvpnisthebest!";
pub const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) XJTU-Launcher/0.1 Safari/537.36";

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq, Eq, Debug)]
#[serde(rename_all = "camelCase")]
pub enum AccessMode {
    #[default]
    Direct,
    Webvpn,
}

pub fn trusted(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "login.xjtu.edu.cn"
                    | "webvpn.xjtu.edu.cn"
                    | "bk-kq.xjtu.edu.cn"
                    | "yjs-kq.xjtu.edu.cn"
                    | "ywtb.xjtu.edu.cn"
                    | "authx-service.xjtu.edu.cn"
            )
        )
}

pub fn vpn_url(url: &Url) -> Result<Url, String> {
    if !trusted(url) {
        return Err("校园请求地址不受支持".into());
    }
    encode_vpn_url(url)
}

/// Browsing accepts university sites; authentication transport keeps its narrower allowlist.
pub fn campus_web_url(url: &Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && url
            .host_str()
            .is_some_and(|host| host == "xjtu.edu.cn" || host.ends_with(".xjtu.edu.cn"))
}

pub fn browser_address(url: &Url, mode: AccessMode) -> Result<Url, String> {
    if !campus_web_url(url) {
        return Err("校园窗口仅支持学校 HTTPS 网站".into());
    }
    if mode == AccessMode::Webvpn {
        encode_vpn_url(url)
    } else {
        Ok(url.clone())
    }
}

fn encode_vpn_url(url: &Url) -> Result<Url, String> {
    if url.host_str() == Some(VPN_HOST) {
        return Ok(url.clone());
    }
    let mut host = url
        .host_str()
        .ok_or("校园地址缺少主机名")?
        .as_bytes()
        .to_vec();
    cfb_mode::Encryptor::<aes::Aes128>::new(VPN_KEY.into(), VPN_KEY.into()).encrypt(&mut host);
    let mut mapped = Url::parse(&format!(
        "https://{VPN_HOST}/https/{}{}{}",
        hex::encode(VPN_KEY),
        hex::encode(host),
        url.path()
    ))
    .map_err(|_| "无效的 VPN 地址")?;
    mapped.set_query(url.query());
    mapped.set_fragment(url.fragment());
    Ok(mapped)
}

pub fn original_url(url: &Url) -> Result<Url, String> {
    if !trusted(url) {
        return Err("校园跳转地址不受支持".into());
    }
    if url.host_str() != Some(VPN_HOST) || !url.path().starts_with("/https/") {
        return Ok(url.clone());
    }
    let mut parts = url.path().splitn(4, '/');
    let encoded = parts.nth(2).ok_or("VPN 地址格式不受支持")?;
    if !encoded.starts_with(&hex::encode(VPN_KEY)) {
        return Err("VPN 地址格式不受支持".into());
    }
    let mut host = hex::decode(&encoded[32..]).map_err(|_| "VPN 地址格式不受支持")?;
    cfb_mode::Decryptor::<aes::Aes128>::new(VPN_KEY.into(), VPN_KEY.into()).decrypt(&mut host);
    let host = String::from_utf8(host).map_err(|_| "VPN 地址格式不受支持")?;
    let mut ordinary = Url::parse(&format!(
        "https://{host}/{}",
        parts.next().unwrap_or_default()
    ))
    .map_err(|_| "VPN 地址格式不受支持")?;
    ordinary.set_query(url.query());
    ordinary.set_fragment(url.fragment());
    if !trusted(&ordinary) {
        return Err("VPN 目标地址不受支持".into());
    }
    Ok(ordinary)
}

pub struct Transport {
    pub client: Client,
    pub cookies: Arc<CookieStoreMutex>,
}
impl Transport {
    pub fn new(saved: Option<&str>) -> Result<Self, String> {
        let store = match saved {
            Some(json) => cookie_store::serde::json::load(json.as_bytes())
                .map_err(|_| "保存的校园会话损坏，请重新登录")?,
            None => CookieStore::default(),
        };
        let cookies = Arc::new(CookieStoreMutex::new(store));
        let client = Client::builder()
            .cookie_provider(cookies.clone())
            .connect_timeout(Duration::from_secs(8))
            .timeout(Duration::from_secs(25))
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(USER_AGENT)
            .build()
            .map_err(|_| "无法创建校园连接")?;
        Ok(Self { client, cookies })
    }
    pub fn snapshot(&self) -> Result<String, String> {
        let jar = self.cookies.lock().map_err(|_| "会话服务繁忙")?;
        let mut data = Vec::new();
        cookie_store::serde::json::save_incl_expired_and_nonpersistent(&jar, &mut data)
            .map_err(|_| "无法保存校园会话")?;
        String::from_utf8(data).map_err(|_| "无法保存校园会话".into())
    }
    pub fn address(url: &Url, mode: AccessMode) -> Result<Url, String> {
        if !trusted(url) {
            return Err("校园请求地址不受支持".into());
        }
        if mode == AccessMode::Webvpn {
            vpn_url(url)
        } else {
            Ok(url.clone())
        }
    }
    pub async fn get(&self, url: &Url, mode: AccessMode) -> Result<Response, String> {
        self.client
            .get(Self::address(url, mode)?)
            .send()
            .await
            .map_err(|_| "无法连接校园服务，请检查网络后重试".into())
    }
    pub async fn follow(
        &self,
        mut response: Response,
        mode: AccessMode,
        degree: &str,
    ) -> Result<Page, String> {
        for _ in 0..12 {
            let ordinary = original_url(response.url())?;
            if crate::campus::callback(&ordinary, degree).is_some() {
                return Ok(Page {
                    url: ordinary,
                    html: String::new(),
                    status: response.status(),
                });
            }
            if response.status().is_redirection() {
                if matches!(response.status().as_u16(), 307 | 308) {
                    return Err("校园认证要求重放提交，请重新开始登录".into());
                }
                let location = response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .ok_or("校园认证跳转缺少目标地址")?;
                let next = response
                    .url()
                    .join(location)
                    .map_err(|_| "校园跳转地址无效")?;
                let logical = original_url(&next)?;
                if crate::campus::callback(&logical, degree).is_some() {
                    return Ok(Page {
                        url: logical,
                        html: String::new(),
                        status: StatusCode::OK,
                    });
                }
                response = self.get(&logical, mode).await?;
                continue;
            }
            let status = response.status();
            if !status.is_success() && status != StatusCode::UNAUTHORIZED {
                return Err(format!("校园服务暂不可用（HTTP {}）", status.as_u16()));
            }
            let body = limited_bytes(response, 2 * 1024 * 1024).await?;
            let html = String::from_utf8(body).map_err(|_| "校园登录页面编码不受支持")?;
            return Ok(Page {
                url: ordinary,
                html,
                status,
            });
        }
        Err("校园认证跳转次数过多，请重新开始".into())
    }
    pub async fn page(&self, url: &Url, mode: AccessMode, degree: &str) -> Result<Page, String> {
        self.follow(self.get(url, mode).await?, mode, degree).await
    }
}

pub struct Page {
    pub url: Url,
    pub html: String,
    pub status: StatusCode,
}

pub async fn limited_bytes(mut response: Response, limit: usize) -> Result<Vec<u8>, String> {
    if response.content_length().is_some_and(|n| n > limit as u64) {
        return Err("校园响应超过大小限制".into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "校园响应读取失败")? {
        if bytes.len() + chunk.len() > limit {
            return Err("校园响应超过大小限制".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes)
}

pub async fn json(response: Response) -> Result<serde_json::Value, String> {
    if !response.status().is_success() {
        return Err("校园请求失败，请重新登录或重试".into());
    }
    let value: serde_json::Value =
        serde_json::from_slice(&limited_bytes(response, 2 * 1024 * 1024).await?)
            .map_err(|_| "校园系统返回了非预期数据")?;
    if value.get("code").and_then(|v| v.as_i64()) != Some(0) {
        return Err("校园系统未接受请求，请检查输入或重新登录".into());
    }
    Ok(value
        .get("data")
        .cloned()
        .unwrap_or(serde_json::Value::Null))
}

pub async fn probe(degree: &str) -> Result<AccessMode, String> {
    let host = crate::campus::domain(degree)?;
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .connect_timeout(Duration::from_secs(3))
        .redirect(reqwest::redirect::Policy::none())
        .user_agent(USER_AGENT)
        .build()
        .map_err(|_| "无法检测校园网络")?;
    let url = Url::parse(&format!("https://{host}/sa/auth/cas/login/student-pc"))
        .map_err(|_| "校园地址无效")?;
    let reachable = match client.get(url.clone()).send().await {
        Ok(response) => {
            let status = response.status();
            if status.is_redirection() {
                response
                    .headers()
                    .get(header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .and_then(|s| url.join(s).ok())
                    .is_some_and(|target| {
                        trusted(&target)
                            && (target.host_str() == Some("login.xjtu.edu.cn")
                                && target.path().starts_with("/cas/")
                                || target.host_str() == Some(host))
                    })
            } else if status.is_success() {
                limited_bytes(response, 512 * 1024)
                    .await
                    .ok()
                    .is_some_and(|bytes| {
                        let body = String::from_utf8_lossy(&bytes);
                        body.contains("西安交通大学")
                            && (body.contains("统一身份认证") || body.contains("考勤"))
                    })
            } else {
                false
            }
        }
        Err(_) => false,
    };
    Ok(if reachable {
        AccessMode::Direct
    } else {
        AccessMode::Webvpn
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vpn_roundtrip_preserves_query_and_rejects_foreign_hosts() {
        let url = Url::parse("https://bk-kq.xjtu.edu.cn/sa/auth/cas/login/student-pc?service=https%3A%2F%2Fexample.test%2F&x=1#p").unwrap();
        let mapped = vpn_url(&url).unwrap();
        assert_eq!(mapped.host_str(), Some(VPN_HOST));
        assert_eq!(original_url(&mapped).unwrap(), url);
        assert_eq!(vpn_url(&mapped).unwrap(), mapped);
        for bad in [
            "http://login.xjtu.edu.cn/cas/login",
            "https://login.xjtu.edu.cn.evil.test/",
            "https://u:p@login.xjtu.edu.cn/",
            "https://login.xjtu.edu.cn:444/",
        ] {
            assert!(vpn_url(&Url::parse(bad).unwrap()).is_err());
        }
    }
    #[test]
    fn cookies_are_isolated_and_session_cookies_survive_encrypted_snapshot() {
        let a = Transport::new(None).unwrap();
        let b = Transport::new(None).unwrap();
        a.cookies
            .lock()
            .unwrap()
            .parse(
                "test=session-secret; Secure; Path=/",
                &Url::parse("https://login.xjtu.edu.cn/").unwrap(),
            )
            .unwrap();
        assert_eq!(b.cookies.lock().unwrap().iter_unexpired().count(), 0);
        let restored = Transport::new(Some(&a.snapshot().unwrap())).unwrap();
        assert_eq!(restored.cookies.lock().unwrap().iter_unexpired().count(), 1);
    }
}
