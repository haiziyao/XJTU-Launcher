//! Application-owned credentials in ~/.xjtu-launcher/auth.json.
//! The OS keychain is read only when importing an older installation.
use keyring::{Entry, Error};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
};
use zeroize::Zeroizing;

const SERVICE: &str = "cn.haiziyao.xjtu-launcher";
const MAX_FILE: u64 = 32 * 1024 * 1024;
static STORE: OnceLock<Store> = OnceLock::new();
type LegacyResult = Result<Option<String>, String>;
static LEGACY_CACHE: Mutex<BTreeMap<String, LegacyResult>> = Mutex::new(BTreeMap::new());

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

#[derive(Serialize, Deserialize)]
struct Document {
    version: u8,
    #[serde(flatten)]
    sections: BTreeMap<String, Value>,
}
impl Default for Document {
    fn default() -> Self {
        Self {
            version: 1,
            sections: BTreeMap::new(),
        }
    }
}

struct Store {
    path: PathBuf,
    lock: Mutex<()>,
}
impl Store {
    fn new(directory: &Path) -> Result<Self, String> {
        fs::create_dir_all(directory).map_err(|_| "无法创建 .xjtu-launcher 目录")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o700))
                .map_err(|_| "无法设置本地凭据目录权限")?;
        }
        Ok(Self {
            path: directory.join("auth.json"),
            lock: Mutex::new(()),
        })
    }
    fn read(&self) -> Result<Document, String> {
        match fs::metadata(&self.path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Document::default()),
            Err(_) => return Err("无法读取 .xjtu-launcher/auth.json".into()),
            Ok(info) if !info.is_file() || info.len() > MAX_FILE => {
                return Err("auth.json 文件大小或类型无效，原文件已保留".into())
            }
            Ok(_) => {}
        }
        let bytes = Zeroizing::new(fs::read(&self.path).map_err(|_| "无法读取本地凭据")?);
        let doc: Document =
            serde_json::from_slice(&bytes).map_err(|_| "auth.json 格式损坏，原文件已保留")?;
        if doc.version != 1 {
            return Err("auth.json 版本不受支持，原文件已保留".into());
        }
        Ok(doc)
    }
    fn write(&self, doc: &Document) -> Result<(), String> {
        let bytes = Zeroizing::new(serde_json::to_vec_pretty(doc).map_err(|_| "无法编码本地凭据")?);
        if bytes.len() as u64 > MAX_FILE {
            return Err("本地凭据文件超过大小限制".into());
        }
        // write_atomic uses a private temporary file and replaces only after sync succeeds.
        crate::config::write_atomic(&self.path, &bytes)
    }
    fn read_or_import<T: DeserializeOwned + Serialize>(
        &self,
        section: &str,
        import: impl FnOnce() -> Result<Option<T>, String>,
    ) -> Result<Option<T>, String> {
        let _guard = self.lock.lock().map_err(|_| "本地凭据正在使用")?;
        let mut doc = self.read()?;
        if let Some(value) = doc.sections.get(section) {
            return serde_json::from_value(value.clone())
                .map(Some)
                .map_err(|_| "本地凭据字段无效，原文件已保留".into());
        }
        let imported = import()?;
        if let Some(value) = imported.as_ref() {
            doc.sections.insert(
                section.into(),
                serde_json::to_value(value).map_err(|_| "无法编码导入的凭据")?,
            );
            self.write(&doc)?;
        }
        Ok(imported)
    }
    fn save<T: Serialize>(&self, section: &str, value: &T) -> Result<(), String> {
        let _guard = self.lock.lock().map_err(|_| "本地凭据正在使用")?;
        let mut doc = self.read()?;
        doc.sections.insert(
            section.into(),
            serde_json::to_value(value).map_err(|_| "无法编码本地凭据")?,
        );
        self.write(&doc)
    }
    fn contains(&self, section: &str) -> Result<bool, String> {
        let _guard = self.lock.lock().map_err(|_| "本地凭据正在使用")?;
        Ok(self.read()?.sections.contains_key(section))
    }
    fn legacy(&self, name: &str, import: impl FnOnce() -> LegacyResult) -> LegacyResult {
        // Persist None as a tombstone so absent/deleted legacy items are never queried again.
        self.read_or_import::<Option<String>>(&format!("legacy:{name}"), || import().map(Some))
            .map(Option::flatten)
    }
}

pub fn initialize(directory: PathBuf) -> Result<(), String> {
    let store = Store::new(&directory)?;
    STORE
        .set(store)
        .map_err(|_| "本地凭据存储已初始化".to_string())
}
fn store() -> Result<&'static Store, String> {
    STORE.get().ok_or_else(|| "本地凭据存储尚未初始化".into())
}

pub(crate) fn read_or_import<T: DeserializeOwned + Serialize>(
    section: &str,
    import: impl FnOnce() -> Result<Option<T>, String>,
) -> Result<Option<T>, String> {
    store()?.read_or_import(section, import)
}
pub(crate) fn save<T: Serialize>(section: &str, value: &T) -> Result<(), String> {
    store()?.save(section, value)
}

/// Compatibility path only. Never writes/deletes OS entries or exposes keys to the UI.
pub(crate) fn legacy_get(name: &str) -> LegacyResult {
    if ![
        "campus-accounts-key-v1",
        "model-profiles-key-v1",
        "campus-session",
        "model-api-key",
    ]
    .contains(&name)
    {
        return Err("不支持的旧凭据类型".into());
    }
    let mut cache = LEGACY_CACHE.lock().map_err(|_| "旧凭据正在导入")?;
    if let Some(result) = cache.get(name) {
        return result.clone();
    }
    let result = match Entry::new(SERVICE, name).and_then(|entry| entry.get_password()) {
        Ok(value) => Ok(Some(value)),
        Err(Error::NoEntry) => Ok(None),
        Err(_) => Err("旧凭据尚未导入，原数据已保留；重启后允许一次系统授权即可完成迁移".into()),
    };
    // A rejected import is not retried by every concurrent UI request.
    cache.insert(name.into(), result.clone());
    result
}

pub(crate) fn get(name: &str) -> LegacyResult {
    store()?.legacy(name, || legacy_get(name))
}
pub(crate) fn set(name: &str, value: &str) -> Result<(), String> {
    store()?.save(&format!("legacy:{name}"), &Some(value))
}
pub fn delete(name: &str) -> Result<(), String> {
    store()?.save(&format!("legacy:{name}"), &Option::<String>::None)
}

pub fn session() -> Result<Option<Session>, String> {
    if store()?.contains("campusAccounts")? {
        return Ok(None);
    }
    get("campus-session")?
        .map(|text| serde_json::from_str(&text).map_err(|_| "旧校园会话格式损坏".into()))
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
    set("model-api-key", key.trim())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn encrypted_books_migrate_then_work_without_os_keys() {
        use crate::{accounts, providers};
        use base64::{engine::general_purpose::STANDARD, Engine};
        let tmp = tempfile::tempdir().unwrap();
        initialize(tmp.path().join(".xjtu-launcher")).unwrap();
        let account_path = tmp.path().join("campus-accounts.enc");
        let provider_path = tmp.path().join("model-profiles.enc");
        let id = uuid::Uuid::new_v4().to_string();
        let mut accounts = accounts::Book::default();
        accounts
            .upsert(accounts::Account {
                id: id.clone(),
                username: "test-student".into(),
                degree: "undergraduate".into(),
                name: Some("Test Student".into()),
                student_id: Some("test-id".into()),
                avatar: Some("test-avatar".into()),
                mode: crate::network::AccessMode::Direct,
                token: "test-token".into(),
                cookies: "[]".into(),
                direct_cookies: None,
                password: Some(accounts::SavedPassword("test-password".into())),
            })
            .unwrap();
        let mut providers = providers::Book::default();
        providers
            .upsert(
                providers::Input {
                    id: None,
                    name: "Test".into(),
                    base_url: "https://example.com".into(),
                    protocol: "openai-compatible".into(),
                    model: "test-model".into(),
                    models: vec!["test-model".into()],
                    api_key: Some("test-api-key".into()),
                    use_legacy_key: false,
                },
                None,
            )
            .unwrap();
        let model_id = providers.providers[0].id.clone();
        let encrypted_accounts = accounts::seal(&accounts, &[1; 32]).unwrap();
        let encrypted_models = providers::seal(&providers, &[2; 32]).unwrap();
        fs::write(&account_path, &encrypted_accounts).unwrap();
        fs::write(&provider_path, &encrypted_models).unwrap();
        {
            let mut cache = LEGACY_CACHE.lock().unwrap();
            cache.insert(
                "campus-accounts-key-v1".into(),
                Ok(Some(STANDARD.encode([1; 32]))),
            );
            cache.insert(
                "model-profiles-key-v1".into(),
                Ok(Some(STANDARD.encode([2; 32]))),
            );
        }
        let imported = accounts::load(&account_path).unwrap();
        assert_eq!(
            imported.active().unwrap().password.as_ref().unwrap().0,
            "test-password"
        );
        assert_eq!(
            imported.active().unwrap().avatar.as_deref(),
            Some("test-avatar")
        );
        let imported = providers::load(&provider_path).unwrap();
        assert_eq!(imported.default_id.as_deref(), Some(model_id.as_str()));
        assert_eq!(imported.providers[0].api_key, "test-api-key");
        assert_eq!(imported.providers.len(), 2); // DeepSeek remains a one-time preset.
        {
            let mut cache = LEGACY_CACHE.lock().unwrap();
            for name in [
                "campus-accounts-key-v1",
                "model-profiles-key-v1",
                "campus-session",
            ] {
                cache.insert(name.into(), Err("OS access must not be needed".into()));
            }
        }
        let mut restored = accounts::load(&account_path).unwrap();
        assert_eq!(restored.active().unwrap().token, "test-token");
        assert_eq!(restored.active().unwrap().cookies, "[]");
        restored.forget_password(&id).unwrap();
        accounts::save(&account_path, &restored).unwrap();
        assert!(accounts::load(&account_path)
            .unwrap()
            .active()
            .unwrap()
            .password
            .is_none());
        restored.remove(&id).unwrap();
        accounts::save(&account_path, &restored).unwrap();
        assert!(accounts::load(&account_path).unwrap().accounts.is_empty());
        assert!(session().unwrap().is_none());
        let mut restored = providers::load(&provider_path).unwrap();
        restored.remove(&model_id).unwrap();
        providers::save(&provider_path, &restored).unwrap();
        assert_eq!(providers::load(&provider_path).unwrap().providers.len(), 1);
        assert_eq!(fs::read(account_path).unwrap(), encrypted_accounts);
        assert_eq!(fs::read(provider_path).unwrap(), encrypted_models);
        let disk = Store::new(&tmp.path().join(".xjtu-launcher"))
            .unwrap()
            .read()
            .unwrap();
        assert_eq!(disk.sections.len(), 2);
    }
    #[test]
    fn imports_once_and_preserves_accounts_models_and_deletion_tombstones() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path()).unwrap();
        let accounts = json!({"activeId":"test", "accounts":[{"password":"test-only"}]});
        assert_eq!(
            store
                .read_or_import("campusAccounts", || Ok(Some(accounts.clone())))
                .unwrap(),
            Some(accounts.clone())
        );
        assert_eq!(
            store
                .read_or_import::<Value>("campusAccounts", || panic!(
                    "must not read keychain again"
                ))
                .unwrap(),
            Some(accounts)
        );
        store
            .save("modelProviders", &json!({"providers":[]}))
            .unwrap();
        store
            .save("campusAccounts", &json!({"activeId":null,"accounts":[]}))
            .unwrap();
        let reloaded = Store::new(tmp.path()).unwrap();
        let data: Value = reloaded
            .read_or_import("campusAccounts", || {
                panic!("removed accounts must not reappear")
            })
            .unwrap()
            .unwrap();
        assert!(data["accounts"].as_array().unwrap().is_empty());
        assert!(reloaded.contains("modelProviders").unwrap());
        assert_eq!(
            reloaded
                .legacy("model-api-key", || Ok(Some("test-key".into())))
                .unwrap(),
            Some("test-key".into())
        );
        reloaded
            .save("legacy:model-api-key", &Option::<String>::None)
            .unwrap();
        assert!(reloaded
            .legacy("model-api-key", || panic!("deleted key must not reappear"))
            .unwrap()
            .is_none());
    }
    #[test]
    fn failed_import_and_corrupt_files_are_not_overwritten() {
        let tmp = tempfile::tempdir().unwrap();
        let store = Store::new(tmp.path()).unwrap();
        store
            .save("modelProviders", &json!({"providers":[]}))
            .unwrap();
        let original = fs::read(&store.path).unwrap();
        assert!(store
            .read_or_import::<Value>("campusAccounts", || Err("migration denied".into()))
            .is_err());
        assert_eq!(fs::read(&store.path).unwrap(), original);
        fs::write(&store.path, b"broken data").unwrap();
        assert!(store.save("campusAccounts", &json!({})).is_err());
        assert_eq!(fs::read(&store.path).unwrap(), b"broken data");
    }
    #[test]
    fn concurrent_sections_survive_and_file_is_private() {
        let tmp = tempfile::tempdir().unwrap();
        let store = std::sync::Arc::new(Store::new(tmp.path()).unwrap());
        let writers: Vec<_> = (0..8)
            .map(|i| {
                let store = store.clone();
                std::thread::spawn(move || store.save(&format!("test-{i}"), &i).unwrap())
            })
            .collect();
        for writer in writers {
            writer.join().unwrap();
        }
        assert_eq!(store.read().unwrap().sections.len(), 8);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&store.path).unwrap().permissions().mode() & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(tmp.path()).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
    }
}
