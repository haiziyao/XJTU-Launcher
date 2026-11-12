//! Campus sports matches and sign-up.
//!
//! The platform calls these `match`; the module avoids that keyword. Joining
//! and cancelling are writes and share `match/join` (`type` 1 joins, 2 cancels).
use super::{data, Session};
use serde_json::Value;

const LIST: &str = "bdlp_h5_fitness_test/public/index.php/index/match/match_list";
const MINE: &str = "bdlp_h5_fitness_test/public/index.php/index/match/mines";
const DETAIL: &str = "bdlp_h5_fitness_test/public/index.php/index/match/detail";
const JOIN: &str = "bdlp_h5_fitness_test/public/index.php/index/match/join";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Match list, one page at a time.
    pub(crate) async fn list(&self, page: u32) -> Result<Value, String> {
        let page = page.to_string();
        let value = self.session.post(LIST, &[("page", &page)]).await?;
        Ok(data(&value))
    }

    /// Matches the account has signed up for.
    pub(crate) async fn mine(&self, page: u32) -> Result<Value, String> {
        let page = page.to_string();
        let value = self.session.post(MINE, &[("page", &page)]).await?;
        Ok(data(&value))
    }

    pub(crate) async fn detail(&self, match_id: &str) -> Result<Value, String> {
        let value = self.session.post(DETAIL, &[("match_id", match_id)]).await?;
        Ok(data(&value))
    }

    /// Write: sign up for a match. Preview and confirm first.
    pub(crate) async fn join(&self, match_id: &str) -> Result<Value, String> {
        let value = self
            .session
            .post(JOIN, &[("match_id", match_id), ("type", "1")])
            .await?;
        Ok(data(&value))
    }

    /// Write: cancel a sign-up. Do not retry when the result is unknown.
    pub(crate) async fn cancel(&self, match_id: &str) -> Result<Value, String> {
        let value = self
            .session
            .post(JOIN, &[("match_id", match_id), ("type", "2")])
            .await?;
        Ok(data(&value))
    }
}
