use shengjian_contract::{
    Capability, DeviceId, DeviceProfile, EngineCapabilities, PlatformKind, ProfileCatalog,
    ProfileError, RemoteCandidate, SupportStatus, TransportKind,
};
use std::path::Path;

/// Read-only device catalog. This is the only place that turns profile JSON
/// into capability facts for the app layer.
#[derive(Debug, Clone)]
pub struct DeviceCatalog {
    profiles: ProfileCatalog,
}

impl DeviceCatalog {
    pub fn new(profiles: ProfileCatalog) -> Self {
        Self { profiles }
    }

    pub fn bundled() -> Result<Self, ProfileError> {
        Ok(Self::new(ProfileCatalog::bundled()?))
    }

    pub fn load_dir(path: impl AsRef<Path>) -> Result<Self, ProfileError> {
        Ok(Self::new(ProfileCatalog::from_dir(path)?))
    }

    /// Resolve a scan/connection candidate to a device profile. Explicit
    /// `device_id` wins; model/name fallback is for platforms that only expose
    /// a human-readable model before GATT identity is available.
    pub fn resolve(&self, candidate: &RemoteCandidate) -> Option<&DeviceProfile> {
        if let Some(device_id) = &candidate.device_id {
            if let Some(profile) = self.get(device_id) {
                return Some(profile);
            }
        }
        if let Some(model) = &candidate.model {
            if let Some(profile) = self
                .profiles
                .profiles()
                .iter()
                .find(|profile| profile.model.eq_ignore_ascii_case(model))
            {
                return Some(profile);
            }
        }
        let name = candidate.name.trim();
        self.profiles.profiles().iter().find(|profile| {
            profile
                .identity
                .bluetooth_names
                .iter()
                .any(|known| known.eq_ignore_ascii_case(name))
                || profile
                    .identity
                    .bluetooth_name_prefixes
                    .iter()
                    .any(|prefix| {
                        !prefix.is_empty()
                            && name
                                .to_ascii_lowercase()
                                .starts_with(&prefix.to_ascii_lowercase())
                    })
                || profile.model.eq_ignore_ascii_case(name)
        })
    }

    pub fn profiles(&self) -> &[DeviceProfile] {
        self.profiles.profiles()
    }

    pub fn get(&self, id: &DeviceId) -> Option<&DeviceProfile> {
        self.profiles.get(id)
    }

    pub fn engine_capabilities(&self) -> EngineCapabilities {
        let profiles = self
            .profiles
            .profiles()
            .iter()
            .filter(|profile| profile.support.status == SupportStatus::Implemented)
            .map(|profile| profile.id.clone())
            .collect::<Vec<_>>();
        let bluetooth = self.profiles.profiles().iter().any(|profile| {
            profile.transports.iter().any(|transport| {
                matches!(
                    transport.kind,
                    TransportKind::BleGatt | TransportKind::BluetoothHid
                )
            })
        });
        let audio = self
            .profiles
            .profiles()
            .iter()
            .any(|profile| profile.has_capability(Capability::VoiceCapture));
        let full_key_capture = self.profiles.profiles().iter().any(|profile| {
            profile.supports_platform(PlatformKind::Windows)
                && profile
                    .platform(PlatformKind::Windows)
                    .is_some_and(|platform| {
                        platform
                            .features
                            .iter()
                            .any(|feature| feature == "enhanced-key-capture")
                    })
        });
        EngineCapabilities {
            bluetooth,
            audio,
            full_key_capture,
            profiles,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_contract::RemoteId;

    fn candidate(device_id: Option<DeviceId>, model: Option<&str>, name: &str) -> RemoteCandidate {
        RemoteCandidate {
            remote_id: RemoteId::new("remote-1"),
            name: name.to_owned(),
            model: model.map(str::to_owned),
            device_id,
            is_supported: true,
            token: None,
        }
    }

    #[test]
    fn resolves_by_device_id_then_model_then_name() {
        let catalog = DeviceCatalog::bundled().expect("catalog");
        let by_id = candidate(Some(DeviceId::new("xiaomi-rc003")), None, "unknown");
        assert_eq!(
            catalog.resolve(&by_id).map(|profile| profile.id.as_str()),
            Some("xiaomi-rc003")
        );

        let by_model = candidate(None, Some("RC003"), "unknown");
        assert_eq!(
            catalog
                .resolve(&by_model)
                .map(|profile| profile.id.as_str()),
            Some("xiaomi-rc003")
        );

        let unknown = candidate(None, Some("RC999"), "unknown");
        assert!(catalog.resolve(&unknown).is_none());
    }
}
