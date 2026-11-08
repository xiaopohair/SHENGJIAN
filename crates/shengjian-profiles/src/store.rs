//! `button-profiles.json` 的读写护栏。
//!
//! 与 `SettingsStore` 的 button-mappings 模式一致：互斥锁、更高 schemaVersion
//! 拒载并留 `.bak` 旁证、保存前规范化、临时文件 + 改名原子落盘。

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use shengjian_windows::gatt_note;

use crate::model::{
    ButtonProfile, ButtonProfilesDocument, ButtonProfilesExport, BUTTON_PROFILES_EXPORT_VERSION,
    BUTTON_PROFILES_SCHEMA_VERSION,
};

/// 导入文件大小上限（与 button-mappings 导入一致）。
pub const MAX_IMPORT_BYTES: u64 = 1024 * 1024;

pub struct ProfileStore {
    path: PathBuf,
    access: Mutex<()>,
}

impl ProfileStore {
    pub fn new(state_dir: &Path) -> Self {
        Self {
            path: state_dir.join("button-profiles.json"),
            access: Mutex::new(()),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn lock(&self) -> MutexGuard<'_, ()> {
        match self.access.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// 首启（NotFound）→ 默认文档；更高 schemaVersion → 拒载并留旁证备份；
    /// 结构损坏 → 错误。调用方按空库降级继续，不回写，保住用户原始文件。
    pub fn load(&self) -> Result<ButtonProfilesDocument, String> {
        let _guard = self.lock();
        let contents = match fs::read_to_string(&self.path) {
            Ok(contents) => contents,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Ok(ButtonProfilesDocument::default());
            }
            Err(error) => return Err(format!("读取键位方案失败：{error}")),
        };
        if let Some(version) = stored_schema_version(&contents) {
            if version > u64::from(BUTTON_PROFILES_SCHEMA_VERSION) {
                let backup = self.path.with_extension(format!("json.v{version}.bak"));
                if !backup.exists() {
                    let _ = fs::copy(&self.path, &backup);
                }
                gatt_note(format!(
                    "button_profiles store_load action=backup schema_version={version} supported_max={BUTTON_PROFILES_SCHEMA_VERSION}"
                ));
                return Err(format!("键位方案存储版本过高：{version}"));
            }
        }
        let document: ButtonProfilesDocument = serde_json::from_str(&contents)
            .map_err(|error| format!("解析键位方案失败：{error}"))?;
        Ok(document.normalized())
    }

    /// 规范化后原子落盘；返回保存的规范形。
    pub fn save(&self, document: ButtonProfilesDocument) -> Result<ButtonProfilesDocument, String> {
        let _guard = self.lock();
        let document = document.normalized();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|error| format!("创建应用设置目录失败：{error}"))?;
        }
        let contents = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("序列化键位方案失败：{error}"))?;
        let temporary = self.path.with_extension("json.tmp");
        fs::write(&temporary, contents).map_err(|error| format!("保存键位方案失败：{error}"))?;
        fs::rename(&temporary, &self.path).map_err(|error| format!("保存键位方案失败：{error}"))?;
        Ok(document)
    }

    /// 读取导出备份（大小护栏 + 版本校验）。
    pub fn read_import(&self, path: &Path) -> Result<Vec<ButtonProfile>, String> {
        let metadata =
            fs::metadata(path).map_err(|error| format!("读取键位方案备份失败：{error}"))?;
        if metadata.len() > MAX_IMPORT_BYTES {
            return Err("键位方案备份文件过大".to_owned());
        }
        let contents =
            fs::read_to_string(path).map_err(|error| format!("读取键位方案备份失败：{error}"))?;
        let export: ButtonProfilesExport = serde_json::from_str(&contents)
            .map_err(|error| format!("解析键位方案备份失败：{error}"))?;
        if export.format_version != BUTTON_PROFILES_EXPORT_VERSION {
            return Err(format!(
                "不支持的键位方案备份版本：{}",
                export.format_version
            ));
        }
        Ok(export.profiles)
    }

    /// 写出导出备份。
    pub fn write_export(&self, path: &Path, profiles: &[ButtonProfile]) -> Result<(), String> {
        let export = ButtonProfilesExport {
            format_version: BUTTON_PROFILES_EXPORT_VERSION,
            profiles: profiles.to_vec(),
        };
        // 经 serde_json::Value 落盘使键序稳定，便于比对与版本管理。
        let value = serde_json::to_value(&export)
            .map_err(|error| format!("序列化键位方案备份失败：{error}"))?;
        let contents = serde_json::to_vec_pretty(&value)
            .map_err(|error| format!("序列化键位方案备份失败：{error}"))?;
        fs::write(path, contents).map_err(|error| format!("写入键位方案备份失败：{error}"))
    }
}

/// 从原始内容提取 schemaVersion（版本护栏旁证备份用）。
fn stored_schema_version(contents: &str) -> Option<u64> {
    let value: serde_json::Value = serde_json::from_str(contents).ok()?;
    value.get("schemaVersion")?.as_u64()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::new_profile_id;

    fn temporary_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "shengjian-profiles-test-{name}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn sample_profile(id: &str) -> ButtonProfile {
        ButtonProfile {
            id: id.to_owned(),
            name: id.to_owned(),
            created_at: 7,
            app_targets: Vec::new(),
            auto_apply: false,
            bindings: std::collections::BTreeMap::new(),
        }
    }

    #[test]
    fn save_load_round_trip_and_normalizes() {
        let dir = temporary_dir("round-trip");
        let store = ProfileStore::new(&dir);
        let document = ButtonProfilesDocument {
            schema_version: 0,
            profiles: vec![sample_profile(&new_profile_id())],
            active_manual_profile_id: None,
        };
        let saved = store.save(document).unwrap();
        assert_eq!(saved.schema_version, BUTTON_PROFILES_SCHEMA_VERSION);
        let loaded = store.load().unwrap();
        assert_eq!(loaded, saved);
        assert_eq!(loaded.profiles.len(), 1);
    }

    #[test]
    fn missing_file_loads_default() {
        let dir = temporary_dir("missing");
        let store = ProfileStore::new(&dir);
        assert_eq!(store.load().unwrap(), ButtonProfilesDocument::default());
    }

    #[test]
    fn newer_schema_version_is_rejected_and_backed_up() {
        let dir = temporary_dir("newer-schema");
        let store = ProfileStore::new(&dir);
        let future = format!(
            "{{\"schemaVersion\":{}}}",
            u32::from(BUTTON_PROFILES_SCHEMA_VERSION) + 1
        );
        fs::write(store.path(), future).unwrap();
        assert!(store.load().is_err());
        let backup = store.path().with_extension(format!(
            "json.v{}.bak",
            u32::from(BUTTON_PROFILES_SCHEMA_VERSION) + 1
        ));
        assert!(backup.exists());
    }

    #[test]
    fn corrupted_file_is_rejected_without_backup_side_effects() {
        let dir = temporary_dir("corrupted");
        let store = ProfileStore::new(&dir);
        fs::write(store.path(), "{not json").unwrap();
        assert!(store.load().is_err());
    }

    #[test]
    fn export_round_trip_through_file() {
        let dir = temporary_dir("export");
        let store = ProfileStore::new(&dir);
        let export_path = dir.join("export.json");
        store
            .write_export(&export_path, &[sample_profile("x")])
            .unwrap();
        let profiles = store.read_import(&export_path).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "x");
    }

    #[test]
    fn import_rejects_unknown_format_version() {
        let dir = temporary_dir("import-version");
        let store = ProfileStore::new(&dir);
        let path = dir.join("import.json");
        fs::write(&path, "{\"formatVersion\":99,\"profiles\":[]}").unwrap();
        assert!(store.read_import(&path).is_err());
    }
}
