//! Pending-item counters from the transaction service.
//!
//! These feed the superapp tab badges. The payload is small and the exact field
//! names are not verified yet, so the raw JSON is returned.
use super::Session;
use serde_json::Value;

pub(super) const TODO: &str =
    "https://transaction.xjtu.edu.cn/ttc/api/ttc/center-list/getToDoListCountNew";
pub(super) const APPS: &str =
    "https://transaction.xjtu.edu.cn/ttc/api/ttc/center-list/getAppsCount";
pub(super) const OTHER: &str =
    "https://transaction.xjtu.edu.cn/ttc/api/ttc/center-list/getOtherCount";
// The app's base URL already ends with `/ttc/`; the doubled segment is what it
// actually sends after collapsing duplicate slashes.
const OVERDUE: &str =
    "https://transaction.xjtu.edu.cn/ttc/ttc/person-setting/getPersonOverdueSetting";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Pending to-do count.
    pub(crate) async fn count(&self) -> Result<Value, String> {
        self.session.get(TODO, &[]).await
    }

    /// Per-application pending count.
    pub(crate) async fn apps(&self) -> Result<Value, String> {
        self.session.get(APPS, &[]).await
    }

    /// Counts for other message categories.
    pub(crate) async fn other(&self) -> Result<Value, String> {
        self.session.get(OTHER, &[]).await
    }

    /// Whether the account enabled overdue reminders.
    pub(crate) async fn overdue_setting(&self) -> Result<Value, String> {
        self.session.get(OVERDUE, &[]).await
    }
}
