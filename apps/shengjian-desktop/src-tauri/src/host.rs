//! Platform-neutral host surface consumed by Tauri commands.
//!
//! Windows-specific types stay in `platform/windows.rs`; the command layer
//! only sees this trait. A future macOS/Linux host implements the same surface.

use serde::{Deserialize, Serialize};
use shengjian_domain::{MacroDefinition, MacroLibraryDocument};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostCapabilities {
    pub platform: String,
    pub enhanced_capture: bool,
    pub macro_runtime: bool,
    pub autostart: bool,
    pub tray: bool,
    pub common_phrases: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStatus {
    pub phase: String,
    pub generation: u64,
    pub reconnect_attempt: u32,
    pub power_notifications_available: bool,
    pub battery_level: Option<u8>,
    pub firmware_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscriptRecord {
    pub session_id: String,
    pub started_at_ms: i64,
    pub ended_at_ms: i64,
    pub local_date_key: String,
    pub application_key: String,
    pub application_name: String,
    pub source: String,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct MacroInfo {
    pub macro_id: String,
    pub name: String,
}

#[derive(Debug)]
pub struct HostError {
    pub code: &'static str,
    pub message: String,
}

impl HostError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MacroRunStatusView {
    pub running: bool,
    pub report: Option<serde_json::Value>,
}

/// Host services that are necessarily platform-specific.
///
/// Core ports already cover remote/audio/actions/buttons/settings; this trait
/// covers the lifecycle/services that live outside `CoreEngine`.
pub trait HostPlatform: Send + Sync {
    fn platform_id(&self) -> &'static str;
    fn capabilities(&self) -> HostCapabilities;
    fn battery_level(&self) -> Option<u8>;
    fn voice_level_dbfs(&self) -> f32;
    fn foreground_executable(&self) -> Option<String>;
    fn device_status(&self) -> Option<DeviceStatus>;

    fn set_enhanced_capture_enabled(&self, enabled: bool);
    /// 当前输入工具：传 `config/voice-tools.json` 里的 **id**（`None` = 未选择）。
    /// 默认热键由 id 查配置目录，Windows 行为由 id 对应 `kind` 决定。
    fn set_voice_input_tool(&self, tool: Option<&str>);
    /// 与 [`HostPlatform::set_voice_input_tool`] 同义的历史入口（同样接收工具 id）。
    fn set_voice_input_tool_name(&self, tool: Option<&str>);
    /// 自定义语音热键；`None` 表示跟随输入工具默认。
    fn set_voice_hotkey(&self, keys: Option<Vec<String>>);
    fn set_macro_library(&self, document: MacroLibraryDocument);
    fn macro_infos(&self) -> Vec<MacroInfo>;
    fn configure_transcript_capture(&self, sink: Arc<dyn Fn(TranscriptRecord) + Send + Sync>);
    fn set_transcript_capture_enabled(&self, enabled: bool);
    fn test_macro(&self, macro_id: &str) -> Result<(), HostError>;
    fn test_macro_definition(&self, definition: MacroDefinition) -> Result<(), HostError>;
    fn cancel_macro(&self, macro_id: &str) -> bool;
    fn macro_run_status(&self, macro_id: &str) -> MacroRunStatusView;
    fn helper_installed(&self) -> bool;
    fn install_helper(&self, app: &tauri::AppHandle) -> Result<(), HostError>;
    fn uninstall_helper(&self, app: &tauri::AppHandle) -> Result<(), HostError>;
    fn open_diagnostic_log_folder(&self) -> Result<(), HostError>;
    fn read_diagnostic_log(&self, max_bytes: usize) -> Result<String, HostError>;
    fn export_diagnostic_bundle(&self, path: &str) -> Result<(), HostError>;
}
