//! Campus card balance and transactions. The card service is its own SSO client:
//! the saved CAS cookies are exchanged for a short-lived bearer token.
use crate::accounts::Account;
use reqwest::{redirect::Policy, Client as Http};
use serde_json::{json, Value};
use std::time::Duration;
use url::Url;

pub(crate) const LOGIN_URL: &str =
    "https://ncard.xjtu.edu.cn/berserker-base/redirect?type=login&loginFrom=h5&synAccessSource=h5";
const TOKEN_URL: &str = "https://ncard.xjtu.edu.cn/berserker-auth/oauth/token";
const USER_URL: &str = "https://ncard.xjtu.edu.cn/berserker-base/user?synAccessSource=h5";
const CARD_URL: &str =
    "https://ncard.xjtu.edu.cn/berserker-app/ykt/tsm/queryCard?synAccessSource=h5";
const TURNOVER_URL: &str = "https://ncard.xjtu.edu.cn/berserker-search/search/personal/turnover";
// Public client credentials used by the H5 campus-card frontend.
const BASIC_AUTH: &str =
    "Basic bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm06bW9iaWxlX3NlcnZpY2VfcGxhdGZvcm1fc2VjcmV0";

pub struct Client {
    http: Http,
    token: String,
    pub name: String,
    pub student_no: String,
    pub account: String,
}

fn card_host(url: &Url) -> bool {
    url.scheme() == "https"
        && url
            .host_str()
            .is_some_and(|host| host == "xjtu.edu.cn" || host.ends_with(".xjtu.edu.cn"))
}

fn http_client(account: &Account) -> Result<Http, String> {
    // The card service rejects WebVPN sessions; prefer its own direct CAS login.
    let saved = account
        .direct_cookies
        .as_deref()
        .unwrap_or(&account.cookies);
    let transport = crate::network::Transport::new(Some(saved))?;
    Http::builder()
        .cookie_provider(transport.cookies.clone())
        .redirect(Policy::custom(|attempt| {
            if card_host(attempt.url()) {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        // The CAS session is bound to the user agent that created it, so the
        // card client must keep the shared transport agent; ncard accepts it.
        .user_agent(crate::network::USER_AGENT)
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|_| "无法创建校园卡连接".into())
}

impl Client {
    pub async fn connect(account: &Account) -> Result<Self, String> {
        let http = http_client(account)?;
        let response = http
            .get(LOGIN_URL)
            .send()
            .await
            .map_err(|_| "无法连接校园卡服务，请检查网络后重试")?;
        let ticket = response
            .url()
            .query_pairs()
            .find(|(key, _)| key == "ticket")
            .map(|(_, value)| value.into_owned())
            .filter(|value| !value.is_empty())
            .ok_or("校园卡登录态已失效，请重新登录校园账号")?;
        let token = http
            .post(TOKEN_URL)
            .header(reqwest::header::AUTHORIZATION, BASIC_AUTH)
            .form(&[
                ("username", ticket.as_str()),
                ("password", ticket.as_str()),
                ("grant_type", "password"),
                ("scope", "all"),
                ("loginFrom", "h5"),
                ("logintype", "sso"),
                ("device_token", "h5"),
                ("synAccessSource", "h5"),
            ])
            .send()
            .await
            .map_err(|_| "校园卡登录请求失败")?;
        let token: Value = token
            .json()
            .await
            .map_err(|_| "校园卡登录返回了非预期数据")?;
        let access = token
            .get("access_token")
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .ok_or("校园卡登录未返回访问令牌，请重新登录校园账号")?
            .to_string();
        let mut client = Self {
            http,
            token: access,
            name: String::new(),
            student_no: String::new(),
            account: String::new(),
        };
        let user = client.get(USER_URL).await?;
        client.name = text(user.get("name")).ok_or("校园卡用户资料缺少姓名")?;
        client.student_no = text(user.get("sno")).ok_or("校园卡用户资料缺少学号")?;
        client.account = text(user.get("cardAccount")).ok_or("校园卡用户资料缺少卡号")?;
        Ok(client)
    }

    async fn get(&self, url: &str) -> Result<Value, String> {
        let response = self
            .http
            .get(url)
            .header("Synjones-Auth", format!("bearer {}", self.token))
            .header("synAccessSource", "h5")
            .send()
            .await
            .map_err(|_| "校园卡请求未收到响应")?;
        let status = response.status();
        if matches!(status.as_u16(), 401 | 403) {
            return Err("校园卡登录态已失效，请重新登录校园账号".into());
        }
        if !status.is_success() {
            return Err(format!("校园卡服务返回 HTTP {}", status.as_u16()));
        }
        let payload: Value = response
            .json()
            .await
            .map_err(|_| "校园卡返回了非预期数据")?;
        let code = payload
            .get("code")
            .map(|value| value.to_string().trim_matches('"').to_string())
            .unwrap_or_default();
        if code != "200" {
            return Err("校园卡请求被拒绝，请重新登录后重试".into());
        }
        payload
            .get("data")
            .cloned()
            .filter(Value::is_object)
            .ok_or_else(|| "校园卡返回的数据格式已变化".into())
    }

    pub async fn info(&self) -> Result<Value, String> {
        let data = self.get(CARD_URL).await?;
        let card = data
            .get("card")
            .and_then(Value::as_array)
            .and_then(|cards| cards.first())
            .and_then(Value::as_object)
            .ok_or("校园卡没有返回卡片信息")?;
        Ok(json!({
            "name": self.name,
            "studentNo": self.student_no,
            "account": self.account,
            "balance": integer(card.get("elec_accamt")).ok_or("校园卡余额格式错误")?,
            "pending": integer(card.get("unsettle_amount")).unwrap_or(0),
            "lost": card.get("barflag").and_then(Value::as_i64) == Some(1),
            "frozen": card.get("freezeflag").and_then(Value::as_i64) == Some(1),
            "cardType": text(card.get("cardname")).unwrap_or_default(),
            "expireDate": expire_date(text(card.get("expdate")).as_deref().unwrap_or("")),
        }))
    }

    pub async fn transactions(&self, start: &str, end: &str, page: u32) -> Result<Value, String> {
        let url = Url::parse_with_params(
            TURNOVER_URL,
            &[
                ("size", "30"),
                ("current", &page.to_string()),
                ("timeFrom", start),
                ("timeTo", end),
                ("synAccessSource", "h5"),
            ],
        )
        .map_err(|_| "校园卡流水地址无效")?;
        let data = self.get(url.as_str()).await?;
        let total = integer(data.get("total")).ok_or("校园卡流水总数格式错误")?;
        let records = data
            .get("records")
            .and_then(Value::as_array)
            .ok_or("校园卡流水格式已变化")?;
        Ok(json!({
            "page": page,
            "total": total,
            "rows": records.iter().filter_map(|item| {
                let amount = integer(item.get("tranamt"))?;
                let balance = integer(item.get("cardBalance")).unwrap_or(0);
                let type_name = text(item.get("turnoverType")).unwrap_or_default();
                let description = text(item.get("resume")).unwrap_or_default();
                let merchant = text(item.get("toMerchant"))
                    .or_else(|| description.split('-').next().map(str::to_string))
                    .unwrap_or_default();
                Some(json!({
                    "time": text(item.get("jndatetimeStr")).unwrap_or_default(),
                    "amount": signed_amount(amount, &type_name),
                    "balance": balance,
                    "merchant": merchant,
                    "type": type_name,
                    "description": description,
                }))
            }).collect::<Vec<_>>()
        }))
    }
}

fn text(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(text) => Some(text.trim().to_string()).filter(|text| !text.is_empty()),
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

/// Accepts integers and numeric strings; booleans are rejected like the reference.
fn integer(value: Option<&Value>) -> Option<i64> {
    match value? {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.trim().parse().ok(),
        _ => None,
    }
}

fn expire_date(value: &str) -> String {
    if value.len() == 8 && value.bytes().all(|byte| byte.is_ascii_digit()) {
        format!("{}-{}-{}", &value[..4], &value[4..6], &value[6..])
    } else {
        value.to_string()
    }
}

const INCOME: [&str; 6] = ["充值", "圈存", "退款", "补助", "recharge", "refund"];
const EXPENSE: [&str; 5] = ["消费", "支出", "扣款", "consume", "expense"];

/// Keeps the server's sign, then falls back to known income/expense markers.
fn signed_amount(amount: i64, type_name: &str) -> i64 {
    if amount < 0 {
        return amount;
    }
    let normalized = type_name.to_lowercase();
    if INCOME.iter().any(|marker| normalized.contains(marker)) {
        return amount.abs();
    }
    if EXPENSE.iter().any(|marker| normalized.contains(marker)) {
        return -amount.abs();
    }
    amount
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cents_and_dates() {
        assert_eq!(integer(Some(&json!(1250))), Some(1250));
        assert_eq!(integer(Some(&json!("1250"))), Some(1250));
        assert_eq!(integer(Some(&json!(true))), None);
        assert_eq!(expire_date("20261231"), "2026-12-31");
        assert_eq!(expire_date(""), "");
    }

    #[test]
    fn signs_amounts_by_type_markers() {
        assert_eq!(signed_amount(500, "消费"), -500);
        assert_eq!(signed_amount(500, "消费退款"), 500);
        assert_eq!(signed_amount(500, "充值"), 500);
        assert_eq!(signed_amount(-500, "充值"), -500);
        assert_eq!(signed_amount(500, "其他"), 500);
    }
}
