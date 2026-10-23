//! Native CAS driver. Password retention is explicit and stored with the local account book.
use crate::{
    accounts::{Account, SavedPassword},
    campus,
    network::{self, AccessMode, Page, Transport},
};
use base64::{
    engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD},
    Engine,
};
use reqwest::StatusCode;
use rsa::{pkcs1::DecodeRsaPublicKey, pkcs8::DecodePublicKey, Pkcs1v15Encrypt, RsaPublicKey};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
use url::Url;
use uuid::Uuid;
use zeroize::Zeroizing;

const CAS: &str = "https://login.xjtu.edu.cn";
const VPN_LOGIN: &str = "https://webvpn.xjtu.edu.cn/login?cas_login=true";
const YWTB: &str = "https://login.xjtu.edu.cn/cas/login?service=https%3A%2F%2Fywtb.xjtu.edu.cn%2F%3Fpath%3Dhttps%253A%252F%252Fywtb.xjtu.edu.cn%252Fmain.html%2523%252FIndex";
const TTL: Duration = Duration::from_secs(15 * 60);

#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    Credentials,
    Captcha,
    Mfa,
    Identity,
    Success,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Step {
    Webvpn,
    Campus,
}
#[derive(Clone, Serialize)]
pub struct Choice {
    pub label: String,
    pub name: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub flow_id: String,
    pub access_mode: AccessMode,
    pub step: Step,
    pub phase: Phase,
    pub phone: Option<String>,
    pub choices: Vec<Choice>,
    pub message: Option<String>,
    pub can_continue: bool,
    pub retry_after_seconds: u64,
    pub save_pending: bool,
}
#[derive(Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Action {
    Credentials {
        username: String,
        password: String,
        #[serde(default)]
        remember_password: bool,
        #[serde(default)]
        captcha: String,
    },
    SavedCredentials {
        account_id: String,
        #[serde(default)]
        captcha: String,
        remember_password: bool,
    },
    Continue {
        #[serde(default)]
        captcha: String,
    },
    SendMfa,
    VerifyMfa {
        code: String,
    },
    ChooseIdentity {
        label: String,
    },
}
struct Mfa {
    state: String,
    flow: &'static str,
    gid: Option<String>,
    phone: Option<String>,
    verified: bool,
    sent: Option<Instant>,
}
struct Form {
    execution: String,
    sec_state: Option<String>,
    event: String,
    submit: String,
    choices: Vec<Choice>,
    mfa_enabled: bool,
}
fn input(html: &Html, name: &str) -> Option<String> {
    html.select(&Selector::parse("input").unwrap())
        .find(|e| e.value().attr("name") == Some(name))
        .and_then(|e| e.value().attr("value"))
        .map(str::to_string)
}
fn parse_form(text: &str) -> Option<Form> {
    let html = Html::parse_document(text);
    let execution = input(&html, "execution").filter(|s| !s.is_empty())?;
    let mut choices = Vec::new();
    for wrap in html.select(&Selector::parse(".account-wrap").unwrap()) {
        let name = wrap
            .select(&Selector::parse(".name").unwrap())
            .next()
            .map(|e| e.text().collect::<String>().trim().to_string());
        let label = wrap
            .select(&Selector::parse("el-radio[label]").unwrap())
            .next()
            .and_then(|e| e.value().attr("label"));
        if let (Some(name), Some(label)) = (name, label) {
            if !label.is_empty() {
                choices.push(Choice {
                    name,
                    label: label.into(),
                });
            }
        }
    }
    let mfa_enabled = regex::Regex::new(
        r#"globalConfig\s*=\s*eval\('\('\s*\+\s*("(?:\\.|[^"\\])*")\s*\+\s*'\)'\s*\)"#,
    )
    .unwrap()
    .captures(text)
    .and_then(|c| serde_json::from_str::<String>(&c[1]).ok())
    .and_then(|v| serde_json::from_str::<Value>(&v).ok())
    .and_then(|v| v.get("mfaEnabled").cloned())
    .is_none_or(|v| v != Value::Bool(false) && v != "false");
    Some(Form {
        execution,
        sec_state: input(&html, "secState"),
        event: input(&html, "_eventId").unwrap_or("submit".into()),
        submit: input(&html, "submit").unwrap_or("Login1".into()),
        choices,
        mfa_enabled,
    })
}
fn has_error(text: &str) -> bool {
    Html::parse_document(text)
        .select(&Selector::parse("el-alert").unwrap())
        .any(|e| {
            e.value().attr("type") == Some("error")
                || e.value().attr("title").is_some_and(|s| !s.is_empty())
        })
}
fn parse_public_key(pem: &str) -> Result<RsaPublicKey, String> {
    // The campus endpoint returns an unwrapped Base64 line. Decode the supported
    // PEM envelopes explicitly; keep RSA DER validation strict.
    let invalid = || "无法解析校园登录公钥，请重新开始后重试".to_string();
    if pem.len() > 32 * 1024 {
        return Err(invalid());
    }
    let pem = pem.trim();
    for label in ["PUBLIC KEY", "RSA PUBLIC KEY"] {
        let begin = format!("-----BEGIN {label}-----");
        let end = format!("-----END {label}-----");
        if let Some(body) = pem.strip_prefix(&begin).and_then(|s| s.strip_suffix(&end)) {
            let base64: String = body.chars().filter(|c| !c.is_ascii_whitespace()).collect();
            let der = STANDARD.decode(base64).map_err(|_| invalid())?;
            return if label == "PUBLIC KEY" {
                RsaPublicKey::from_public_key_der(&der).map_err(|_| invalid())
            } else {
                RsaPublicKey::from_pkcs1_der(&der).map_err(|_| invalid())
            };
        }
    }
    Err(invalid())
}
fn encrypt_password(password: &str, pem: &str) -> Result<String, String> {
    let public = parse_public_key(pem)?;
    let bytes = public
        .encrypt(&mut rand::rngs::OsRng, Pkcs1v15Encrypt, password.as_bytes())
        .map_err(|_| "无法加密校园登录密码")?;
    Ok(format!("__RSA__{}", STANDARD.encode(bytes)))
}

/// An unattended password login that starts at a service, so services which
/// reject WebVPN get a real CAS session of their own. The returned cookie jar
/// holds that CAS session plus whatever the service sets on the way back.
/// Captcha and two-step verification are reported instead of guessed.
pub async fn direct_login(service: &str, username: &str, password: &str) -> Result<String, String> {
    let (transport, _) = direct_login_page(service, username, password).await?;
    transport.snapshot()
}

/// The profile system starts at an OAuth landing page rather than a cookie
/// service, so the caller needs the final page as well as the cookie jar.
async fn direct_login_page(
    service: &str,
    username: &str,
    password: &str,
) -> Result<(Transport, Page), String> {
    if username.trim().is_empty() || password.is_empty() || password.len() > 512 {
        return Err("保存的校园密码无效，请重新登录".into());
    }
    let service = Url::parse(service).map_err(|_| "直连服务地址无效")?;
    if !network::trusted(&service) && !network::service_http(&service) {
        return Err("直连服务地址不受支持".into());
    }
    let degree = "undergraduate";
    let transport = Transport::new(None)?;
    let mut page = transport.page(&service, AccessMode::Direct, degree).await?;
    if page.url.host_str() != Some("login.xjtu.edu.cn") {
        // Without the CAS page there is nothing to submit; do not keep a
        // service session that cannot authenticate the user.
        return Err("直连登录未能打开校园认证页面，请稍后重试".into());
    }
    let first = parse_form(&page.html).ok_or("校园认证页面格式发生变化，请手动登录")?;
    let response = transport
        .get(
            &Url::parse(&format!("{CAS}/cas/jwt/publicKey")).unwrap(),
            AccessMode::Direct,
        )
        .await?;
    if !response.status().is_success() {
        return Err("校园登录公钥获取失败".into());
    }
    let pem = String::from_utf8(network::limited_bytes(response, 32 * 1024).await?)
        .map_err(|_| "校园登录公钥格式不受支持")?;
    let encrypted = Zeroizing::new(encrypt_password(password, &pem)?);
    let visitor = Uuid::new_v4().simple().to_string();
    // The detection call also opens the server-side verification state that the
    // password submission must echo back, even when no phone step is needed.
    let mut mfa_state = String::new();
    if first.mfa_enabled {
        let url = Url::parse(&format!("{CAS}/cas/mfa/detect")).unwrap();
        let response = transport
            .client
            .post(Transport::address(&url, AccessMode::Direct)?)
            .header("Referer", page.url.as_str())
            .form(&[
                ("username", username),
                ("password", encrypted.as_str()),
                ("fpVisitorId", visitor.as_str()),
                ("loginType", "passwordLogin"),
            ])
            .send()
            .await
            .map_err(|_| "无法检测校园两步验证")?;
        let data = network::json(response).await?;
        mfa_state = data
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let need = data.get("need").and_then(Value::as_bool).unwrap_or(true);
        if need {
            return Err("该账号需要手机验证，无法自动直连".into());
        }
    }
    // The campus CAS may show one confirmation step before the service callback.
    for attempt in 0..2 {
        let form = parse_form(&page.html).ok_or("校园认证页面格式发生变化，请手动登录")?;
        let response = transport
            .client
            .post(Transport::address(&page.url, AccessMode::Direct)?)
            .form(&[
                ("username", username.to_string()),
                ("password", encrypted.to_string()),
                ("execution", form.execution),
                ("_eventId", "submit".into()),
                ("submit1", "Login1".into()),
                ("fpVisitorId", visitor.clone()),
                ("captcha", String::new()),
                ("currentMenu", "1".into()),
                ("failN", attempt.to_string()),
                ("mfaState", mfa_state.clone()),
                ("geolocation", String::new()),
                ("trustAgent", "false".into()),
            ])
            .send()
            .await
            .map_err(|_| "校园登录提交失败，请检查网络")?;
        page = transport
            .follow(response, AccessMode::Direct, degree)
            .await?;
        if page.url.host_str() != Some("login.xjtu.edu.cn") {
            return Ok((transport, page));
        }
        if has_error(&page.html) {
            return Err("校园账号或密码不正确，请重新登录校园账号".into());
        }
    }
    Err("直连登录需要验证码，请稍后重试或手动登录".into())
}

const HELLO_LOGIN: &str = "https://org.xjtu.edu.cn/openplatform/oauth/authorize?appId=966&redirectUri=http%3A%2F%2Fhello.xjtu.edu.cn%2Fyingxin%2Flogin%2Fxjtu%2Foauth%2Fpc&responseType=code&scope=user_info&state=pc";

/// Signs in to the profile (hello) system and returns its cookie jar, bearer
/// token and the JWT-provided system type. The token is not logged.
pub async fn hello_login(
    username: &str,
    password: &str,
) -> Result<(String, String, String), String> {
    let (transport, mut page) = direct_login_page(HELLO_LOGIN, username, password).await?;
    if page.url.host_str() != Some("hello.xjtu.edu.cn") {
        // Some CAS responses land on the OAuth consent page first; ask once more.
        page = transport
            .page(
                &Url::parse(HELLO_LOGIN).map_err(|_| "个人信息登录地址无效")?,
                AccessMode::Direct,
                "undergraduate",
            )
            .await?;
    }
    let token = network::hello_token(&page.url).ok_or("个人信息系统未返回访问令牌")?;
    let system_type = jwt_system_type(&token);
    Ok((transport.snapshot()?, token, system_type))
}

/// The profile JWT carries the target app in `systemType`; fall back to the
/// student app when the payload is absent or malformed.
fn jwt_system_type(token: &str) -> String {
    let Some(payload) = token.split('.').nth(1) else {
        return "yingxin_student_pc".into();
    };
    let Ok(bytes) = URL_SAFE_NO_PAD.decode(payload.trim_end_matches('=')) else {
        return "yingxin_student_pc".into();
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return "yingxin_student_pc".into();
    };
    value
        .get("systemType")
        .and_then(Value::as_str)
        .filter(|kind| !kind.is_empty() && kind.len() <= 64)
        .unwrap_or("yingxin_student_pc")
        .to_string()
}

pub struct Login {
    id: String,
    pub created: Instant,
    transport: Transport,
    degree: String,
    mode: AccessMode,
    step: Step,
    phase: Phase,
    page: Option<Page>,
    username: String,
    encrypted: Zeroizing<String>,
    retained_password: Option<SavedPassword>,
    pending_captcha: String,
    visitor: String,
    mfa: Option<Mfa>,
    choices: Vec<Choice>,
    chosen: Option<String>,
    failures: u8,
    message: Option<String>,
    pub completed: Option<Account>,
}
impl Login {
    pub async fn begin(degree: String) -> Result<Self, String> {
        let mode = network::probe(&degree).await?;
        let transport = Transport::new(None)?;
        let step = if mode == AccessMode::Webvpn {
            Step::Webvpn
        } else {
            Step::Campus
        };
        let target = if step == Step::Webvpn {
            VPN_LOGIN.to_string()
        } else {
            format!(
                "https://{}/sa/auth/cas/login/student-pc",
                campus::domain(&degree)?
            )
        };
        let page = transport
            .page(
                &Url::parse(&target).map_err(|_| "校园入口无效")?,
                AccessMode::Direct,
                &degree,
            )
            .await?;
        let mut flow = Self {
            id: Uuid::new_v4().to_string(),
            created: Instant::now(),
            transport,
            degree,
            mode,
            step,
            phase: Phase::Credentials,
            page: None,
            username: String::new(),
            encrypted: Zeroizing::new(String::new()),
            retained_password: None,
            pending_captcha: String::new(),
            visitor: Uuid::new_v4().simple().to_string(),
            mfa: None,
            choices: vec![],
            chosen: None,
            failures: 0,
            message: None,
            completed: None,
        };
        flow.accept(page).await?;
        Ok(flow)
    }
    fn request_mode(&self) -> AccessMode {
        if self.step == Step::Webvpn {
            AccessMode::Direct
        } else {
            self.mode
        }
    }
    pub fn view(&self) -> View {
        View {
            flow_id: self.id.clone(),
            access_mode: self.mode,
            step: self.step,
            phase: self.phase,
            phone: self.mfa.as_ref().and_then(|m| m.phone.clone()),
            choices: self.choices.clone(),
            message: self.message.clone(),
            can_continue: !self.encrypted.is_empty()
                && matches!(self.phase, Phase::Credentials | Phase::Captcha),
            retry_after_seconds: self
                .mfa
                .as_ref()
                .and_then(|m| m.sent)
                .map(|sent| 60u64.saturating_sub(sent.elapsed().as_secs()))
                .unwrap_or(0),
            save_pending: self.completed.is_some(),
        }
    }
    pub fn degree(&self) -> &str {
        &self.degree
    }
    fn ensure_active(&self) -> Result<(), String> {
        if self.created.elapsed() > TTL || self.phase == Phase::Success {
            Err("登录流程已结束或过期，请重新开始".into())
        } else {
            Ok(())
        }
    }
    async fn post_form(&self, url: &Url, data: &[(&str, String)]) -> Result<Page, String> {
        if url.host_str() != Some("login.xjtu.edu.cn") || !url.path().starts_with("/cas/") {
            return Err("校园登录提交地址不受支持".into());
        }
        let response = self
            .transport
            .client
            .post(Transport::address(url, self.request_mode())?)
            .form(data)
            .send()
            .await
            .map_err(|_| "校园登录提交失败，请检查网络")?;
        self.transport
            .follow(response, self.request_mode(), &self.degree)
            .await
    }
    async fn api(&self, path: &str, data: &Value) -> Result<Value, String> {
        let url = Url::parse(&format!("{CAS}{path}")).map_err(|_| "校园认证接口地址无效")?;
        let response = self
            .transport
            .client
            .post(Transport::address(&url, self.request_mode())?)
            .json(data)
            .send()
            .await
            .map_err(|_| "无法连接校园验证服务")?;
        network::json(response).await
    }
    async fn init_mfa(&mut self) -> Result<(), String> {
        let mfa = self.mfa.as_ref().ok_or("当前无需两步验证")?;
        let mut url = Url::parse(&format!("{CAS}/cas/{}/initByType/securephone", mfa.flow))
            .map_err(|_| "验证地址无效")?;
        url.query_pairs_mut().append_pair("state", &mfa.state);
        let data = network::json(self.transport.get(&url, self.request_mode()).await?).await?;
        let gid = data
            .get("gid")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .ok_or("校园验证缺少请求标识")?
            .to_string();
        let phone = data
            .get("securePhone")
            .and_then(Value::as_str)
            .map(str::to_string);
        let mfa = self.mfa.as_mut().unwrap();
        mfa.gid = Some(gid);
        mfa.phone = phone;
        Ok(())
    }
    async fn password_page(&mut self, captcha: String) -> Result<Option<Page>, String> {
        if !captcha.is_empty() {
            self.pending_captcha = captcha;
        }
        let captcha = self.pending_captcha.clone();
        let page = self.page.as_ref().ok_or("缺少校园登录页面")?;
        let form = parse_form(&page.html).ok_or("校园登录页面格式发生变化，请重新开始")?;
        if self.failures >= 3 && captcha.is_empty() {
            self.phase = Phase::Captcha;
            return Ok(None);
        }
        if form.mfa_enabled && self.mfa.is_none() {
            let url = Url::parse(&format!("{CAS}/cas/mfa/detect")).unwrap();
            let response = self
                .transport
                .client
                .post(Transport::address(&url, self.request_mode())?)
                .header(
                    "Referer",
                    Transport::address(&page.url, self.request_mode())?.as_str(),
                )
                .form(&[
                    ("username", self.username.as_str()),
                    ("password", self.encrypted.as_str()),
                    ("fpVisitorId", self.visitor.as_str()),
                    ("loginType", "passwordLogin"),
                ])
                .send()
                .await
                .map_err(|_| "无法检测校园两步验证")?;
            let data = network::json(response).await?;
            let state = data
                .get("state")
                .and_then(Value::as_str)
                .ok_or("校园验证响应缺少 state")?
                .to_string();
            let need = data
                .get("need")
                .and_then(Value::as_bool)
                .ok_or("校园验证响应缺少 need")?;
            self.mfa = Some(Mfa {
                state,
                flow: "mfa",
                gid: None,
                phone: None,
                verified: !need,
                sent: None,
            });
            if need {
                self.phase = Phase::Mfa;
                self.init_mfa().await?;
                return Ok(None);
            }
        }
        if self.mfa.as_ref().is_some_and(|m| !m.verified) {
            self.phase = Phase::Mfa;
            return Ok(None);
        }
        let page = self.page.as_ref().ok_or("缺少校园登录页面")?;
        let mfa = self
            .mfa
            .as_ref()
            .map(|m| m.state.clone())
            .unwrap_or_default();
        self.post_form(
            &page.url,
            &[
                ("username", self.username.clone()),
                ("password", self.encrypted.to_string()),
                ("execution", form.execution),
                ("_eventId", "submit".into()),
                ("submit1", "Login1".into()),
                ("fpVisitorId", self.visitor.clone()),
                ("captcha", captcha),
                ("currentMenu", "1".into()),
                ("failN", self.failures.to_string()),
                ("mfaState", mfa),
                ("geolocation", String::new()),
                ("trustAgent", "false".into()),
            ],
        )
        .await
        .map(Some)
    }
    async fn accept(&mut self, mut page: Page) -> Result<(), String> {
        loop {
            // A successful service callback is mandatory. An arbitrary page without an error is not login success.
            if self.step == Step::Campus {
                if let Some((request, ticket)) = campus::callback(&page.url, &self.degree) {
                    let url = Url::parse(&format!(
                        "https://{}/sa/auth/cas/exchange",
                        campus::domain(&self.degree)?
                    ))
                    .unwrap();
                    let response = self
                        .transport
                        .client
                        .post(Transport::address(&url, self.mode)?)
                        .json(&json!({"loginRequestId":request,"ticket":ticket}))
                        .send()
                        .await
                        .map_err(|_| "校园业务换票失败")?;
                    let data = network::json(response).await?;
                    let token = data
                        .get("tokenValue")
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        .ok_or("校园业务未返回会话")?
                        .to_string();
                    let (name, student_id) = self.profile().await.unwrap_or((None, None));
                    self.completed = Some(Account {
                        id: Uuid::new_v4().to_string(),
                        username: self.username.clone(),
                        degree: self.degree.clone(),
                        name,
                        student_id: student_id.or(self.chosen.clone()),
                        avatar: None,
                        password: self.retained_password.take(),
                        mode: self.mode,
                        token,
                        cookies: self.transport.snapshot()?,
                        direct_cookies: None,
                        sites: Default::default(),
                    });
                    self.phase = Phase::Success;
                    self.page = None;
                    self.encrypted.clear();
                    self.pending_captcha.clear();
                    self.mfa = None;
                    self.choices.clear();
                    return Ok(());
                }
            } else if page.url.host_str() == Some("webvpn.xjtu.edu.cn")
                && page.status.is_success()
                && page.html.contains("西安交通大学WebVPN - 资源站点")
            {
                self.step = Step::Campus;
                self.mfa = None;
                self.failures = 0;
                self.pending_captcha.clear();
                let target = Url::parse(&format!(
                    "https://{}/sa/auth/cas/login/student-pc",
                    campus::domain(&self.degree)?
                ))
                .unwrap();
                page = self
                    .transport
                    .page(&target, self.mode, &self.degree)
                    .await?;
                continue;
            }
            if page.url.host_str() != Some("login.xjtu.edu.cn")
                || !page.url.path().starts_with("/cas/")
            {
                return Err("未收到预期的校园认证页面，请重新开始".into());
            }
            let form = parse_form(&page.html).ok_or("校园认证页面格式发生变化，请重新开始")?;
            let failed = page.status == StatusCode::UNAUTHORIZED || has_error(&page.html);
            self.page = Some(page);
            if failed {
                self.failures = self.failures.saturating_add(1);
                self.mfa = None;
                self.encrypted.clear();
                self.retained_password = None;
                self.pending_captcha.clear();
                self.phase = if self.failures >= 3 {
                    Phase::Captcha
                } else {
                    Phase::Credentials
                };
                self.message = Some("登录未通过，请检查账号、密码或验证码后重试".into());
                return Ok(());
            }
            if let Some(state) = form.sec_state {
                self.mfa = Some(Mfa {
                    state,
                    flow: "sec",
                    gid: None,
                    phone: None,
                    verified: false,
                    sent: None,
                });
                self.phase = Phase::Mfa;
                self.init_mfa().await?;
            } else if !form.choices.is_empty() {
                self.choices = form.choices;
                self.phase = Phase::Identity;
            } else {
                self.phase = Phase::Credentials;
            }
            return Ok(());
        }
    }
    async fn profile(&self) -> Result<(Option<String>, Option<String>), String> {
        // YWTB supplies verified name / student number. Do not invent a name from the login identifier.
        let page = self
            .transport
            .page(&Url::parse(YWTB).unwrap(), self.mode, &self.degree)
            .await?;
        if page.url.host_str() != Some("ywtb.xjtu.edu.cn") {
            return Err("基础信息认证尚未完成".into());
        }
        let ticket = page
            .url
            .query_pairs()
            .find(|(k, _)| k == "ticket")
            .map(|(_, v)| v.into_owned())
            .ok_or("基础信息回调缺少 ticket")?;
        let payload = ticket.split('.').nth(1).ok_or("基础信息凭据格式不受支持")?;
        let json: Value = serde_json::from_slice(
            &URL_SAFE_NO_PAD
                .decode(payload.trim_end_matches('='))
                .map_err(|_| "基础信息凭据格式不受支持")?,
        )
        .map_err(|_| "基础信息凭据格式不受支持")?;
        // The token is received over the authenticated TLS flow and used for the service, not trusted as authorization locally.
        let token = json
            .get("idToken")
            .and_then(Value::as_str)
            .ok_or("基础信息缺少服务凭据")?;
        let url = Url::parse("https://authx-service.xjtu.edu.cn/personal/api/v1/personal/me/user")
            .unwrap();
        let response = self
            .transport
            .client
            .get(Transport::address(&url, self.mode)?)
            .header("x-device-info", "PC")
            .header("x-terminal-info", "PC")
            .header("x-id-token", token)
            .send()
            .await
            .map_err(|_| "基础信息读取失败")?;
        let data = network::json(response).await?;
        let attributes = data.get("attributes").ok_or("基础信息字段不受支持")?;
        let clean = |v: Option<&Value>| {
            v.and_then(Value::as_str)
                .filter(|s| !s.is_empty() && s.len() <= 256)
                .map(str::to_string)
        };
        Ok((
            clean(attributes.get("userName")).or_else(|| clean(data.get("username"))),
            clean(attributes.get("userUid")),
        ))
    }
    pub async fn captcha(&self) -> Result<String, String> {
        self.ensure_active()?;
        if !matches!(self.phase, Phase::Credentials | Phase::Captcha) {
            return Err("当前登录阶段无需图片验证码".into());
        }
        let response = self
            .transport
            .get(
                &Url::parse(&format!("{CAS}/cas/captcha.jpg")).unwrap(),
                self.request_mode(),
            )
            .await?;
        if !response.status().is_success() {
            return Err("图片验证码获取失败".into());
        }
        let bytes = network::limited_bytes(response, 256 * 1024).await?;
        let kind = match image::guess_format(&bytes).map_err(|_| "验证码未返回图片")? {
            image::ImageFormat::Png => "png",
            image::ImageFormat::Jpeg => "jpeg",
            _ => return Err("验证码图片格式不受支持".into()),
        };
        Ok(format!(
            "data:image/{kind};base64,{}",
            STANDARD.encode(bytes)
        ))
    }
    pub async fn act(&mut self, action: Action) -> Result<(), String> {
        self.ensure_active()?;
        self.message = None;
        let page = match action {
            Action::Credentials {
                username,
                password,
                remember_password,
                captcha,
            } => {
                let password = Zeroizing::new(password);
                if !matches!(self.phase, Phase::Credentials | Phase::Captcha) {
                    return Err("当前阶段不接受账号密码".into());
                }
                if username.trim().is_empty()
                    || username.len() > 256
                    || password.is_empty()
                    || password.len() > 512
                    || captcha.len() > 32
                {
                    return Err("请输入有效的校园账号、密码和验证码".into());
                }
                let response = self
                    .transport
                    .get(
                        &Url::parse(&format!("{CAS}/cas/jwt/publicKey")).unwrap(),
                        self.request_mode(),
                    )
                    .await?;
                if !response.status().is_success() {
                    return Err("校园登录公钥获取失败".into());
                }
                let pem = String::from_utf8(network::limited_bytes(response, 32 * 1024).await?)
                    .map_err(|_| "校园登录公钥格式不受支持")?;
                self.username = username.trim().to_string();
                self.encrypted = Zeroizing::new(encrypt_password(&password, &pem)?);
                self.retained_password =
                    remember_password.then(|| SavedPassword(password.to_string()));
                self.pending_captcha = captcha.clone();
                self.mfa = None;
                self.password_page(captcha).await?
            }
            Action::SavedCredentials { .. } => {
                return Err("保存的凭据必须由本机账号服务读取".into())
            }
            Action::Continue { captcha } => {
                if !matches!(self.phase, Phase::Credentials | Phase::Captcha)
                    || self.encrypted.is_empty()
                    || captcha.len() > 32
                {
                    return Err("请重新提交校园登录信息".into());
                }
                self.password_page(captcha).await?
            }
            Action::SendMfa => {
                if self.phase != Phase::Mfa {
                    return Err("当前无需两步验证".into());
                }
                if self.mfa.as_ref().is_some_and(|m| {
                    m.sent
                        .is_some_and(|s| s.elapsed() < Duration::from_secs(60))
                }) {
                    return Err("验证码已发送，请稍后重试".into());
                }
                if self.mfa.as_ref().is_some_and(|m| m.gid.is_none()) {
                    self.init_mfa().await?;
                }
                let mfa = self.mfa.as_ref().ok_or("两步验证上下文已过期")?;
                self.api(
                    "/attest/api/guard/securephone/send",
                    &json!({"gid":mfa.gid}),
                )
                .await?;
                self.mfa.as_mut().unwrap().sent = Some(Instant::now());
                self.message = Some("验证码已发送".into());
                None
            }
            Action::VerifyMfa { code } => {
                if self.phase != Phase::Mfa
                    || code.is_empty()
                    || code.len() > 16
                    || !code.bytes().all(|b| b.is_ascii_digit())
                {
                    return Err("请输入有效的手机验证码".into());
                }
                let mfa = self.mfa.as_ref().ok_or("两步验证上下文已过期")?;
                if mfa.sent.is_none() {
                    return Err("请先发送手机验证码".into());
                }
                let data = self
                    .api(
                        "/attest/api/guard/securephone/valid",
                        &json!({"gid":mfa.gid,"code":code}),
                    )
                    .await?;
                if !matches!(data.get("status"),Some(v) if v==2 || v=="2") {
                    return Err("手机验证码验证未通过".into());
                }
                self.mfa.as_mut().unwrap().verified = true;
                if self.mfa.as_ref().unwrap().flow == "sec" {
                    let page = self.page.as_ref().ok_or("缺少安全验证页面")?;
                    let form = parse_form(&page.html).ok_or("安全验证表单缺失")?;
                    Some(
                        self.post_form(
                            &page.url,
                            &[
                                ("secState", form.sec_state.ok_or("安全验证状态缺失")?),
                                ("execution", form.execution),
                                ("_eventId", form.event),
                                ("submit", form.submit),
                                ("geolocation", String::new()),
                                ("fpVisitorId", self.visitor.clone()),
                            ],
                        )
                        .await?,
                    )
                } else {
                    self.password_page(String::new()).await?
                }
            }
            Action::ChooseIdentity { label } => {
                if self.phase != Phase::Identity || !self.choices.iter().any(|c| c.label == label) {
                    return Err("请选择当前登录提供的校园身份".into());
                }
                let page = self.page.as_ref().ok_or("缺少身份选择页面")?;
                let form = parse_form(&page.html).ok_or("身份选择表单缺失")?;
                self.chosen = Some(label.clone());
                Some(
                    self.post_form(
                        &page.url,
                        &[
                            ("execution", form.execution),
                            ("_eventId", "submit".into()),
                            ("geolocation", String::new()),
                            ("fpVisitorId", self.visitor.clone()),
                            ("trustAgent", "false".into()),
                            ("username", label),
                            ("useDefault", "false".into()),
                        ],
                    )
                    .await?,
                )
            }
        };
        if let Some(page) = page {
            self.accept(page).await?;
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct Handle {
    pub login: Arc<tokio::sync::Mutex<Login>>,
    pub cancelled: Arc<AtomicBool>,
}
struct Slot {
    created: Instant,
    cancelled: Arc<AtomicBool>,
    handle: Option<Handle>,
}
#[derive(Default)]
pub struct Manager {
    flows: Mutex<HashMap<String, Slot>>,
}
impl Manager {
    pub fn reserve(&self, id: &str) -> Result<Arc<AtomicBool>, String> {
        if Uuid::parse_str(id).is_err() {
            return Err("登录请求标识无效".into());
        }
        let mut flows = self.flows.lock().map_err(|_| "认证服务繁忙")?;
        flows.retain(|_, slot| {
            let live = slot.created.elapsed() < TTL;
            if !live {
                slot.cancelled.store(true, Ordering::SeqCst);
            }
            live
        });
        if flows.contains_key(id) || flows.len() >= 4 {
            return Err("待完成的登录过多或请求重复，请先取消现有登录".into());
        }
        let cancelled = Arc::new(AtomicBool::new(false));
        flows.insert(
            id.into(),
            Slot {
                created: Instant::now(),
                cancelled: cancelled.clone(),
                handle: None,
            },
        );
        Ok(cancelled)
    }
    pub fn complete(
        &self,
        id: &str,
        token: &Arc<AtomicBool>,
        result: Result<Login, String>,
    ) -> Result<View, String> {
        let mut flows = self.flows.lock().map_err(|_| "认证服务繁忙")?;
        let slot = flows.get_mut(id).ok_or("登录已取消")?;
        if !Arc::ptr_eq(token, &slot.cancelled) || token.load(Ordering::SeqCst) {
            return Err("登录已取消".into());
        }
        match result {
            Ok(mut login) => {
                login.id = id.into();
                let view = login.view();
                slot.handle = Some(Handle {
                    cancelled: token.clone(),
                    login: Arc::new(tokio::sync::Mutex::new(login)),
                });
                Ok(view)
            }
            Err(e) => {
                flows.remove(id);
                Err(e)
            }
        }
    }
    pub fn get(&self, id: &str) -> Result<Handle, String> {
        let mut flows = self.flows.lock().map_err(|_| "认证服务繁忙")?;
        if flows.get(id).is_some_and(|h| h.created.elapsed() >= TTL) {
            if let Some(h) = flows.remove(id) {
                h.cancelled.store(true, Ordering::SeqCst);
            }
        }
        flows
            .get(id)
            .and_then(|slot| slot.handle.clone())
            .ok_or("登录流程不存在或已过期，请重新开始".into())
    }
    pub fn cancel(&self, id: &str) -> Result<(), String> {
        if let Some(h) = self.flows.lock().map_err(|_| "认证服务繁忙")?.remove(id) {
            h.cancelled.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
    pub fn cancel_all(&self) {
        if let Ok(mut flows) = self.flows.lock() {
            for (_, h) in flows.drain() {
                h.cancelled.store(true, Ordering::SeqCst);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_jwt_system_type_falls_back_safely() {
        let payload = URL_SAFE_NO_PAD.encode(r#"{"systemType":"yingxin_teacher_pc"}"#);
        assert_eq!(
            jwt_system_type(&format!("h.{payload}.s")),
            "yingxin_teacher_pc"
        );
        for token in ["", "onlyone", "h..s", "h.bm90LWpzb24.s"] {
            assert_eq!(jwt_system_type(token), "yingxin_student_pc");
        }
    }
    #[test]
    fn campus_public_key_accepts_unwrapped_base64_and_crlf() {
        use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey};
        let private = RsaPrivateKey::new(&mut rand::rngs::OsRng, 1024).unwrap();
        let public = RsaPublicKey::from(&private);
        let pem = public
            .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
            .unwrap();
        let body: String = pem
            .lines()
            .filter(|line| !line.starts_with("-----"))
            .collect();
        for text in [
            pem,
            format!("-----BEGIN PUBLIC KEY-----\n{body}\n-----END PUBLIC KEY-----"),
            format!("-----BEGIN PUBLIC KEY----- \r\n{body}\r\n-----END PUBLIC KEY----- \r\n"),
        ] {
            let encrypted = encrypt_password("local-test-password", &text).unwrap();
            let bytes = STANDARD
                .decode(encrypted.strip_prefix("__RSA__").unwrap())
                .unwrap();
            assert_eq!(
                private.decrypt(Pkcs1v15Encrypt, &bytes).unwrap(),
                b"local-test-password"
            );
        }
    }
    #[test]
    fn campus_public_key_supports_pkcs1_and_rejects_invalid_envelopes() {
        use rsa::{pkcs1::EncodeRsaPublicKey, RsaPrivateKey};
        let private = RsaPrivateKey::new(&mut rand::rngs::OsRng, 1024).unwrap();
        let public = RsaPublicKey::from(&private);
        let pem = public.to_pkcs1_pem(rsa::pkcs1::LineEnding::LF).unwrap();
        assert_eq!(parse_public_key(&pem).unwrap(), public);
        for invalid in [
            "<html>login required</html>",
            "-----BEGIN PUBLIC KEY-----\ninvalid!\n-----END PUBLIC KEY-----",
            "-----BEGIN PUBLIC KEY-----\nMAA=\n-----END PUBLIC KEY-----",
            "-----BEGIN PUBLIC KEY-----\nMAA=\n-----END RSA PUBLIC KEY-----",
        ] {
            assert!(parse_public_key(invalid).is_err());
        }
        assert!(parse_public_key(&pem.replace("RSA PUBLIC KEY", "PUBLIC KEY")).is_err());
    }
    #[test]
    #[ignore = "Fetches a public key only; does not submit credentials"]
    fn live_campus_public_key_probe() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let login = Login::begin("undergraduate".into()).await.unwrap();
            let response = login
                .transport
                .get(
                    &Url::parse(&format!("{CAS}/cas/jwt/publicKey")).unwrap(),
                    login.request_mode(),
                )
                .await
                .unwrap();
            let pem = String::from_utf8(network::limited_bytes(response, 32 * 1024).await.unwrap())
                .unwrap();
            encrypt_password("local-probe-never-submitted", &pem).unwrap();
            println!(
                "School public key parsed and local encryption succeeded; no credentials submitted"
            );
        });
    }
    #[test]
    fn ui_actions_decode_remember_password_and_saved_account_fields() {
        let action: Action = serde_json::from_value(
            json!({"type":"credentials","username":"u","password":"p","rememberPassword":true}),
        )
        .unwrap();
        assert!(matches!(
            action,
            Action::Credentials {
                remember_password: true,
                ..
            }
        ));
        let saved: Action = serde_json::from_value(
            json!({"type":"savedCredentials","accountId":"local-id","rememberPassword":false}),
        )
        .unwrap();
        assert!(matches!(
            saved,
            Action::SavedCredentials {
                remember_password: false,
                ..
            }
        ));
    }
    #[test]
    fn cancellation_and_expiry_invalidate_inflight_initialization() {
        let manager = Manager::default();
        let id = Uuid::new_v4().to_string();
        let token = manager.reserve(&id).unwrap();
        manager.cancel(&id).unwrap();
        assert!(token.load(Ordering::SeqCst));
        assert!(manager
            .complete(&id, &token, Err("late result".into()))
            .is_err());
        let token = manager.reserve(&id).unwrap();
        manager.flows.lock().unwrap().get_mut(&id).unwrap().created =
            Instant::now() - TTL - Duration::from_secs(1);
        manager.reserve(&Uuid::new_v4().to_string()).unwrap();
        assert!(token.load(Ordering::SeqCst));
        assert!(manager.get(&id).is_err());
        manager.cancel_all();
        assert!(manager.flows.lock().unwrap().is_empty());
    }
    #[test]
    fn captcha_survives_mfa_pause_without_resubmitting_credentials() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut login = Login {
                id: Uuid::new_v4().to_string(),
                created: Instant::now(),
                transport: Transport::new(None).unwrap(),
                degree: "undergraduate".into(),
                mode: AccessMode::Direct,
                step: Step::Campus,
                phase: Phase::Captcha,
                page: Some(Page {
                    url: Url::parse("https://login.xjtu.edu.cn/cas/login").unwrap(),
                    status: StatusCode::OK,
                    html: "<input name='execution' value='e1s1'>".into(),
                }),
                username: "student".into(),
                encrypted: Zeroizing::new("encrypted-test".into()),
                retained_password: None,
                pending_captcha: String::new(),
                visitor: "test".into(),
                mfa: Some(Mfa {
                    state: "mfa-state".into(),
                    flow: "mfa",
                    gid: Some("gid".into()),
                    phone: None,
                    verified: false,
                    sent: None,
                }),
                choices: vec![],
                chosen: None,
                failures: 3,
                message: None,
                completed: None,
            };
            assert!(login.password_page("ABC1".into()).await.unwrap().is_none());
            assert_eq!(login.phase as u8, Phase::Mfa as u8);
            assert!(login.password_page(String::new()).await.unwrap().is_none());
            assert_eq!(login.pending_captcha, "ABC1");
        });
    }
    #[test]
    #[ignore = "Network-only entry probe; never submits credentials or sends SMS"]
    fn live_login_entry_probe() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let login = runtime
            .block_on(Login::begin("undergraduate".into()))
            .unwrap();
        let view = login.view();
        assert!(matches!(view.phase, Phase::Credentials | Phase::Captcha));
        println!(
            "Entry reachable; route={:?}, credentials form parsed",
            view.access_mode
        );
    }
    #[test]
    fn html_fields_choices_and_safety_forms_are_parsed_without_executing_scripts() {
        let form=parse_form(r#"<form><input value='e1s1' name='execution'><input name='secState' value='s&amp;x'><input name='_eventId' value='submit'></form><div class='account-wrap'><div class='name'>研究生</div><el-radio label='20260000'></el-radio></div>"#).unwrap();
        assert_eq!(form.execution, "e1s1");
        assert_eq!(form.sec_state.as_deref(), Some("s&x"));
        assert_eq!(form.choices[0].label, "20260000");
        assert!(form.mfa_enabled);
        assert!(parse_form("<title>Unexpected gateway</title>").is_none());
    }
    #[test]
    fn password_encryption_matches_rsa_pkcs1_protocol_and_never_returns_plaintext() {
        use rsa::{pkcs8::EncodePublicKey, RsaPrivateKey};
        let private = RsaPrivateKey::new(&mut rand::rngs::OsRng, 2048).unwrap();
        let pem = private
            .to_public_key()
            .to_public_key_pem(Default::default())
            .unwrap();
        let encrypted = encrypt_password("a-test-password", &pem).unwrap();
        assert!(encrypted.starts_with("__RSA__"));
        let bytes = STANDARD
            .decode(encrypted.trim_start_matches("__RSA__"))
            .unwrap();
        assert_eq!(
            private.decrypt(Pkcs1v15Encrypt, &bytes).unwrap(),
            b"a-test-password"
        );
        assert!(encrypt_password("x", "not-a-key").is_err());
    }
    #[test]
    fn unknown_mfa_config_defaults_to_detection_and_explicit_false_is_respected() {
        let html = r#"<input name='execution' value='e1s1'><script>globalConfig = eval('(' + "{\"mfaEnabled\":false}" + ')');</script>"#;
        assert!(!parse_form(html).unwrap().mfa_enabled);
        assert!(
            parse_form("<input name='execution' value='e1s1'>")
                .unwrap()
                .mfa_enabled
        );
    }
}
