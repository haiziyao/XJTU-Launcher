//! Service directory from 办事大厅 (`ywtb`).
//!
//! The portal lists its apps and services through the ywtb proxy. Only the
//! read paths are covered; submitting an item still belongs to the ywtb
//! adapter and its own confirmation flow.
use super::Session;
use serde_json::Value;

pub(super) const MY_SERVICES: &str = "https://ywtb.xjtu.edu.cn/portal-api/api/proxy/service/api/rest/mobile/v2/selectMyServicesByTypeId";
const CATEGORIES: &str = "https://ywtb.xjtu.edu.cn/portal-api/v2/service/category/list";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    /// Services available to the account. The portal calls this with filters
    /// such as `keyWords` / `platformType`; the caller passes the query.
    pub(crate) async fn my_services(&self, query: &[(&str, &str)]) -> Result<Value, String> {
        self.session.get(MY_SERVICES, query).await
    }

    /// Service categories shown on the portal home page.
    pub(crate) async fn categories(&self) -> Result<Value, String> {
        self.session.get(CATEGORIES, &[]).await
    }
}
