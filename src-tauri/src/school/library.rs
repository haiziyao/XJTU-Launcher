//! Library seat adapter. The seat system is plain HTTP on the campus network
//! and keeps its own CAS cookie session under `sites.library`.
use crate::{
    accounts::Account,
    network::{AccessMode, Transport},
};
use serde_json::{json, Value};
use url::Url;

const BASE: &str = "http://rg.lib.xjtu.edu.cn:8086";
pub(crate) const LOGIN_URL: &str = "http://rg.lib.xjtu.edu.cn:8086/seat/";

/// Floor codes are fixed by the school system; the UI picks from this list.
pub fn floors() -> Value {
    json!([
        {"campus": "兴庆", "code": "xingqing2floor", "name": "2 层"},
        {"campus": "兴庆", "code": "xingqing3floor", "name": "3 层"},
        {"campus": "兴庆", "code": "xingqing4floor", "name": "4 层"},
        {"campus": "创新港", "code": "inno1floor", "name": "1 层"},
        {"campus": "创新港", "code": "inno2floor", "name": "2 层"},
        {"campus": "雁塔", "code": "yanta1floor", "name": "1 层"},
        {"campus": "雁塔", "code": "yanta2floor", "name": "2 层"},
        {"campus": "雁塔", "code": "yanta3floor", "name": "3 层"},
        {"campus": "雁塔", "code": "yanta4floor", "name": "4 层"},
    ])
}

pub fn valid_floor(code: &str) -> bool {
    floors()
        .as_array()
        .is_some_and(|list| list.iter().any(|item| item["code"] == code))
}

pub fn valid_area(code: &str) -> bool {
    !code.is_empty()
        && code.len() <= 64
        && code
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

pub struct Client {
    transport: Transport,
}

impl Client {
    pub fn connect(account: &Account) -> Result<Self, String> {
        if account.mode == AccessMode::Webvpn {
            return Err("图书馆座位仅支持校园网直连，请连接校园网后重试".into());
        }
        let site = account.sites.get("library").ok_or("请先完成图书馆登录")?;
        let cookies = site
            .cookies
            .as_deref()
            .filter(|cookies| !cookies.is_empty())
            .ok_or("请先完成图书馆登录")?;
        Ok(Self {
            transport: Transport::new(Some(cookies))?,
        })
    }

    async fn get(&self, path: &str, params: &[(&str, &str)]) -> Result<Value, String> {
        if !path.starts_with('/') || path.contains("..") {
            return Err("图书馆接口路径无效".into());
        }
        let url = Url::parse_with_params(&format!("{BASE}{path}"), params)
            .map_err(|_| "图书馆接口地址无效")?;
        let address = Transport::address(&url, AccessMode::Direct)?;
        let response = self
            .transport
            .client
            .get(address)
            .header("X-Requested-With", "XMLHttpRequest")
            .header("Accept", "application/json, text/javascript, */*; q=0.01")
            .send()
            .await
            .map_err(|_| "无法连接图书馆座位系统，请检查校园网后重试")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err("图书馆登录已失效，请刷新后重试".into());
        }
        if !status.is_success() {
            return Err(format!("图书馆服务暂不可用（HTTP {}）", status.as_u16()));
        }
        response
            .json()
            .await
            .map_err(|_| "图书馆登录已失效，请刷新后重试".into())
    }

    pub async fn areas(&self, floor: &str) -> Result<Value, String> {
        if !valid_floor(floor) {
            return Err("请选择有效的图书馆楼层".into());
        }
        let value = self
            .get("/qspace", &[("lang", "zh"), ("floor", floor)])
            .await?;
        let areas = value
            .get("sp")
            .and_then(Value::as_object)
            .map(|areas| {
                areas
                    .iter()
                    .filter(|(code, name)| {
                        !code.is_empty() && name.as_str().is_some_and(|name| !name.is_empty())
                    })
                    .map(|(code, name)| {
                        json!({"code": code, "name": name.as_str().unwrap_or_default()})
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(json!({"floor": floor, "areas": areas}))
    }

    pub async fn seats(&self, area: &str) -> Result<Value, String> {
        if !valid_area(area) {
            return Err("请选择有效的图书馆区域".into());
        }
        let value = self.get("/qseat", &[("sp", area)]).await?;
        let mut seats = value
            .get("seat")
            .and_then(Value::as_object)
            .map(|seats| {
                seats
                    .iter()
                    .filter_map(|(id, status)| {
                        let status = status.as_i64()?;
                        Some((id.clone(), status == 0))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        seats.sort_by(|a, b| a.0.cmp(&b.0));
        let stats = value
            .get("scount")
            .and_then(Value::as_object)
            .map(|counts| {
                counts
                    .iter()
                    .filter_map(|(code, value)| {
                        let list = value.as_array()?;
                        let total = list.first()?.as_i64()?;
                        let free = list.get(1)?.as_i64()?;
                        Some(json!({"code": code, "total": total, "free": free}))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        Ok(json!({
            "area": area,
            "seats": seats.iter().map(|(id, free)| json!({"id": id, "free": free})).collect::<Vec<_>>(),
            "stats": stats,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn floors_and_area_codes_are_bounded() {
        assert!(valid_floor("xingqing2floor"));
        assert!(!valid_floor("xingqing9floor"));
        assert!(valid_area("A101"));
        assert!(!valid_area(""));
        assert!(!valid_area("A 101"));
        assert!(!valid_area("区域/../x"));
    }
}
