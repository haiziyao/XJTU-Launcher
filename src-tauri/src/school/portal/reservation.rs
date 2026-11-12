//! Venue and meeting-room reservations (`reservation.xjtu.edu.cn`).
//!
//! The portal opens this URL in a webview. Its query parameters have not been
//! verified, so `mine` forwards whatever the caller supplies. Booking and
//! cancelling live on the same host but have no confirmed endpoint yet; add
//! them only after observing a real request, and behind preview → confirm.
use super::Session;
use serde_json::Value;

pub(super) const MINE: &str = "https://reservation.xjtu.edu.cn/api/myreservation/my";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// The account's reservations.
    pub(crate) async fn mine(&self, query: &[(&str, &str)]) -> Result<Value, String> {
        self.session.get(MINE, query).await
    }
}
