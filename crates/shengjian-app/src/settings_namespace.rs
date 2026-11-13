//! Core settings namespace.
//!
//! `global` keeps the flat `AppSettings` shape; device entries are sparse
//! overrides under `devices.<device-id>.<platform>`. Resolution applies the
//! selected device/platform override on top of global settings.
//!
//! Persistence migration is intentionally not in this module: the host owns
//! reading an old flat settings file and deciding which device it belongs to.

use serde::{Deserialize, Serialize};
use shengjian_domain::AppSettings;
use std::collections::BTreeMap;

pub const SETTINGS_NAMESPACE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSettingsOverride {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gain_db: Option<i16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_endpoint_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio_endpoint_name: Option<String>,
    /// 输入工具 id（配置目录 `config/voice-tools.json` 的 `id`，未知 id 原样保留）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice_input_tool: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDocument {
    pub schema_version: u32,
    pub global: AppSettings,
    #[serde(default)]
    pub devices: BTreeMap<String, BTreeMap<String, DeviceSettingsOverride>>,
}

impl Default for SettingsDocument {
    fn default() -> Self {
        Self::from_flat(AppSettings::default())
    }
}

impl SettingsDocument {
    pub fn from_flat(settings: AppSettings) -> Self {
        Self {
            schema_version: SETTINGS_NAMESPACE_SCHEMA_VERSION,
            global: settings,
            devices: BTreeMap::new(),
        }
    }
    pub fn resolve(&self, device_id: Option<&str>, platform: &str) -> AppSettings {
        let mut settings = self.global.clone();
        let Some(device_id) = device_id else {
            return settings;
        };
        let Some(platforms) = self.devices.get(device_id) else {
            return settings;
        };
        let Some(overrides) = platforms.get(platform) else {
            return settings;
        };
        if let Some(gain_db) = overrides.gain_db {
            settings.gain_db = f32::from(gain_db);
        }
        if let Some(endpoint_id) = &overrides.audio_endpoint_id {
            settings.audio_endpoint_id = Some(endpoint_id.clone());
        }
        if let Some(endpoint_name) = &overrides.audio_endpoint_name {
            settings.audio_endpoint_name = Some(endpoint_name.clone());
        }
        if let Some(tool) = &overrides.voice_input_tool {
            settings.voice_input_tool = Some(tool.clone());
        }
        settings
    }

    pub fn set_override(
        &mut self,
        device_id: &str,
        platform: &str,
        overrides: DeviceSettingsOverride,
    ) {
        self.devices
            .entry(device_id.to_owned())
            .or_default()
            .insert(platform.to_owned(), overrides);
    }

    pub fn normalized(mut self) -> Self {
        self.schema_version = SETTINGS_NAMESPACE_SCHEMA_VERSION;
        self.global = self.global.normalized();
        for platforms in self.devices.values_mut() {
            for overrides in platforms.values_mut() {
                if let Some(gain) = overrides.gain_db {
                    overrides.gain_db = Some(gain.clamp(0, 24));
                }
                if overrides
                    .audio_endpoint_id
                    .as_deref()
                    .is_some_and(|id| id.trim().is_empty())
                {
                    overrides.audio_endpoint_id = None;
                }
                if overrides
                    .audio_endpoint_name
                    .as_deref()
                    .is_some_and(|name| name.trim().is_empty())
                {
                    overrides.audio_endpoint_name = None;
                }
                // 输入工具覆盖值同样是「空 = 未覆盖」；id 集合由配置目录决定，
                // 认不出的 id 不在这里清掉。
                if overrides
                    .voice_input_tool
                    .as_deref()
                    .is_some_and(|id| id.trim().is_empty())
                {
                    overrides.voice_input_tool = None;
                }
            }
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn override_with_gain(gain: i16) -> DeviceSettingsOverride {
        DeviceSettingsOverride {
            gain_db: Some(gain),
            audio_endpoint_id: Some("cable".to_owned()),
            audio_endpoint_name: Some("CABLE Input".to_owned()),
            voice_input_tool: Some("doubao".to_owned()),
        }
    }

    #[test]
    fn resolves_global_then_device_override() {
        let mut document = SettingsDocument::from_flat(AppSettings::default());
        document.set_override("xiaomi-rc003", "windows", override_with_gain(12));
        let resolved = document.resolve(Some("xiaomi-rc003"), "windows");
        assert_eq!(resolved.gain_db, 12.0);
        assert_eq!(resolved.audio_endpoint_id.as_deref(), Some("cable"));
        assert_eq!(resolved.voice_input_tool.as_deref(), Some("doubao"));

        let other = document.resolve(Some("xiaomi-rc001"), "windows");
        assert_eq!(other.gain_db, document.global.gain_db);
    }

    #[test]
    fn legacy_flat_settings_become_global_without_device_overrides() {
        let flat = AppSettings {
            gain_db: 7.0,
            ..AppSettings::default()
        };
        let document = SettingsDocument::from_flat(flat.clone());
        assert_eq!(document.global, flat);
        assert!(document.devices.is_empty());
        assert_eq!(document.resolve(None, "windows").gain_db, 7.0);
    }

    #[test]
    fn normalization_clamps_device_gain_and_drops_empty_endpoint() {
        let mut document = SettingsDocument::from_flat(AppSettings::default());
        document.set_override(
            "device",
            "windows",
            DeviceSettingsOverride {
                gain_db: Some(99),
                audio_endpoint_id: Some(" ".to_owned()),
                audio_endpoint_name: Some(" ".to_owned()),
                voice_input_tool: None,
            },
        );
        let document = document.normalized();
        let overrides = &document.devices["device"]["windows"];
        assert_eq!(overrides.gain_db, Some(24));
        assert_eq!(overrides.audio_endpoint_id, None);
        assert_eq!(overrides.audio_endpoint_name, None);
    }
}
