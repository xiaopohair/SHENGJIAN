use crate::ids::DeviceId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SupportStatus {
    Implemented,
    Research,
    Planned,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Support {
    pub status: SupportStatus,
    pub notes: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HidIdentity {
    pub vendor_id: u16,
    pub product_id: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceIdentity {
    #[serde(default)]
    pub bluetooth_names: Vec<String>,
    #[serde(default)]
    pub bluetooth_name_prefixes: Vec<String>,
    #[serde(default)]
    pub model_numbers: Vec<String>,
    #[serde(default)]
    pub hid: Option<HidIdentity>,
    #[serde(default)]
    pub audio_input_name_prefixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportKind {
    BleGatt,
    BluetoothHid,
    BluetoothBrEdrAudio,
    UsbDigitalAudio,
    AnalogAudio,
    SystemAudioInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TransportRole {
    Control,
    ButtonInput,
    VoiceInput,
    VoiceOutput,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transport {
    pub kind: TransportKind,
    pub role: TransportRole,
    pub status: SupportStatus,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Capability {
    VoiceCapture,
    PressToTalk,
    ButtonInput,
    HostKeyMapping,
    GainControl,
    ChannelSelection,
    DeviceStatus,
    EnhancedKeyCapture,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Protocol {
    pub id: String,
    pub status: SupportStatus,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlatformKind {
    Macos,
    Windows,
    Linux,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PlatformSupport {
    pub platform: PlatformKind,
    pub status: SupportStatus,
    pub notes: String,
    #[serde(default)]
    pub features: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeviceProfile {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub schema_version: u32,
    pub id: DeviceId,
    pub display_name: String,
    pub vendor: String,
    pub model: String,
    pub support: Support,
    #[serde(default)]
    pub identity: DeviceIdentity,
    pub transports: Vec<Transport>,
    pub capabilities: Vec<Capability>,
    #[serde(default)]
    pub protocols: Vec<Protocol>,
    pub platforms: Vec<PlatformSupport>,
    pub sources: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl DeviceProfile {
    pub fn has_capability(&self, capability: Capability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn platform(&self, platform: PlatformKind) -> Option<&PlatformSupport> {
        self.platforms
            .iter()
            .find(|entry| entry.platform == platform)
    }

    pub fn supports_platform(&self, platform: PlatformKind) -> bool {
        self.platform(platform)
            .is_some_and(|entry| entry.status == SupportStatus::Implemented)
    }

    fn validate(&self) -> Result<(), ProfileError> {
        if self.schema_version != 1 {
            return Err(ProfileError::UnsupportedVersion(self.schema_version));
        }
        if !is_valid_profile_id(self.id.as_str()) {
            return Err(ProfileError::InvalidId(self.id.to_string()));
        }
        if self.display_name.trim().is_empty() || self.vendor.trim().is_empty() {
            return Err(ProfileError::MissingField("displayName/vendor".to_owned()));
        }
        if self.transports.is_empty() {
            return Err(ProfileError::MissingField("transports".to_owned()));
        }
        if self.capabilities.is_empty() {
            return Err(ProfileError::MissingField("capabilities".to_owned()));
        }
        if self.platforms.is_empty() {
            return Err(ProfileError::MissingField("platforms".to_owned()));
        }
        if self.sources.is_empty() {
            return Err(ProfileError::MissingField("sources".to_owned()));
        }
        Ok(())
    }
}

fn is_valid_profile_id(value: &str) -> bool {
    !value.is_empty()
        && value.split(|ch| ch == '.' || ch == '-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit())
        })
}

#[derive(Debug, Clone, Default)]
pub struct ProfileCatalog {
    profiles: Vec<DeviceProfile>,
}

impl ProfileCatalog {
    pub fn new(profiles: Vec<DeviceProfile>) -> Result<Self, ProfileError> {
        let mut seen: BTreeMap<DeviceId, ()> = BTreeMap::new();
        for profile in &profiles {
            profile.validate()?;
            if seen.insert(profile.id.clone(), ()).is_some() {
                return Err(ProfileError::DuplicateId(profile.id.to_string()));
            }
        }
        Ok(Self { profiles })
    }

    pub fn from_json(json: &str) -> Result<DeviceProfile, ProfileError> {
        let profile: DeviceProfile = serde_json::from_str(json)?;
        profile.validate()?;
        Ok(profile)
    }

    /// Load all device profiles from a directory. `schema.json` and
    /// non-JSON files are ignored. Empty directories are rejected so a typo
    /// cannot silently produce an empty runtime catalog.
    pub fn from_dir(path: impl AsRef<Path>) -> Result<Self, ProfileError> {
        let mut paths = fs::read_dir(path)?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.is_file()
                    && path
                        .extension()
                        .is_some_and(|extension| extension.eq_ignore_ascii_case("json"))
                    && path.file_name().is_none_or(|name| name != "schema.json")
            })
            .collect::<Vec<_>>();
        paths.sort();
        if paths.is_empty() {
            return Err(ProfileError::NoProfiles);
        }
        let mut profiles = Vec::with_capacity(paths.len());
        for path in paths {
            let json = fs::read_to_string(&path)?;
            profiles.push(Self::from_json(&json)?);
        }
        Self::new(profiles)
    }

    pub fn bundled() -> Result<Self, ProfileError> {
        let rc003 = Self::from_json(include_str!("../../../profiles/xiaomi-rc003.json"))?;
        let rc001 = Self::from_json(include_str!("../../../profiles/xiaomi-rc001.json"))?;
        Self::new(vec![rc003, rc001])
    }

    pub fn profiles(&self) -> &[DeviceProfile] {
        &self.profiles
    }

    pub fn get(&self, id: &DeviceId) -> Option<&DeviceProfile> {
        self.profiles.iter().find(|profile| &profile.id == id)
    }

    pub fn implemented(&self) -> impl Iterator<Item = &DeviceProfile> {
        self.profiles
            .iter()
            .filter(|profile| profile.support.status == SupportStatus::Implemented)
    }
}

#[derive(Debug, Error)]
pub enum ProfileError {
    #[error("invalid profile JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("profile I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("no device profiles found")]
    NoProfiles,
    #[error("unsupported profile schema version: {0}")]
    UnsupportedVersion(u32),
    #[error("invalid profile id: {0}")]
    InvalidId(String),
    #[error("missing or empty profile field: {0}")]
    MissingField(String),
    #[error("duplicate profile id: {0}")]
    DuplicateId(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_profiles_parse_and_validate() {
        let catalog = ProfileCatalog::bundled().expect("bundled profiles must be valid");
        assert_eq!(catalog.profiles().len(), 2);
        let rc003 = catalog
            .get(&DeviceId::new("xiaomi-rc003"))
            .expect("rc003 profile");
        assert!(rc003.has_capability(Capability::VoiceCapture));
        assert!(rc003.has_capability(Capability::EnhancedKeyCapture));
        assert!(rc003.supports_platform(PlatformKind::Windows));
        assert!(!rc003.supports_platform(PlatformKind::Linux));

        let rc001 = catalog
            .get(&DeviceId::new("xiaomi-rc001"))
            .expect("rc001 profile");
        assert_eq!(rc001.support.status, SupportStatus::Research);
        assert!(rc001.identity.hid.is_none());
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let profile =
            ProfileCatalog::from_json(include_str!("../../../profiles/xiaomi-rc003.json"))
                .expect("profile");
        let error = ProfileCatalog::new(vec![profile.clone(), profile]).expect_err("duplicate");
        assert!(matches!(error, ProfileError::DuplicateId(_)));
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let json = include_str!("../../../profiles/xiaomi-rc003.json")
            .replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
        let error = ProfileCatalog::from_json(&json).expect_err("version");
        assert!(matches!(error, ProfileError::UnsupportedVersion(2)));
    }

    #[test]
    fn invalid_id_is_rejected() {
        let json = include_str!("../../../profiles/xiaomi-rc003.json")
            .replace("\"id\": \"xiaomi-rc003\"", "\"id\": \"Xiaomi RC003\"");
        let error = ProfileCatalog::from_json(&json).expect_err("id");
        assert!(matches!(error, ProfileError::InvalidId(_)));
    }

    #[test]
    fn loads_profiles_from_directory_and_ignores_schema() {
        let dir =
            std::env::temp_dir().join(format!("shengjian-profiles-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        fs::write(
            dir.join("rc003.json"),
            include_str!("../../../profiles/xiaomi-rc003.json"),
        )
        .expect("write profile");
        fs::write(dir.join("schema.json"), "{}").expect("write schema");
        let catalog = ProfileCatalog::from_dir(&dir).expect("catalog");
        assert_eq!(catalog.profiles().len(), 1);
        assert!(catalog.get(&DeviceId::new("xiaomi-rc003")).is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_profile_directory_is_rejected() {
        let dir = std::env::temp_dir().join(format!(
            "shengjian-profiles-empty-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("dir");
        let error = ProfileCatalog::from_dir(&dir).expect_err("empty");
        assert!(matches!(error, ProfileError::NoProfiles));
        let _ = fs::remove_dir_all(&dir);
    }
}
