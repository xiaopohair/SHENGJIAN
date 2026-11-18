//! Ports implemented by platform adapters. The app layer never touches
//! Windows, macOS or UI types directly.

use crate::settings_namespace::SettingsDocument;
use shengjian_contract::{
    AudioEndpoint, AudioSnapshot, ButtonEdge, ButtonMappingSet, ButtonProfilesDocument,
    ConnectionSnapshot, CoreResult, EventEnvelope, LogicalAction, RemoteCandidate, RemoteId,
    VoiceState,
};
use shengjian_domain::AppSettings;
use std::sync::Arc;

pub trait RemoteLinkPort: Send + Sync {
    fn snapshot(&self) -> ConnectionSnapshot;
    fn scan(&self) -> CoreResult<Vec<RemoteCandidate>>;
    fn connect(&self, remote_id: &RemoteId) -> CoreResult<ConnectionSnapshot>;
    fn disconnect(&self) -> CoreResult<ConnectionSnapshot>;
    /// 解除平台配对关系（「忘记设备」）：先断开，再从系统配对列表里移除。
    fn forget(&self, remote_id: &RemoteId) -> CoreResult<()>;
}

/// Platform button edges after semantic decoding, before gesture recognition.
pub trait ButtonSourcePort: Send + Sync {
    fn subscribe_edges(&self, sink: Arc<dyn Fn(ButtonEdge) + Send + Sync>);
}

/// Foreground application executable (file name only; no path/title).
pub trait ForegroundPort: Send + Sync {
    fn executable(&self) -> Option<String>;
}

/// Live platform voice state; the engine reconciles it into `VoiceService`.
pub trait VoiceSourcePort: Send + Sync {
    fn state(&self) -> VoiceState;
}

pub trait AudioSinkPort: Send + Sync {
    fn snapshot(&self) -> AudioSnapshot;
    fn list_endpoints(&self) -> CoreResult<Vec<AudioEndpoint>>;
    fn select_endpoint(&self, endpoint_id: &str) -> CoreResult<AudioSnapshot>;
    fn set_gain_db(&self, gain_db: i16) -> CoreResult<AudioSnapshot>;
}

pub trait ActionSinkPort: Send + Sync {
    fn execute(&self, action: &LogicalAction) -> CoreResult<()>;

    /// 把**合成后**的按键映射推给平台注入层。
    ///
    /// 默认空实现：纯逻辑实现与测试替身不需要注入层。Windows 适配器会覆盖它，
    /// 否则 helper 旁路（Chromecast）的边沿到了平台侧却查不到动作（2026-11-18）。
    fn set_button_mappings(&self, _mappings: &shengjian_contract::ButtonMappingSet) {}
}

pub trait SettingsStorePort: Send + Sync {
    fn load(&self) -> CoreResult<AppSettings>;
    fn save(&self, settings: &AppSettings) -> CoreResult<()>;

    /// Full namespaced document. Default keeps older test doubles working by
    /// wrapping their flat settings as `global` with no device overrides.
    fn load_document(&self) -> CoreResult<SettingsDocument> {
        Ok(SettingsDocument::from_flat(self.load()?))
    }

    /// Default persistence only writes the global settings; stores that support
    /// namespaced documents should override this.
    fn save_document(&self, document: &SettingsDocument) -> CoreResult<()> {
        self.save(&document.global)
    }
}

pub trait ButtonProfilesStorePort: Send + Sync {
    fn load(&self) -> CoreResult<Option<ButtonProfilesDocument>>;
    fn save(&self, document: &ButtonProfilesDocument) -> CoreResult<()>;
}

pub trait ButtonMappingStorePort: Send + Sync {
    fn load(&self) -> CoreResult<Option<ButtonMappingSet>>;
    fn save(&self, mappings: &ButtonMappingSet) -> CoreResult<()>;
}

pub trait EventSink: Send + Sync {
    fn emit(&self, envelope: EventEnvelope);
}
