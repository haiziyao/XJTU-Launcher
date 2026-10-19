//! Model profiles are encrypted together; public views never contain API keys.
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};
use zeroize::Zeroizing;

const HEADER: &[u8] = b"XJTU-MODELS-1\0";
const KEY: &str = "model-profiles-key-v1";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub protocol: String,
    pub model: String,
    pub api_key: String,
}
#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Book {
    pub default_id: Option<String>,
    pub providers: Vec<Provider>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicProvider {
    id: String,
    name: String,
    base_url: String,
    protocol: String,
    model: String,
    key_set: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    default_id: Option<String>,
    providers: Vec<PublicProvider>,
    legacy_key_available: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    pub id: Option<String>,
    pub name: String,
    pub base_url: String,
    pub protocol: String,
    pub model: String,
    // None preserves the stored key; an empty string explicitly removes it.
    pub api_key: Option<String>,
    pub use_legacy_key: bool,
}
fn valid_url(value: &str) -> bool {
    let Ok(url) = url::Url::parse(value) else {
        return false;
    };
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    url.host_str().is_some()
        && (url.scheme() == "https" || url.scheme() == "http" && local)
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}
impl Book {
    pub fn view(&self, legacy: bool) -> View {
        View {
            default_id: self.default_id.clone(),
            legacy_key_available: legacy,
            providers: self
                .providers
                .iter()
                .map(|p| PublicProvider {
                    id: p.id.clone(),
                    name: p.name.clone(),
                    base_url: p.base_url.clone(),
                    protocol: p.protocol.clone(),
                    model: p.model.clone(),
                    key_set: !p.api_key.is_empty(),
                })
                .collect(),
        }
    }
    fn validate(&self) -> Result<(), String> {
        let mut ids = std::collections::HashSet::new();
        if self.providers.len() > 32
            || self
                .default_id
                .as_ref()
                .is_some_and(|id| !self.providers.iter().any(|p| &p.id == id))
        {
            return Err("模型配置数量或默认项无效".into());
        }
        for p in &self.providers {
            if uuid::Uuid::parse_str(&p.id).is_err()
                || !ids.insert(&p.id)
                || p.name.trim().is_empty()
                || p.name.chars().count() > 60
                || p.model.trim().is_empty()
                || p.model.len() > 256
                || p.base_url.len() > 2048
                || !valid_url(&p.base_url)
                || !["openai-compatible", "anthropic", "gemini"].contains(&p.protocol.as_str())
                || p.api_key.len() > 8192
            {
                return Err("请填写厂商名称、模型和有效接口地址；仅本机地址支持 HTTP".into());
            }
        }
        Ok(())
    }
    pub fn upsert(&mut self, input: Input, legacy: Option<String>) -> Result<(), String> {
        let existing = input
            .id
            .as_ref()
            .map(|id| {
                self.providers
                    .iter()
                    .position(|p| &p.id == id)
                    .ok_or("模型配置不存在")
            })
            .transpose()?;
        let api_key = if input.use_legacy_key {
            if input.api_key.as_ref().is_some_and(|k| !k.trim().is_empty()) {
                return Err("请只选择一种密钥来源".into());
            }
            legacy.ok_or("没有可导入的旧密钥")?
        } else {
            input
                .api_key
                .map(|k| k.trim().to_owned())
                .unwrap_or_else(|| {
                    existing
                        .map(|i| self.providers[i].api_key.clone())
                        .unwrap_or_default()
                })
        };
        let provider = Provider {
            id: input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            name: input.name.trim().into(),
            base_url: input.base_url.trim().trim_end_matches('/').into(),
            protocol: input.protocol,
            model: input.model.trim().into(),
            api_key,
        };
        // Validate a candidate before mutating the in-memory book.
        let mut next = Book {
            default_id: self.default_id.clone(),
            providers: self.providers.clone(),
        };
        if next.default_id.is_none() {
            next.default_id = Some(provider.id.clone());
        }
        if let Some(i) = existing {
            next.providers[i] = provider;
        } else {
            next.providers.push(provider);
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if !self.providers.iter().any(|p| p.id == id) {
            return Err("模型配置不存在".into());
        }
        self.providers.retain(|p| p.id != id);
        if self.default_id.as_deref() == Some(id) {
            self.default_id = self.providers.first().map(|p| p.id.clone());
        }
        Ok(())
    }
    pub fn select(&mut self, id: &str) -> Result<(), String> {
        if !self.providers.iter().any(|p| p.id == id) {
            return Err("模型配置不存在".into());
        }
        self.default_id = Some(id.into());
        Ok(())
    }
}
fn key(create: bool) -> Result<Zeroizing<Vec<u8>>, String> {
    if let Some(s) = crate::vault::get(KEY)? {
        let bytes = Zeroizing::new(STANDARD.decode(s).map_err(|_| "模型存储密钥损坏")?);
        if bytes.len() != 32 {
            return Err("模型存储密钥损坏".into());
        }
        return Ok(bytes);
    }
    if !create {
        return Err("模型存储密钥缺失，原文件已保留".into());
    }
    let mut bytes = Zeroizing::new(vec![0; 32]);
    OsRng.fill_bytes(&mut bytes);
    crate::vault::set(KEY, &STANDARD.encode(&bytes))?;
    Ok(bytes)
}
fn seal(book: &Book, key: &[u8]) -> Result<Vec<u8>, String> {
    book.validate()?;
    let plain = Zeroizing::new(serde_json::to_vec(book).map_err(|_| "无法编码模型配置")?);
    let mut nonce = [0; 12];
    OsRng.fill_bytes(&mut nonce);
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "模型存储密钥无效")?;
    let encrypted = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &plain,
                aad: HEADER,
            },
        )
        .map_err(|_| "无法加密模型配置")?;
    Ok([HEADER, &nonce, &encrypted].concat())
}
fn unseal(bytes: &[u8], key: &[u8]) -> Result<Book, String> {
    let split = HEADER.len() + 12;
    if !bytes.starts_with(HEADER) || bytes.len() < split + 16 {
        return Err("模型文件损坏，原文件已保留".into());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "模型存储密钥无效")?;
    let plain = Zeroizing::new(
        cipher
            .decrypt(
                Nonce::from_slice(&bytes[HEADER.len()..split]),
                Payload {
                    msg: &bytes[split..],
                    aad: HEADER,
                },
            )
            .map_err(|_| "模型配置解密失败，原文件已保留")?,
    );
    let book: Book = serde_json::from_slice(&plain).map_err(|_| "模型配置格式损坏")?;
    book.validate()?;
    Ok(book)
}
pub fn load(path: &Path) -> Result<Book, String> {
    if !path.exists() {
        return Ok(Book::default());
    }
    if fs::metadata(path).map_err(|_| "无法读取模型文件")?.len() > 1024 * 1024 {
        return Err("模型配置文件过大".into());
    }
    unseal(
        &fs::read(path).map_err(|_| "无法读取模型配置")?,
        &key(false)?,
    )
}
pub fn save(path: &Path, book: &Book) -> Result<(), String> {
    crate::config::write_atomic(path, &seal(book, &key(!path.exists())?)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn input() -> Input {
        Input {
            id: None,
            name: "Custom".into(),
            base_url: "https://example.com/v1".into(),
            protocol: "openai-compatible".into(),
            model: "my-model".into(),
            api_key: Some("private-test-key".into()),
            use_legacy_key: false,
        }
    }
    #[test]
    fn encryption_and_public_view_do_not_expose_keys() {
        let mut b = Book::default();
        b.upsert(input(), None).unwrap();
        assert!(!serde_json::to_string(&b.view(false))
            .unwrap()
            .contains("private-test-key"));
        let bytes = seal(&b, &[7; 32]).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("private-test-key"));
        assert_eq!(
            unseal(&bytes, &[7; 32]).unwrap().providers[0].api_key,
            "private-test-key"
        );
        assert!(unseal(&bytes, &[8; 32]).is_err());
        let mut damaged = bytes;
        let last = damaged.len() - 1;
        damaged[last] ^= 1;
        assert!(unseal(&damaged, &[7; 32]).is_err());
    }
    #[test]
    fn editing_preserves_or_explicitly_clears_key_and_removal_repairs_default() {
        let mut b = Book::default();
        b.upsert(input(), None).unwrap();
        let id = b.providers[0].id.clone();
        let mut edit = input();
        edit.id = Some(id.clone());
        edit.api_key = None;
        b.upsert(edit, None).unwrap();
        assert_eq!(b.providers[0].api_key, "private-test-key");
        let mut edit = input();
        edit.id = Some(id.clone());
        edit.api_key = Some(String::new());
        b.upsert(edit, None).unwrap();
        assert!(b.providers[0].api_key.is_empty());
        b.upsert(input(), None).unwrap();
        b.remove(&id).unwrap();
        assert_eq!(b.default_id.as_ref(), Some(&b.providers[0].id));
        b.remove(&b.providers[0].id.clone()).unwrap();
        assert!(b.default_id.is_none());
    }
    #[test]
    fn validates_endpoints_and_imports_legacy_only_when_requested() {
        assert!(valid_url("http://127.0.0.1:11434/v1"));
        for url in [
            "http://example.com/v1",
            "https://user:secret@example.com",
            "https://example.com?key=secret",
            "file:///tmp/test",
        ] {
            assert!(!valid_url(url));
        }
        let mut b = Book::default();
        let mut i = input();
        i.use_legacy_key = true;
        i.api_key = None;
        b.upsert(i, Some("legacy-secret".into())).unwrap();
        assert_eq!(b.providers[0].api_key, "legacy-secret");
        let mut bad = input();
        bad.id = Some(uuid::Uuid::new_v4().to_string());
        assert!(b.upsert(bad, None).is_err());
    }
}
