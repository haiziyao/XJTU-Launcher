//! Campus run (乐跑) data from the v3 API.
//!
//! The H5 app reaches these pages with `uid`, `school_id`, `token` and `sign`
//! in the URL. With a CAS cookie session those query values should not be
//! needed, but the envelope is still unverified; helpers therefore return the
//! decoded body without reshaping it. Verify with one real term before wiring
//! the UI.
use super::Session;
use serde_json::Value;

const RULE: &str = "v3/index.php/Rule/running";
const TERM: &str = "v3/index.php/RunningData/termData";
const RECENT: &str = "v3/index.php/RunningData/show7dataNew";
const TERM_DETAIL: &str = "v3/index.php/Run/getTermDetail";
const APPLY_FREE: &str = "v3/index.php/Running/applyFreeRun";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Run rule for the current term.
    pub(crate) async fn rule(&self) -> Result<Value, String> {
        self.session.post_v3(RULE, &[]).await
    }

    /// This term's totals and requirements.
    pub(crate) async fn term(&self) -> Result<Value, String> {
        self.session.post_v3(TERM, &[]).await
    }

    /// Runs recorded in the last seven days.
    pub(crate) async fn recent(&self) -> Result<Value, String> {
        self.session.post_v3(RECENT, &[]).await
    }

    /// Per-run detail for the term.
    pub(crate) async fn term_detail(&self) -> Result<Value, String> {
        self.session.post_v3(TERM_DETAIL, &[]).await
    }

    /// Write: exempt-run application. Preview the request and obtain explicit
    /// confirmation first; never retry automatically.
    pub(crate) async fn apply_free(&self) -> Result<Value, String> {
        self.session.post_v3(APPLY_FREE, &[]).await
    }
}
