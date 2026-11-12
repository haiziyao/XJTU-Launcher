//! Personal profile from the portal's personal service.
//!
//! This is the mobile-portal identity card, not the 迎新 profile already
//! handled by `school/profile.rs`. The app authorizes it with a personal JWT;
//! with only the campus cookie this may answer 401 until a per-service session
//! is stored. Keep the raw JSON so the caller can decide which fields to show.
use super::Session;
use serde_json::Value;

pub(super) const PROFILE: &str =
    "https://authx-service.xjtu.edu.cn/personal/api/v1/personal/me/user";

pub(crate) struct Client<'a> {
    pub(super) session: &'a Session,
}

impl Client<'_> {
    pub(crate) async fn profile(&self) -> Result<Value, String> {
        self.session.get(PROFILE, &[]).await
    }
}
