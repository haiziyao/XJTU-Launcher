use keyring::{Entry, Error};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "cn.haiziyao.xjtu-launcher";

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub degree: String,
    pub token: String,
    #[serde(default)]
    pub mode: crate::network::AccessMode,
    #[serde(default)]
    pub cookies: Option<String>,
}

fn entry(name: &str) -> Result<Entry, String> {
    Entry::new(SERVICE, name).map_err(|_| "无法访问系统凭据库".to_string())
}

pub(crate) fn get(name: &str) -> Result<Option<String>, String> {
    match entry(name)?.get_password() {
        Ok(value) => Ok(Some(value)),
        Err(Error::NoEntry) => Ok(None),
        Err(_) => Err("无法读取系统凭据库，请检查系统授权".into()),
    }
}

pub(crate) fn set(name: &str, value: &str) -> Result<(), String> {
    entry(name)?
        .set_password(value)
        .map_err(|_| "无法写入系统凭据库".into())
}

pub fn delete(name: &str) -> Result<(), String> {
    match entry(name)?.delete_credential() {
        Ok(()) | Err(Error::NoEntry) => Ok(()),
        Err(_) => Err("无法删除系统凭据".into()),
    }
}

pub fn session() -> Result<Option<Session>, String> {
    get("campus-session")?
        .map(|text| {
            serde_json::from_str(&text)
                .map_err(|_| "校园会话格式损坏，请退出校园账户后重新登录".into())
        })
        .transpose()
}

pub fn api_key_set() -> Result<bool, String> {
    Ok(get("model-api-key")?.is_some_and(|key| !key.is_empty()))
}

pub fn save_api_key(key: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return delete("model-api-key");
    }
    if key.len() > 8192 {
        return Err("API Key 长度超过限制".into());
    }
    entry("model-api-key")?
        .set_password(key.trim())
        .map_err(|_| "无法将 API Key 存入系统凭据库".into())
}
