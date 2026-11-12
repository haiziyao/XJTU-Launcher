//! Calendar and message records from the message service.
//!
//! The app sends `X-Id-Token` alongside the CAS cookie. A cookie session may
//! be enough for the read endpoints, but verify before relying on it; if the
//! service answers 401/403 the adapter reports an expired login and the caller
//! should not retry.
use super::Session;
use serde_json::Value;

pub(super) const CALENDAR: &str =
    "https://message-service.xjtu.edu.cn/center/api/v1/messageCenter/getPersonlCalendarList";
const RECORDS: &str =
    "https://message-service.xjtu.edu.cn/center/api/v1/messageCenter/listMessageRecord";
const UNREAD: &str =
    "https://message-service.xjtu.edu.cn/center/api/v1/messageCenter/getUnReadedCount";
const DETAIL: &str =
    "https://message-service.xjtu.edu.cn/center/api/v1/messageCenter/getMessageRecord";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Personal schedule entries.
    pub(crate) async fn calendar(&self) -> Result<Value, String> {
        self.session.get(CALENDAR, &[]).await
    }

    /// Message records. Paging and filters have not been verified, so the
    /// caller passes the query it wants to try.
    pub(crate) async fn records(&self, query: &[(&str, &str)]) -> Result<Value, String> {
        self.session.get(RECORDS, query).await
    }

    /// Unread message count.
    pub(crate) async fn unread(&self) -> Result<Value, String> {
        self.session.get(UNREAD, &[]).await
    }

    /// One message. The identifier field name is unverified; pass it as query.
    pub(crate) async fn detail(&self, query: &[(&str, &str)]) -> Result<Value, String> {
        self.session.get(DETAIL, query).await
    }
}
