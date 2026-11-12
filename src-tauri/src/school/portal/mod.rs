//! Services behind 移动交通大学 (`superapp.xjtu.edu.cn`).
//!
//! The app is a shell; its data lives on separate hosts, each with its own CAS
//! session. The paths below come from the app's public front-end bundle and
//! have not yet been verified with a live account:
//!
//! | module        | host                          | data                       |
//! | ------------- | ----------------------------- | -------------------------- |
//! | `todo`        | `transaction.xjtu.edu.cn`     | 待办 / 应用 / 其他数量     |
//! | `message`     | `message-service.xjtu.edu.cn` | 日程、消息、未读数         |
//! | `me`          | `authx-service.xjtu.edu.cn`   | 个人资料                   |
//! | `service`     | `ywtb.xjtu.edu.cn`            | 服务目录                   |
//! | `reservation` | `reservation.xjtu.edu.cn`     | 我的场馆 / 会议室预约      |
//! | `bus`         | `xjbus.xjtu.edu.cn`           | 校车预约                   |
//!
//! The services return different envelopes, so helpers return the decoded JSON
//! unchanged. All are read-only; writing (submitting a form, booking a bus
//! seat) still needs the preview → confirm protocol once an endpoint is
//! verified.
pub(crate) mod bus;
pub(crate) mod me;
pub(crate) mod message;
pub(crate) mod reservation;
pub(crate) mod service;
pub(crate) mod todo;

use crate::accounts::Account;
use crate::network::{limited_bytes, trusted};
use reqwest::Client as Http;
use serde_json::Value;
use url::Url;

pub(super) const IDLE: &str = "移动交通大学登录态已失效，请重新登录校园账号";

pub(crate) struct Session {
    http: Http,
}

impl Session {
    pub(crate) fn connect(account: &Account) -> Result<Self, String> {
        Ok(Self {
            http: super::direct::account(account)?,
        })
    }

    pub(crate) fn todo(&self) -> todo::Client<'_> {
        todo::Client { session: self }
    }

    pub(crate) fn message(&self) -> message::Client<'_> {
        message::Client { session: self }
    }

    pub(crate) fn me(&self) -> me::Client<'_> {
        me::Client { session: self }
    }

    pub(crate) fn service(&self) -> service::Client<'_> {
        service::Client { session: self }
    }

    pub(crate) fn reservation(&self) -> reservation::Client<'_> {
        reservation::Client { session: self }
    }

    pub(crate) fn bus(&self) -> bus::Client<'_> {
        bus::Client { session: self }
    }

    /// GET a campus URL. Query values are percent-encoded by `Url`.
    async fn get(&self, url: &str, query: &[(&str, &str)]) -> Result<Value, String> {
        let mut target = Url::parse(url).map_err(|_| "门户接口地址无效")?;
        if !trusted(&target) {
            return Err("门户接口地址不受支持".into());
        }
        if !query.is_empty() {
            target.query_pairs_mut().extend_pairs(query);
        }
        let response = self
            .http
            .get(target)
            .send()
            .await
            .map_err(|_| "无法连接门户服务，请检查网络后重试")?;
        let status = response.status();
        if status.is_redirection() || matches!(status.as_u16(), 401 | 403) {
            return Err(IDLE.into());
        }
        if !status.is_success() {
            return Err(format!("门户服务返回 HTTP {}", status.as_u16()));
        }
        let bytes = limited_bytes(response, 2 * 1024 * 1024).await?;
        if bytes.iter().all(u8::is_ascii_whitespace) {
            return Ok(Value::Null);
        }
        // A login page is HTML, which means the saved session did not reach
        // this service even though the request itself succeeded.
        if bytes.iter().find(|byte| !byte.is_ascii_whitespace()) == Some(&b'<') {
            return Err(IDLE.into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "门户返回了非预期数据".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_service_host_is_trusted() {
        let urls = [
            todo::TODO,
            message::CALENDAR,
            me::PROFILE,
            service::MY_SERVICES,
            reservation::MINE,
            bus::PAGE,
        ];
        for url in urls {
            let parsed = Url::parse(url).expect(url);
            assert!(trusted(&parsed), "{url} must be in the trusted list");
        }
    }
}
