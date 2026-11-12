//! Sports venue adapter (read-only). The booking site is plain HTTP on the
//! campus network and keeps its own OAuth CAS session under `sites.venue`.
use crate::{
    accounts::Account,
    network::{AccessMode, Transport},
};
use serde_json::{json, Value};
use url::Url;

const BASE: &str = "http://202.117.17.144:8080";
pub(crate) const HOME_URL: &str = "http://202.117.17.144:8080/web/index.html";
pub(crate) const LOGIN_URL: &str = "https://org.xjtu.edu.cn/openplatform/oauth/authorize?responseType=code&scope=user_info&appId=1659&state=1&redirectUri=http%3A%2F%2F202.117.17.144%3A8080%2Fweb%2Fcas%2Foauth2url.html";
const STATUS: [(i64, &str); 3] = [(0, "预订中"), (1, "预订成功"), (2, "预订取消")];

fn text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.trim().to_string(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

fn number(value: Option<&Value>) -> f64 {
    match value {
        Some(Value::Number(number)) => number.as_f64().unwrap_or(0.0),
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn integer(value: Option<&Value>) -> i64 {
    match value {
        Some(Value::Number(number)) => number.as_i64().unwrap_or(0),
        Some(Value::String(text)) => text.trim().parse().unwrap_or(0),
        _ => 0,
    }
}

/// The venue endpoints are older than the frontend wrapper: some return the
/// page as a bare array, others wrap it in `object`/`rows`. Accept both rather
/// than rejecting a reply that carries the rows we need.
fn rows_of(value: &Value) -> Option<&[Value]> {
    if let Some(rows) = value.as_array() {
        return Some(rows);
    }
    ["object", "rows", "data", "list"]
        .iter()
        .find_map(|key| value.get(key).and_then(Value::as_array).map(Vec::as_slice))
}

/// The site answers notices (for example "not in booking hours") with an HTML
/// page, so JSON parsing alone cannot tell success from failure. An expired
/// session can also answer 200 with a login page; that must report an expired
/// session so the caller renews the login instead of showing the page title.
fn notice(bytes: &[u8]) -> Option<String> {
    let body = String::from_utf8_lossy(bytes);
    let trimmed = body.trim_start();
    if !trimmed.starts_with('<') {
        return None;
    }
    let title = trimmed
        .split_once("<title>")
        .and_then(|(_, rest)| rest.split_once("</title>"))
        .map(|(title, _)| title.trim().to_string())
        .filter(|title| !title.is_empty());
    if title
        .as_deref()
        .is_some_and(|title| title.contains("登录") || title.contains("认证"))
        || trimmed.contains("oauth2url")
        || trimmed.contains("cas/login")
        || trimmed.contains("统一身份")
    {
        return Some("场馆登录已失效，请重新登录".into());
    }
    // Outside 8:40–21:40 the site closes every endpoint and answers with its
    // own "not a booking time" page. That is the service being off for the
    // evening, not a network or account problem, so say so plainly.
    if title.as_deref().is_some_and(|title| title.contains("预订")) || trimmed.contains("再来预订")
    {
        return Some("晚上服务不可用（预约开放 8:40–21:40）".into());
    }
    Some(title.unwrap_or_else(|| "场馆系统返回了无法识别的页面".into()))
}

/// The venue home page fills a hidden `userno` input with the logged-in user.
/// A cookie alone proves nothing: the anonymous landing page also sets
/// `SESSION`, so this is the only reliable signal that the OAuth chain reached
/// the venue and issued a usable session.
pub(crate) fn index_logged_in(html: &str) -> bool {
    let Some((_, rest)) = html.split_once("userno") else {
        return false;
    };
    let window = &rest[..rest.len().min(200)];
    let Some((_, after)) = window.split_once("value=\"") else {
        return false;
    };
    !after
        .split('"')
        .next()
        .unwrap_or_default()
        .trim()
        .is_empty()
}

async fn fetch(
    transport: &Transport,
    mode: AccessMode,
    path: &str,
    params: &[(&str, String)],
) -> Result<Value, String> {
    if !path.starts_with("/web/") || path.contains("..") {
        return Err("场馆接口路径无效".into());
    }
    let url =
        Url::parse_with_params(&format!("{BASE}{path}"), params).map_err(|_| "场馆接口地址无效")?;
    let mut response = send(transport, mode, &url).await?;
    // The venue answers a closed booking window (and some other notices) with
    // a redirect to its own notice page instead of JSON. Follow on-site
    // redirects so that notice reaches the user; a redirect anywhere else is
    // the CAS bounce that means the session expired.
    for _ in 0..3 {
        if !response.status().is_redirection() {
            break;
        }
        let location = response
            .headers()
            .get(reqwest::header::LOCATION)
            .and_then(|value| value.to_str().ok())
            .ok_or("场馆登录已失效，请重新登录")?;
        let next = response
            .url()
            .join(location)
            .map_err(|_| "场馆跳转地址无效")?;
        let logical = crate::network::original_url(&next)?;
        if logical.origin() != Url::parse(BASE).unwrap().origin() {
            return Err("场馆登录已失效，请重新登录".into());
        }
        response = send(transport, mode, &logical).await?;
    }
    let status = response.status();
    if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
        return Err("场馆登录已失效，请重新登录".into());
    }
    if !status.is_success() {
        return Err(format!("场馆系统暂不可用（HTTP {}）", status.as_u16()));
    }
    let bytes = crate::network::limited_bytes(response, 4 * 1024 * 1024).await?;
    if let Some(notice) = notice(&bytes) {
        return Err(notice);
    }
    serde_json::from_slice(&bytes).map_err(|_| "场馆系统返回了非预期数据".into())
}

async fn send(
    transport: &Transport,
    mode: AccessMode,
    url: &Url,
) -> Result<reqwest::Response, String> {
    let address = Transport::address(url, mode)?;
    transport
        .client
        .get(address)
        .header("Referer", format!("{BASE}/web/index.html"))
        .send()
        .await
        .map_err(|_| "无法连接场馆系统，请检查网络后重试".into())
}

/// Name the fields of an unexpected reply; venue data is public, and this is
/// what tells a shape change apart from an error page during联调.
fn list_shape_error(label: &str, value: &Value) -> String {
    let keys = value
        .as_object()
        .map(|object| object.keys().cloned().collect::<Vec<_>>().join(", "))
        .unwrap_or_else(|| "非对象".into());
    format!("{label}没有返回可识别的数据（字段：{keys}）")
}

pub struct Client {
    transport: Transport,
    mode: AccessMode,
}

impl Client {
    pub fn connect(account: &Account) -> Result<Self, String> {
        // The venue is plain HTTP on the campus network. Off campus the same
        // origin is reachable through the WebVPN port mapping (`http-8080`),
        // so the account's current mode decides how each request is addressed.
        let site = account.sites.get("venue").ok_or("请先完成场馆系统登录")?;
        let cookies = site
            .cookies
            .as_deref()
            .filter(|cookies| !cookies.is_empty())
            .ok_or("请先完成场馆系统登录")?;
        Ok(Self {
            transport: Transport::new(Some(cookies))?,
            mode: account.mode,
        })
    }

    async fn get(&self, path: &str, params: &[(&str, String)]) -> Result<Value, String> {
        fetch(&self.transport, self.mode, path, params).await
    }

    pub async fn venues(&self) -> Result<Value, String> {
        let mut venues = Vec::new();
        for page in 1..=20 {
            let value = self
                .get(
                    "/web/product/productData.html",
                    &[
                        ("page", page.to_string()),
                        ("rows", "8".into()),
                        ("merccode", "100001".into()),
                        ("remark", "defaultProList".into()),
                    ],
                )
                .await?;
            let rows = rows_of(&value).ok_or_else(|| list_shape_error("场馆列表", &value))?;
            venues.extend(rows.iter().filter_map(venue));
            if rows.len() < 8 {
                break;
            }
        }
        Ok(json!({"venues": venues}))
    }

    pub async fn slots(&self, venue: u64, date: &str) -> Result<Value, String> {
        let available = self
            .get(
                "/web/product/findOkArea.html",
                &[
                    ("s_date", date.to_string()),
                    ("serviceid", venue.to_string()),
                ],
            )
            .await?;
        let locked = self
            .get(
                "/web/product/findLockArea.html",
                &[
                    ("s_date", date.to_string()),
                    ("serviceid", venue.to_string()),
                ],
            )
            .await?;
        let mut slots = Vec::new();
        slots.extend(slots_of(&available, true));
        slots.extend(slots_of(&locked, false));
        Ok(json!({"venue": venue, "date": date, "slots": slots}))
    }

    pub async fn orders(&self) -> Result<Value, String> {
        let mut orders = Vec::new();
        for page in 1..=20 {
            let value = self
                .get(
                    "/web/yyuser/searchorder.html",
                    &[
                        ("page", page.to_string()),
                        ("rows", "20".into()),
                        ("status", String::new()),
                        ("iscomment", String::new()),
                        ("stockSDate", String::new()),
                        ("stockEDate", String::new()),
                    ],
                )
                .await?;
            let rows = value.as_array().ok_or("场馆订单格式已变化")?;
            orders.extend(rows.iter().filter_map(order));
            if rows.len() < 20 {
                break;
            }
        }
        orders.sort_by(|a, b| b["created"].as_str().cmp(&a["created"].as_str()));
        Ok(json!({"orders": orders}))
    }
}

fn venue(value: &Value) -> Option<Value> {
    let id = integer(value.get("id"));
    if id <= 0 {
        return None;
    }
    Some(json!({
        "id": id,
        "name": text(value.get("name")),
        "address": text(value.get("address")),
        "category": text(value.get("firstchart")),
        "advanceDays": integer(value.get("advanceday")),
        "advanceCount": integer(value.get("advancenum")),
    }))
}

fn slots_of(value: &Value, available: bool) -> Vec<Value> {
    value
        .get("object")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    let stock = item.get("stock");
                    let name = text(item.get("sname"));
                    if name.is_empty() {
                        return None;
                    }
                    Some(json!({
                        "area": name,
                        "stockId": integer(item.get("stockid")),
                        "time": stock.map(|s| text(s.get("time_no"))).unwrap_or_default(),
                        "price": stock.map(|s| number(s.get("price"))).unwrap_or(0.0),
                        "using": stock.map(|s| integer(s.get("using_num"))).unwrap_or(0),
                        "total": stock.map(|s| integer(s.get("all_count"))).unwrap_or(0),
                        "available": available && integer(item.get("status")) == 1,
                    }))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn order(value: &Value) -> Option<Value> {
    let id = text(value.get("orderid"));
    if id.is_empty() {
        return None;
    }
    let status = integer(value.get("status"));
    let details = value
        .get("orderdetail")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|detail| {
                    json!({
                        "date": text(detail.pointer("/stock/s_date")),
                        "time": text(detail.pointer("/stock/time_no")),
                        "area": text(detail.pointer("/stockdetail/sname")),
                        "venue": text(detail.pointer("/service/name")),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Some(json!({
        "id": id,
        "status": status,
        "statusText": STATUS.iter().find(|(code, _)| *code == status).map(|(_, text)| *text).unwrap_or("未知状态"),
        "created": text(value.get("createdate")),
        "price": number(value.get("price")),
        "details": details,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_venues_slots_and_orders() {
        let venue = venue(&json!({
            "id": "12", "name": "羽毛球馆", "address": "东南门",
            "firstchart": "G", "advanceday": "7", "advancenum": "2"
        }))
        .unwrap();
        assert_eq!(venue["id"], 12);
        assert_eq!(venue["advanceCount"], 2);

        let slots = slots_of(
            &json!({"object": [{
                "id": 1, "sname": "1 号场", "stockid": 9, "status": "1",
                "stock": {"time_no": "18:30-19:30", "price": "20", "using_num": 0, "all_count": 1}
            }]}),
            true,
        );
        assert_eq!(slots[0]["price"], 20.0);
        assert_eq!(slots[0]["available"], true);
        assert_eq!(slots_of(&json!({"object": [{"status": 1}]}), true).len(), 0);

        let order = order(&json!({
            "orderid": "88", "status": "1", "createdate": "2026-08-03 21:34:43",
            "price": "20", "orderdetail": [{
                "stock": {"s_date": "2026-08-05", "time_no": "18:30-19:30"},
                "stockdetail": {"sname": "1 号场"},
                "service": {"name": "羽毛球馆"}
            }]
        }))
        .unwrap();
        assert_eq!(order["statusText"], "预订成功");
        assert_eq!(order["details"][0]["venue"], "羽毛球馆");
    }

    #[test]
    fn html_notices_are_reported_as_errors() {
        assert_eq!(
            notice("<html><title>当前不在开放时间</title></html>".as_bytes()),
            Some("当前不在开放时间".into())
        );
        assert_eq!(notice(br#"{"object": []}"#), None);
        assert_eq!(
            notice(
                "<html><head><title>未到可预订时间</title></head>\
                 <body>请到8:40-21:40再来预订！</body></html>"
                    .as_bytes()
            ),
            Some("晚上服务不可用（预约开放 8:40–21:40）".into())
        );
    }

    #[test]
    fn login_pages_are_reported_as_expired_sessions() {
        assert_eq!(
            notice("<html><head><title>统一身份认证</title></head></html>".as_bytes()),
            Some("场馆登录已失效，请重新登录".into())
        );
        assert_eq!(
            notice(
                br#"<html><script src="http://202.117.17.144:8080/web/cas/oauth2url.html?code=x"></script></html>"#
            ),
            Some("场馆登录已失效，请重新登录".into())
        );
        assert_eq!(
            notice("<html><body>场馆系统暂时无法处理请求</body></html>".as_bytes()),
            Some("场馆系统返回了无法识别的页面".into())
        );
    }

    #[test]
    fn list_replies_accept_plain_and_wrapped_arrays() {
        let plain = json!([{"id": 1}, {"id": 2}]);
        let wrapped = json!({"object": [{"id": 1}], "total": 1});
        assert_eq!(rows_of(&plain).map(<[Value]>::len), Some(2));
        assert_eq!(rows_of(&wrapped).map(<[Value]>::len), Some(1));
        assert_eq!(rows_of(&json!({"code": 0})), None);
        assert!(list_shape_error("场馆列表", &json!({"code": 0})).contains("code"));
    }

    #[test]
    fn home_page_userno_proves_login() {
        assert!(index_logged_in(
            r#"<input id="userno" name="userno" type="hidden" value="3124000000" />"#
        ));
        assert!(!index_logged_in(
            r#"<input id="userno" name="userno" type="hidden" value="" />"#
        ));
        assert!(!index_logged_in("<html><title>场馆系统</title></html>"));
    }
}
