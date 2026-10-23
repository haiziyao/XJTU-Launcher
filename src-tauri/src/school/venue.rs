//! Sports venue adapter (read-only). The booking site is plain HTTP on the
//! campus network and keeps its own OAuth CAS session under `sites.venue`.
use crate::{
    accounts::Account,
    network::{AccessMode, Transport},
};
use serde_json::{json, Value};
use url::Url;

const BASE: &str = "http://202.117.17.144:8080";
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

/// The site answers notices (for example "not in booking hours") with an HTML
/// page, so JSON parsing alone cannot tell success from failure.
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
    Some(title.unwrap_or_else(|| "场馆系统暂时无法处理请求".into()))
}

pub struct Client {
    transport: Transport,
}

impl Client {
    pub fn connect(account: &Account) -> Result<Self, String> {
        if account.mode == AccessMode::Webvpn {
            return Err("场馆预约仅支持校园网直连，请连接校园网后重试".into());
        }
        let site = account.sites.get("venue").ok_or("请先完成场馆系统登录")?;
        let cookies = site
            .cookies
            .as_deref()
            .filter(|cookies| !cookies.is_empty())
            .ok_or("请先完成场馆系统登录")?;
        Ok(Self {
            transport: Transport::new(Some(cookies))?,
        })
    }

    async fn get(&self, path: &str, params: &[(&str, String)]) -> Result<Value, String> {
        if !path.starts_with("/web/") || path.contains("..") {
            return Err("场馆接口路径无效".into());
        }
        let url = Url::parse_with_params(&format!("{BASE}{path}"), params)
            .map_err(|_| "场馆接口地址无效")?;
        let address = Transport::address(&url, AccessMode::Direct)?;
        let response = self
            .transport
            .client
            .get(address)
            .header("Referer", format!("{BASE}/web/index.html"))
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .header("X-Requested-With", "XMLHttpRequest")
            .send()
            .await
            .map_err(|_| "无法连接场馆系统，请检查校园网后重试")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err("场馆登录已失效，请刷新后重试".into());
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
            let rows = value.as_array().ok_or("场馆列表格式已变化")?;
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
    }
}
