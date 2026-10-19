//! Local account records in auth.json; encrypted files are read only for migration.
use crate::network::AccessMode;
use aes_gcm::{
    aead::{Aead, KeyInit, Payload},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine};
#[cfg(test)]
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::{fs, io::Cursor, path::Path};
use uuid::Uuid;
use zeroize::{Zeroize, Zeroizing};

const HEADER: &[u8] = b"XJTU-ACCOUNTS-1\0";
const KEY_NAME: &str = "campus-accounts-key-v1";
const MAX_FILE: u64 = 24 * 1024 * 1024;

#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct SavedPassword(pub String);
impl Drop for SavedPassword {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Account {
    pub id: String,
    pub username: String,
    pub degree: String,
    pub name: Option<String>,
    pub student_id: Option<String>,
    pub avatar: Option<String>,
    pub mode: AccessMode,
    pub token: String,
    pub cookies: String,
    #[serde(default)]
    pub password: Option<SavedPassword>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub username: String,
    pub degree: String,
    pub name: Option<String>,
    pub student_id: Option<String>,
    pub avatar: Option<String>,
    pub access_mode: AccessMode,
    pub password_saved: bool,
}
impl From<&Account> for Profile {
    fn from(a: &Account) -> Self {
        Self {
            id: a.id.clone(),
            username: a.username.clone(),
            degree: a.degree.clone(),
            name: a.name.clone(),
            student_id: a.student_id.clone(),
            avatar: a.avatar.clone(),
            access_mode: a.mode,
            password_saved: a.password.is_some(),
        }
    }
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Book {
    pub active_id: Option<String>,
    pub accounts: Vec<Account>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountList {
    pub active_id: Option<String>,
    pub accounts: Vec<Profile>,
}
impl Book {
    pub fn view(&self) -> AccountList {
        AccountList {
            active_id: self.active_id.clone(),
            accounts: self.accounts.iter().map(Profile::from).collect(),
        }
    }
    pub fn active(&self) -> Option<&Account> {
        self.accounts
            .iter()
            .find(|a| Some(&a.id) == self.active_id.as_ref())
    }
    pub fn select(&mut self, id: &str) -> Result<(), String> {
        if !self.accounts.iter().any(|a| a.id == id) {
            return Err("校园账号不存在".into());
        }
        self.active_id = Some(id.into());
        Ok(())
    }
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        if !self.accounts.iter().any(|a| a.id == id) {
            return Err("校园账号不存在".into());
        }
        self.accounts.retain(|a| a.id != id);
        if self.active_id.as_deref() == Some(id) {
            self.active_id = self.accounts.first().map(|a| a.id.clone());
        }
        Ok(())
    }
    pub fn upsert(&mut self, mut account: Account) -> Result<Profile, String> {
        let same = self.accounts.iter().position(|a| {
            a.degree == account.degree
                && match (&a.student_id, &account.student_id) {
                    (Some(a), Some(b)) => a == b,
                    _ => a.username == account.username,
                }
        });
        if let Some(i) = same {
            account.id = self.accounts[i].id.clone();
            account.avatar = self.accounts[i].avatar.clone();
            self.accounts[i] = account.clone();
        } else {
            if self.accounts.len() >= 32 {
                return Err("最多保存 32 个校园账号".into());
            }
            self.accounts.push(account.clone());
        }
        self.active_id = Some(account.id.clone());
        Ok(Profile::from(&account))
    }
    pub fn forget_password(&mut self, id: &str) -> Result<(), String> {
        let account = self
            .accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or("校园账号不存在")?;
        account.password = None;
        Ok(())
    }
    fn validate(&self) -> Result<(), String> {
        if self.accounts.len() > 32 || self.active_id.is_some() && self.active().is_none() {
            return Err("校园账号数据格式损坏".into());
        }
        let mut ids = std::collections::HashSet::new();
        for a in &self.accounts {
            if Uuid::parse_str(&a.id).is_err()
                || !ids.insert(&a.id)
                || a.username.is_empty()
                || a.username.len() > 256
                || crate::campus::domain(&a.degree).is_err()
                || a.password
                    .as_ref()
                    .is_some_and(|p| p.0.is_empty() || p.0.len() > 512)
                || a.token.is_empty()
                || a.cookies.len() > 2 * 1024 * 1024
            {
                return Err("校园账号数据格式损坏".into());
            }
        }
        Ok(())
    }
}
fn legacy_key() -> Result<Zeroizing<Vec<u8>>, String> {
    let value =
        crate::vault::legacy_get(KEY_NAME)?.ok_or("校园账号密钥不存在，已保留原加密文件")?;
    let bytes = Zeroizing::new(STANDARD.decode(value).map_err(|_| "校园账号密钥损坏")?);
    if bytes.len() != 32 {
        return Err("校园账号密钥损坏".into());
    }
    Ok(bytes)
}
#[cfg(test)]
pub(crate) fn seal(book: &Book, key: &[u8]) -> Result<Vec<u8>, String> {
    book.validate()?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "校园账号密钥无效")?;
    let plaintext = Zeroizing::new(serde_json::to_vec(book).map_err(|_| "无法编码校园账号")?);
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &plaintext,
                aad: HEADER,
            },
        )
        .map_err(|_| "无法加密校园账号")?;
    Ok([HEADER, &nonce, &ciphertext].concat())
}
fn unseal(bytes: &[u8], key: &[u8]) -> Result<Book, String> {
    if !bytes.starts_with(HEADER) || bytes.len() < HEADER.len() + 12 + 16 {
        return Err("校园账号文件格式不受支持，已保留原文件".into());
    }
    let cipher = Aes256Gcm::new_from_slice(key).map_err(|_| "校园账号密钥无效")?;
    let split = HEADER.len() + 12;
    let plain = Zeroizing::new(
        cipher
            .decrypt(
                Nonce::from_slice(&bytes[HEADER.len()..split]),
                Payload {
                    msg: &bytes[split..],
                    aad: HEADER,
                },
            )
            .map_err(|_| "校园账号解密失败，已保留原文件")?,
    );
    let book: Book =
        serde_json::from_slice(&plain).map_err(|_| "校园账号数据损坏，已保留原文件")?;
    book.validate()?;
    Ok(book)
}
pub fn load(legacy_path: &Path) -> Result<Book, String> {
    let book = crate::vault::read_or_import("campusAccounts", || {
        match fs::metadata(legacy_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err("无法读取旧校园账号文件".into()),
            Ok(info) if info.len() > MAX_FILE => return Err("校园账号文件过大".into()),
            Ok(_) => {}
        }
        unseal(
            &fs::read(legacy_path).map_err(|_| "无法读取旧校园账号文件")?,
            &legacy_key()?,
        )
        .map(Some)
    })?
    .unwrap_or_default();
    book.validate()?;
    Ok(book)
}
pub fn save(_legacy_path: &Path, book: &Book) -> Result<(), String> {
    book.validate()?;
    crate::vault::save("campusAccounts", book)
}
pub fn avatar(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|_| "无法读取头像")?;
    if !metadata.is_file() || metadata.len() > 5 * 1024 * 1024 {
        return Err("请选择不超过 5 MB 的 PNG、JPEG 或 WebP 图片".into());
    }
    let bytes = fs::read(path).map_err(|_| "无法读取头像")?;
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|_| "头像格式不受支持")?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP)
    ) {
        return Err("头像只支持 PNG、JPEG 或 WebP".into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    let img = reader
        .decode()
        .map_err(|_| "头像损坏或尺寸过大")?
        .resize_to_fill(256, 256, image::imageops::FilterType::Lanczos3);
    let mut output = Cursor::new(Vec::new());
    img.write_to(&mut output, image::ImageFormat::Png)
        .map_err(|_| "无法保存头像")?;
    Ok(format!(
        "data:image/png;base64,{}",
        STANDARD.encode(output.into_inner())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_password_is_encrypted_hidden_and_removable() {
        let mut book = Book::default();
        let mut account = record("saved-user");
        account.password = Some(SavedPassword("private-campus-password".into()));
        let id = book.upsert(account).unwrap().id;
        let encoded = seal(&book, &[4; 32]).unwrap();
        assert!(!String::from_utf8_lossy(&encoded).contains("private-campus-password"));
        let mut restored = unseal(&encoded, &[4; 32]).unwrap();
        assert_eq!(
            restored.active().unwrap().password.as_ref().unwrap().0,
            "private-campus-password"
        );
        let public = serde_json::to_string(&restored.view()).unwrap();
        assert!(public.contains("\"passwordSaved\":true"));
        assert!(!public.contains("private-campus-password"));
        restored.forget_password(&id).unwrap();
        assert!(!restored.view().accounts[0].password_saved);
        assert!(unseal(&seal(&restored, &[4; 32]).unwrap(), &[4; 32])
            .unwrap()
            .active()
            .unwrap()
            .password
            .is_none());
        assert!(restored.active().is_some());
    }
    #[test]
    fn old_accounts_load_without_password_and_relogin_can_disable_remembering() {
        let mut account = record("student");
        let mut old = serde_json::to_value(&account).unwrap();
        old.as_object_mut().unwrap().remove("password");
        assert!(serde_json::from_value::<Account>(old)
            .unwrap()
            .password
            .is_none());
        account.password = Some(SavedPassword("saved".into()));
        let mut book = Book::default();
        let id = book.upsert(account).unwrap().id;
        book.upsert(record("student")).unwrap();
        assert_eq!(book.active().unwrap().id, id);
        assert!(book.active().unwrap().password.is_none());
    }
    fn record(user: &str) -> Account {
        Account {
            id: Uuid::new_v4().to_string(),
            username: user.into(),
            degree: "undergraduate".into(),
            name: None,
            student_id: None,
            avatar: None,
            mode: AccessMode::Direct,
            token: format!("token-{user}"),
            cookies: "[]".into(),
            password: None,
        }
    }
    #[test]
    fn authenticated_encryption_detects_tampering_and_wrong_key() {
        let mut book = Book::default();
        book.upsert(record("student-a")).unwrap();
        let bytes = seal(&book, &[3; 32]).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("token-student-a"));
        assert_eq!(
            unseal(&bytes, &[3; 32]).unwrap().accounts[0].username,
            "student-a"
        );
        assert!(unseal(&bytes, &[4; 32]).is_err());
        let mut damaged = bytes;
        let end = damaged.len() - 1;
        damaged[end] ^= 1;
        assert!(unseal(&damaged, &[3; 32]).is_err());
    }
    #[test]
    fn switching_removal_and_reauthentication_keep_account_identity() {
        let mut book = Book::default();
        let a = book.upsert(record("a")).unwrap();
        book.accounts[0].avatar = Some("custom".into());
        let b = book.upsert(record("b")).unwrap();
        book.select(&a.id).unwrap();
        assert_eq!(book.active().unwrap().token, "token-a");
        let replaced = book.upsert(record("a")).unwrap();
        assert_eq!(replaced.id, a.id);
        assert_eq!(replaced.avatar.as_deref(), Some("custom"));
        assert_eq!(book.accounts.len(), 2);
        book.remove(&a.id).unwrap();
        assert_eq!(book.active().unwrap().id, b.id);
        let public = serde_json::to_string(&book.view()).unwrap();
        assert!(!public.contains("token-b"));
        assert!(!public.contains("cookies"));
    }
    #[test]
    fn avatar_rejects_svg_and_resizes_raster() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("avatar");
        fs::write(&path, b"<svg onload='alert(1)'/>").unwrap();
        assert!(avatar(&path).is_err());
        image::DynamicImage::new_rgb8(600, 400)
            .save_with_format(&path, image::ImageFormat::Png)
            .unwrap();
        let value = avatar(&path).unwrap();
        let bytes = STANDARD.decode(value.split_once(',').unwrap().1).unwrap();
        let image = image::load_from_memory(&bytes).unwrap();
        assert_eq!((image.width(), image.height()), (256, 256));
    }
}
