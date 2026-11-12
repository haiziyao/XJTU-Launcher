//! School-bus reservations (`xjbus.xjtu.edu.cn`).
//!
//! Only the "my reservations" page URL is known. Paging fields are passed
//! through from the caller; booking a seat is not implemented until a real
//! request confirms the endpoint and payload.
use super::Session;
use serde_json::Value;

pub(super) const PAGE: &str = "https://xjbus.xjtu.edu.cn/api/school/bus/user/pageReservationUsers";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// One page of the account's bus reservations.
    pub(crate) async fn page(&self, query: &[(&str, &str)]) -> Result<Value, String> {
        self.session.get(PAGE, query).await
    }
}
