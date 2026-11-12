//! Physical-education platform adapters (`tyxylp.xjtu.edu.cn`).
//!
//! One CAS session serves two API generations:
//!
//! - `bdlp_h5_fitness_test/public/index.php/index/...` — form POST, replies
//!   `{status, data, info}`; `status == -1` means the session expired;
//! - `v3/index.php/...` — campus run data. The reply envelope has not been
//!   verified with a live account yet, so those helpers return the decoded
//!   JSON untouched instead of guessing a field layout.
//!
//! Every endpoint that changes state (`Appoint/setOnAppointment`,
//! `Free/setFree`, `stadium/checkOrder`, `match/join`, `Running/applyFreeRun`)
//! is only assembled here. A caller must show a preview and receive explicit
//! confirmation before sending it, and must not retry automatically when the
//! result is unknown.
pub(crate) mod contest;
pub(crate) mod course;
pub(crate) mod fitness;
pub(crate) mod run;
pub(crate) mod stadium;

use crate::accounts::Account;
use crate::network::limited_bytes;
use md5::Digest;
use reqwest::Client as Http;
use serde_json::Value;
use url::Url;

const HOST: &str = "https://tyxylp.xjtu.edu.cn";
pub(crate) const LOGIN: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/public/index.php/index/login/xjtuLogin";
const LEGACY: &str = "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/public/index.php/index";
/// Signature salt used by the platform's own H5 page (public client constant).
const SIGN_SALT: &str = "rDJiNB9j7vD2";
const LEGACY_REFERER: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/fitness/homepage/year_fitness.html";
const H5_HOME: &str =
    "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/h5xajt/#/pages/index/index";

/// Shared wording for an expired campus session.
pub(crate) const IDLE: &str = "体测登录态已失效，请重新登录校园账号";

/// The launch parameters the H5 page receives in its URL fragment after CAS.
///
/// They are the platform's own session tokens: the legacy PHP endpoints
/// (year list, score, measurements) only answer once these have been exchanged
/// at `Index/checkLogin`. Stored under `sites.fitness.token`; never logged.
#[derive(Clone)]
pub(crate) struct Launch {
    uid: String,
    token: String,
    school_id: String,
    term_id: String,
    student_num: String,
    card_id: String,
    user_type: String,
    login_type: String,
    referer: String,
}

impl Launch {
    /// Read the tokens out of the H5 callback, whose query sits after `#`.
    pub(crate) fn parse(url: &Url) -> Result<Self, String> {
        if url.host_str() != Some("tyxylp.xjtu.edu.cn") {
            return Err("体测登录回调地址异常".into());
        }
        let fragment = url.fragment().ok_or("体测登录回调缺少会话参数")?;
        let (path, query) = fragment.split_once('?').ok_or("体测登录回调缺少会话参数")?;
        let params: std::collections::HashMap<String, String> =
            url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();
        let field = |key: &str| {
            params
                .get(key)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .ok_or_else(|| "体测登录回调缺少会话参数".to_string())
        };
        let mut base = url.clone();
        base.set_fragment(None);
        base.set_query(None);
        Ok(Self {
            uid: field("uid")?,
            token: field("token")?,
            school_id: field("school_id")?,
            term_id: field("term_id")?,
            student_num: field("student_num")?,
            card_id: field("card_id")?,
            user_type: field("user_type")?,
            login_type: params
                .get("login_type")
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_else(|| "4".into()),
            referer: format!("{base}#{path}"),
        })
    }

    /// Serialize for `sites.fitness.token`.
    pub(crate) fn encode(&self) -> String {
        serde_json::json!({
            "uid": self.uid,
            "token": self.token,
            "schoolId": self.school_id,
            "termId": self.term_id,
            "studentNum": self.student_num,
            "cardId": self.card_id,
            "userType": self.user_type,
            "loginType": self.login_type,
            "referer": self.referer,
        })
        .to_string()
    }

    pub(crate) fn decode(raw: &str) -> Option<Self> {
        let value: Value = serde_json::from_str(raw).ok()?;
        let field = |key: &str| {
            value
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_string)
                .filter(|text| !text.is_empty())
        };
        Some(Self {
            uid: field("uid")?,
            token: field("token")?,
            school_id: field("schoolId")?,
            term_id: field("termId")?,
            student_num: field("studentNum")?,
            card_id: field("cardId")?,
            user_type: field("userType")?,
            login_type: field("loginType").unwrap_or_else(|| "4".into()),
            referer: field("referer").unwrap_or_else(|| H5_HOME.to_string()),
        })
    }
}

/// Exchange the launch tokens for the legacy PHP session. The platform signs
/// the sorted `key + value` pairs (plus its salt) and posts them once on the
/// H5 callback; every later legacy request relies on the resulting cookie.
pub(crate) async fn check_login(http: &Http, launch: &Launch) -> Result<(), String> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|_| "系统时间异常")?
        .as_secs();
    let nonce = format!("{:06}", rand::random::<u32>() % 1_000_000);
    let fields = [
        ("timestamp", seconds.to_string()),
        ("nonce", nonce),
        ("course_id", "0".to_string()),
        ("uid", launch.uid.clone()),
        ("card_id", launch.card_id.clone()),
        ("login_type", launch.login_type.clone()),
        ("type", "1".to_string()),
        ("school_id", launch.school_id.clone()),
        ("student_num", launch.student_num.clone()),
        ("user_type", launch.user_type.clone()),
        ("token", launch.token.clone()),
    ];
    let mut sorted: Vec<(&str, &str)> = fields
        .iter()
        .map(|(key, value)| (*key, value.as_str()))
        .collect();
    sorted.sort_by_key(|(key, _)| *key);
    let source = sorted
        .iter()
        .map(|(key, value)| format!("{key}{value}"))
        .collect::<String>()
        + SIGN_SALT;
    let sign = format!("{:x}", md5::Md5::digest(source.as_bytes()));
    let mut form = sorted;
    form.push(("sign", sign.as_str()));
    form.push(("term_id", ""));
    form.push(("id", ""));
    let response = http
        .post(format!("{LEGACY}/Index/checkLogin"))
        .header("Origin", HOST)
        .header("Referer", LEGACY_REFERER)
        .header("X-Requested-With", "XMLHttpRequest")
        .form(&form)
        .send()
        .await
        .map_err(|_| "无法连接体测服务，请检查网络后重试")?;
    let status = response.status();
    if status.as_u16() >= 500 {
        return Err("学校体测服务暂不可用，请过段时间再试".into());
    }
    if !status.is_success() {
        return Err(format!("体测登录未完成（HTTP {}）", status.as_u16()));
    }
    let bytes = limited_bytes(response, 128 * 1024).await?;
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "体测登录返回了非预期数据")?;
    let ok = matches!(
        value.get("status"),
        Some(Value::Number(number)) if number.as_i64() == Some(1)
    ) || matches!(value.get("status"), Some(Value::String(text)) if text.trim() == "1");
    if ok {
        Ok(())
    } else {
        Err(info(&value).unwrap_or_else(|| "体测系统未完成登录，请稍后重试".into()))
    }
}

/// A verified session. `connect` proves it with a real score query, because the
/// login redirect alone also succeeds when only the CAS cookie is present.
pub(crate) struct Session {
    http: Http,
}

impl Session {
    pub(crate) async fn connect(account: &Account) -> Result<Self, String> {
        let site = account.sites.get("fitness");
        let saved = site
            .and_then(|site| site.cookies.as_deref())
            .filter(|cookies| !cookies.is_empty());
        // The fitness site keeps its own PHP session; the shared direct jar is
        // only the fallback before the first service login.
        let http = match saved {
            Some(cookies) => super::direct::client(cookies)?,
            None => super::direct::account(account)?,
        };
        let session = Self { http };
        match session.fitness().years().await {
            Ok(_) => Ok(session),
            Err(error) if error == IDLE => {
                // A stored launch token can rebuild the PHP session without a
                // password prompt; the cookie snapshot is persisted by the
                // refresh command, so this is only the in-memory fast path.
                if let Some(launch) = site
                    .and_then(|site| site.token.as_deref())
                    .and_then(Launch::decode)
                {
                    if check_login(&session.http, &launch).await.is_ok() {
                        session.fitness().years().await?;
                        return Ok(session);
                    }
                }
                Err(error)
            }
            Err(error) => Err(error),
        }
    }

    pub(crate) fn fitness(&self) -> fitness::Client<'_> {
        fitness::Client { session: self }
    }

    pub(crate) fn run(&self) -> run::Client<'_> {
        run::Client { session: self }
    }

    pub(crate) fn course(&self) -> course::Client<'_> {
        course::Client { session: self }
    }

    pub(crate) fn stadium(&self) -> stadium::Client<'_> {
        stadium::Client { session: self }
    }

    pub(crate) fn contest(&self) -> contest::Client<'_> {
        contest::Client { session: self }
    }

    /// `index.php` endpoint; returns the checked envelope so callers that need
    /// top-level fields (for example `Free/getYear`) can read them too.
    async fn post(&self, path: &str, form: &[(&str, &str)]) -> Result<Value, String> {
        let value = self.send(path, form).await?;
        match status(&value) {
            Some(1) => Ok(value),
            Some(-1) => Err(IDLE.into()),
            _ => {
                let message = info(&value).unwrap_or_else(|| "体测请求失败".into());
                // The legacy PHP endpoints answer an expired session with
                // `登录信息失效` rather than the -1 status.
                if message.contains("登录信息失效") || message.contains("请重新登录") {
                    Err(IDLE.into())
                } else {
                    Err(message)
                }
            }
        }
    }

    /// `v3` endpoint. Returns `data` when the reply uses the familiar envelope,
    /// otherwise the decoded body as-is.
    async fn post_v3(&self, path: &str, form: &[(&str, &str)]) -> Result<Value, String> {
        let value = self.send(path, form).await?;
        match status(&value) {
            Some(1) => Ok(value.get("data").cloned().unwrap_or(Value::Null)),
            Some(-1) => Err(IDLE.into()),
            Some(_) => Err(info(&value).unwrap_or_else(|| "体测请求失败".into())),
            None => Ok(value),
        }
    }

    async fn send(&self, path: &str, form: &[(&str, &str)]) -> Result<Value, String> {
        let response = self
            .http
            .post(endpoint(path)?)
            .header("Origin", HOST)
            .header("Referer", H5_HOME)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(form)
            .send()
            .await
            .map_err(|_| "体测请求未收到响应")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err(IDLE.into());
        }
        if !status.is_success() {
            return Err(format!("体测服务返回 HTTP {}", status.as_u16()));
        }
        let bytes = limited_bytes(response, 2 * 1024 * 1024).await?;
        serde_json::from_slice(&bytes).map_err(|_| "体测返回了非预期数据".into())
    }
}

/// Builds an absolute URL and refuses anything that could escape the host.
fn endpoint(path: &str) -> Result<String, String> {
    if path.is_empty()
        || path.starts_with('/')
        || path.contains("..")
        || path.contains("://")
        || path.contains('\\')
    {
        return Err("体测接口路径无效".into());
    }
    Ok(format!("{HOST}/{path}"))
}

fn status(value: &Value) -> Option<i64> {
    value.get("status").and_then(Value::as_i64)
}

fn info(value: &Value) -> Option<String> {
    text(value.get("info"))
}

/// The `data` payload of a checked envelope.
pub(super) fn data(value: &Value) -> Value {
    value.get("data").cloned().unwrap_or(Value::Null)
}

pub(super) fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn endpoints_stay_on_the_platform_host() {
        assert_eq!(
            endpoint("v3/index.php/Rule/running").unwrap(),
            "https://tyxylp.xjtu.edu.cn/v3/index.php/Rule/running"
        );
        for bad in [
            "",
            "/index/x",
            "../index/x",
            "https://example.com/x",
            "a\\b",
        ] {
            assert!(endpoint(bad).is_err(), "{bad} should be rejected");
        }
    }

    #[test]
    fn launch_tokens_round_trip_through_the_stored_form() {
        let url = Url::parse(
            "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/h5xajt/\
             #/pages/index/index?uid=42&token=abc&school_id=1&term_id=2025&\
             student_num=0001&card_id=99&user_type=2&login_type=4",
        )
        .unwrap();
        let launch = Launch::parse(&url).unwrap();
        assert_eq!(
            launch.referer,
            "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/h5xajt/#/pages/index/index"
        );
        let stored = Launch::decode(&launch.encode()).unwrap();
        assert_eq!(stored.uid, "42");
        assert_eq!(stored.token, "abc");
        assert_eq!(stored.login_type, "4");
        assert_eq!(stored.referer, launch.referer);
    }

    #[test]
    fn launch_rejects_foreign_or_incomplete_callbacks() {
        for bad in [
            "https://example.com/#/pages/index/index?uid=1",
            "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/h5xajt/#/pages/index/index",
            "https://tyxylp.xjtu.edu.cn/bdlp_h5_fitness_test/view/h5xajt/#/pages/index/index?uid=1&token=",
        ] {
            assert!(Launch::parse(&Url::parse(bad).unwrap()).is_err(), "{bad}");
        }
    }

    #[test]
    fn envelope_helpers_read_the_familiar_shape() {
        assert_eq!(status(&json!({"status": 1, "data": []})), Some(1));
        assert_eq!(
            info(&json!({"status": 0, "info": " 已结束 "})),
            Some("已结束".into())
        );
        assert_eq!(data(&json!({"status": 1, "data": 2025})), json!(2025));
        assert_eq!(text(Some(&json!(2025))), Some("2025".into()));
    }
}
