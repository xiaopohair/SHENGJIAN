use crate::events::TauriEventSink;
use crate::helper;
use crate::host::{
    DeviceStatus, HostCapabilities, HostError, HostPlatform, MacroInfo, MacroRunStatusView,
    TranscriptRecord,
};
use shengjian_app::CoreEngine;
use shengjian_domain::voice_tools;
use shengjian_domain::{MacroDefinition, MacroLibraryDocument};
use shengjian_windows::common_phrases::{current_common_phrase_host, CommonPhraseAccess};
use shengjian_windows::macro_runtime::{macro_key_to_key_code, MacroRunReport, MacroStartError};
use shengjian_windows::send_input::{KeyChord, KeyCode};
use shengjian_windows::{DiagnosticLogMetadata, VoiceToolKind, WindowsPlatform};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub fn initialize_diagnostics() {
    let log_path = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("Logs")
        .join("shengjian-diagnostic.log");
    let ready = shengjian_windows::initialize_diagnostic_log(
        log_path,
        DiagnosticLogMetadata {
            app_version: env!("CARGO_PKG_VERSION").to_owned(),
            app_build: "desktop".to_owned(),
            source_revision: String::new(),
            build_channel: "desktop".to_owned(),
            release_tag: String::new(),
        },
    );
    println!("shengjian desktop diagnostic log ready={ready}");
}

pub fn live_engine(
    sink: Arc<TauriEventSink>,
) -> Result<(CoreEngine, Arc<WindowsPlatform>), String> {
    shengjian_windows_adapter::live_core_engine_with_platform(sink)
        .map_err(|error| error.to_string())
}

pub fn note(message: String) {
    shengjian_windows::gatt_note(message);
}

pub struct PlatformHost {
    platform: Arc<WindowsPlatform>,
    /// 当前输入工具的 **id**（`config/voice-tools.json` 的 `id`，如 `wechat`）。
    ///
    /// 平台层保存 id 而不是枚举/行为：默认热键按 id 查配置目录，Windows 行为按
    /// id 对应配置里的 `kind` 解析（见 [`voice_tool_kind`]）。这样配置里新增/改名
    /// 工具时，这里不需要跟着改代码。
    current_tool: Mutex<Option<String>>,
    custom_hotkey: Mutex<Option<KeyChord>>,
}

impl PlatformHost {
    pub fn new(platform: Arc<WindowsPlatform>) -> Self {
        Self {
            platform,
            current_tool: Mutex::new(None),
            custom_hotkey: Mutex::new(None),
        }
    }

    fn apply_voice_hotkey(&self) {
        let custom = self
            .custom_hotkey
            .lock()
            .expect("voice hotkey lock")
            .clone();
        let current_tool = self.current_tool.lock().expect("voice tool lock").clone();
        let hotkey = custom.or_else(|| default_voice_hotkey(current_tool.as_deref()));
        self.platform.set_voice_hold_hotkey(hotkey);
    }
}

/// 工具 id → 默认热键（配置目录里的 `defaultHotkey`，`KeyboardEvent.code` 名）。
///
/// 目录里没有该 id、默认热键为空或无法识别时返回 `None`：没有默认热键的工具
/// （Vokie / custom）只有用户自定义热键，不替用户猜组合键。
fn default_voice_hotkey(tool_id: Option<&str>) -> Option<KeyChord> {
    let tool_id = tool_id?;
    let keys = voice_tools::default_hotkey(tool_id);
    parse_voice_hotkey(&keys)
}

/// 工具 id → Windows 行为类别：查配置目录的 `kind`；认不出的 id 按 `custom`
/// （只注入热键、不切/救输入法），**绝不**借用微信/豆包行为。
fn voice_tool_kind(tool_id: Option<&str>) -> Option<VoiceToolKind> {
    let tool_id = tool_id.map(str::trim).filter(|id| !id.is_empty())?;
    Some(VoiceToolKind::from_kind(
        &voice_tools::kind(tool_id).unwrap_or_default(),
    ))
}

/// 把前端 `KeyboardEvent.code` 列表解析成注入用 `KeyChord`。
///
/// 前端存原始 `code`（如 `ControlLeft`、`KeyQ`、`Digit1`、`ArrowLeft`），
/// 这里兼容该形式及宏的语义键名；无法识别的按键返回 `None`，由调用方回落到默认。
fn parse_voice_hotkey(keys: &[String]) -> Option<KeyChord> {
    let mut codes: Vec<KeyCode> = Vec::new();
    for name in keys {
        let code = match name.as_str() {
            "ControlLeft" | "LeftControl" => KeyCode::LeftControl,
            "ControlRight" | "RightControl" => KeyCode::RightControl,
            "ShiftLeft" | "LeftShift" => KeyCode::LeftShift,
            "ShiftRight" | "RightShift" => KeyCode::RightShift,
            "AltLeft" | "LeftAlt" => KeyCode::LeftAlt,
            "AltRight" | "RightAlt" => KeyCode::RightAlt,
            "MetaLeft" | "LeftWindows" => KeyCode::LeftWindows,
            "MetaRight" | "RightWindows" => KeyCode::RightWindows,
            "Space" => KeyCode::Space,
            "Enter" => KeyCode::Enter,
            "Escape" => KeyCode::Escape,
            "Tab" => KeyCode::Tab,
            "Backspace" => KeyCode::Backspace,
            "Delete" => KeyCode::Delete,
            "Insert" => KeyCode::Insert,
            "Home" => KeyCode::Home,
            "End" => KeyCode::End,
            "PageUp" => KeyCode::PageUp,
            "PageDown" => KeyCode::PageDown,
            "ArrowLeft" => KeyCode::Left,
            "ArrowRight" => KeyCode::Right,
            "ArrowUp" => KeyCode::Up,
            "ArrowDown" => KeyCode::Down,
            "Backquote" => KeyCode::Oem3,
            "Minus" => KeyCode::OemMinus,
            "Equal" => KeyCode::OemPlus,
            "BracketLeft" => KeyCode::Oem4,
            "BracketRight" => KeyCode::Oem6,
            "Backslash" => KeyCode::Oem5,
            "Semicolon" => KeyCode::Oem1,
            "Quote" => KeyCode::Oem7,
            "Comma" => KeyCode::OemComma,
            "Period" => KeyCode::OemPeriod,
            "Slash" => KeyCode::Oem2,
            "NumpadAdd" => KeyCode::NumpadAdd,
            "NumpadSubtract" => KeyCode::NumpadSubtract,
            "NumpadMultiply" => KeyCode::NumpadMultiply,
            "NumpadDivide" => KeyCode::NumpadDivide,
            "NumpadDecimal" => KeyCode::NumpadDecimal,
            "NumpadEnter" => KeyCode::Enter,
            other => {
                let macro_name = if let Some(rest) = other.strip_prefix("Key") {
                    rest.to_ascii_lowercase()
                } else if let Some(rest) = other.strip_prefix("Digit") {
                    rest.to_owned()
                } else {
                    other.to_ascii_lowercase()
                };
                macro_key_to_key_code(&macro_name)?
            }
        };
        if !codes.contains(&code) {
            codes.push(code);
        }
    }
    (!codes.is_empty()).then_some(KeyChord { keys: codes })
}

impl HostPlatform for PlatformHost {
    fn platform_id(&self) -> &'static str {
        "windows"
    }

    fn capabilities(&self) -> HostCapabilities {
        HostCapabilities {
            platform: self.platform_id().to_owned(),
            enhanced_capture: true,
            macro_runtime: true,
            autostart: true,
            tray: true,
            common_phrases: !matches!(
                current_common_phrase_host().access(),
                CommonPhraseAccess::Unavailable
            ),
        }
    }

    fn battery_level(&self) -> Option<u8> {
        self.platform.connection_snapshot().battery_level
    }

    fn voice_level_dbfs(&self) -> f32 {
        self.platform.voice_level_dbfs()
    }

    fn foreground_executable(&self) -> Option<String> {
        shengjian_windows::focus_windows::foreground_executable_path()
            .and_then(|path| path.rsplit(['\\', '/']).next().map(str::to_owned))
            .filter(|name| !name.is_empty())
    }

    fn device_status(&self) -> Option<DeviceStatus> {
        let snapshot = self.platform.connection_snapshot();
        if snapshot.phase == shengjian_windows::ConnectionPhase::Idle {
            return None;
        }
        Some(DeviceStatus {
            phase: format!("{:?}", snapshot.phase),
            generation: snapshot.generation,
            reconnect_attempt: snapshot.reconnect_attempt,
            power_notifications_available: snapshot.power_notifications_available,
            battery_level: snapshot.battery_level,
            firmware_version: snapshot.firmware_version,
        })
    }

    fn set_enhanced_capture_enabled(&self, enabled: bool) {
        self.platform.set_enhanced_capture_enabled(enabled);
        if enabled {
            helper::start_helper_task();
        } else {
            helper::stop_helper_task();
        }
    }

    fn set_voice_input_tool(&self, tool: Option<&str>) {
        // 平台层只认 id；空串按「未选择」处理，未知 id 保留（默认热键查不到就
        // 没有默认，行为按 custom），不静默丢掉用户的选择。
        let tool_id = tool
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .map(str::to_owned);
        self.platform
            .set_voice_input_tool(voice_tool_kind(tool_id.as_deref()));
        *self.current_tool.lock().expect("voice tool lock") = tool_id;
        self.apply_voice_hotkey();
    }

    fn set_voice_hotkey(&self, keys: Option<Vec<String>>) {
        let parsed = keys.as_deref().and_then(parse_voice_hotkey);
        note(format!(
            "voice_hotkey action=set source={} keys={}",
            if parsed.is_some() {
                "custom"
            } else {
                "default"
            },
            keys.as_deref().unwrap_or(&[]).join(","),
        ));
        *self.custom_hotkey.lock().expect("voice hotkey lock") = parsed;
        self.apply_voice_hotkey();
    }

    /// 与 [`HostPlatform::set_voice_input_tool`] 同义的历史入口：都接收配置目录里的
    /// 工具 id（前台轮询/方案覆盖用它换当前工具）。
    fn set_voice_input_tool_name(&self, tool: Option<&str>) {
        self.set_voice_input_tool(tool);
    }

    fn set_macro_library(&self, document: MacroLibraryDocument) {
        self.platform.set_macro_library(document);
    }

    fn macro_infos(&self) -> Vec<MacroInfo> {
        let library = self.platform.macro_runtime().library();
        let mut latest: std::collections::BTreeMap<String, (String, String)> =
            std::collections::BTreeMap::new();
        for definition in library.definitions {
            latest.insert(
                definition.macro_id.clone(),
                (definition.macro_id, definition.name),
            );
        }
        latest
            .into_values()
            .map(|(macro_id, name)| MacroInfo { macro_id, name })
            .collect()
    }

    fn configure_transcript_capture(&self, sink: Arc<dyn Fn(TranscriptRecord) + Send + Sync>) {
        let sink = Arc::new(
            move |captured: shengjian_windows::transcript_capture::CapturedTranscript| {
                sink(TranscriptRecord {
                    session_id: captured.session_id,
                    started_at_ms: captured.started_at_ms,
                    ended_at_ms: captured.ended_at_ms,
                    local_date_key: captured.local_date_key,
                    application_key: captured.application_key,
                    application_name: captured.application_name,
                    source: format!("{:?}", captured.source),
                    text: captured.text,
                });
            },
        );
        self.platform.set_transcript_capture_sink(sink);
        self.platform.set_transcript_audio_recording_enabled(false);
    }

    fn set_transcript_capture_enabled(&self, enabled: bool) {
        self.platform.set_transcript_capture_enabled(enabled);
    }

    fn test_macro(&self, macro_id: &str) -> Result<(), HostError> {
        self.platform
            .macro_runtime()
            .start(macro_id)
            .map_err(macro_start_error)
    }

    fn test_macro_definition(&self, definition: MacroDefinition) -> Result<(), HostError> {
        self.platform
            .macro_runtime()
            .start_definition(definition)
            .map_err(macro_start_error)
    }

    fn macro_run_status(&self, macro_id: &str) -> MacroRunStatusView {
        let status = self.platform.macro_runtime().run_status(macro_id);
        MacroRunStatusView {
            running: status.running,
            report: status
                .report
                .and_then(|report: MacroRunReport| serde_json::to_value(report).ok()),
        }
    }

    fn cancel_macro(&self, macro_id: &str) -> bool {
        self.platform.macro_runtime().cancel(macro_id)
    }

    fn helper_installed(&self) -> bool {
        helper::task_installed()
    }

    fn install_helper(&self, app: &tauri::AppHandle) -> Result<(), HostError> {
        helper::request_helper_task_install(app)
            .map_err(|message| HostError::new("rc003-helper", message))
    }

    fn open_diagnostic_log_folder(&self) -> Result<(), HostError> {
        let path = diagnostic_log_path()
            .parent()
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        std::fs::create_dir_all(&path)
            .map_err(|_| HostError::new("log-folder", "cannot create log folder"))?;
        std::process::Command::new("explorer")
            .arg(&path)
            .spawn()
            .map_err(|_| HostError::new("log-folder", "cannot open log folder"))?;
        Ok(())
    }

    fn read_diagnostic_log(&self, max_bytes: usize) -> Result<String, HostError> {
        let path = diagnostic_log_path();
        let bytes = std::fs::read(&path)
            .map_err(|_| HostError::new("log-read", "cannot read diagnostic log"))?;
        let start = bytes
            .len()
            .saturating_sub(max_bytes.clamp(1024, 512 * 1024));
        Ok(String::from_utf8_lossy(&bytes[start..]).into_owned())
    }

    fn export_diagnostic_bundle(&self, path: &str) -> Result<(), HostError> {
        let log = self.read_diagnostic_log(256 * 1024)?;
        let bundle = serde_json::json!({
            "format": "shengjian-diagnostics",
            "version": 1,
            "appVersion": env!("CARGO_PKG_VERSION"),
            "platform": self.platform_id(),
            "helperInstalled": self.helper_installed(),
            "log": log,
        });
        let contents = serde_json::to_string_pretty(&bundle)
            .map_err(|_| HostError::new("diagnostics-export", "cannot encode diagnostics"))?;
        std::fs::write(path, contents)
            .map_err(|_| HostError::new("diagnostics-export", "cannot write diagnostics"))?;
        Ok(())
    }
}

fn diagnostic_log_path() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Shengjian")
        .join("Logs")
        .join("shengjian-diagnostic.log")
}

pub fn create_host(platform: Arc<WindowsPlatform>) -> Arc<dyn HostPlatform> {
    Arc::new(PlatformHost::new(platform))
}

fn macro_start_error(error: MacroStartError) -> HostError {
    let code = match error {
        MacroStartError::AlreadyRunning => "macro-already-running",
        MacroStartError::NotFound => "macro-not-found",
        MacroStartError::Invalid(_) => "macro-invalid",
    };
    HostError::new(code, code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_and_custom_hotkeys() {
        // 纯解析夹具（不是某个工具的默认值）：`KeyboardEvent.code` 名 → KeyChord。
        let ctrl_win = parse_voice_hotkey(&["ControlLeft".to_owned(), "MetaLeft".to_owned()])
            .expect("ctrl+win hotkey");
        assert_eq!(
            ctrl_win.keys,
            vec![KeyCode::LeftControl, KeyCode::LeftWindows]
        );

        let custom = parse_voice_hotkey(&[
            "ControlLeft".to_owned(),
            "AltLeft".to_owned(),
            "KeyQ".to_owned(),
        ])
        .expect("custom hotkey");
        assert_eq!(
            custom.keys,
            vec![KeyCode::LeftControl, KeyCode::LeftAlt, KeyCode::Q]
        );

        let arrow = parse_voice_hotkey(&["ArrowLeft".to_owned()]).expect("arrow hotkey");
        assert_eq!(arrow.keys, vec![KeyCode::Left]);
        assert!(parse_voice_hotkey(&["NotAKey".to_owned()]).is_none());
        assert!(parse_voice_hotkey(&[]).is_none());
    }

    /// 默认热键**完全来自配置目录**（`config/voice-tools.json` 的 `defaultHotkey`），
    /// 这里不再写死任何工具组合键：断言解析链路与目录一致，并钉住空列表→None。
    #[test]
    fn default_hotkeys_follow_the_voice_tool_catalog() {
        for tool in voice_tools::tools() {
            assert_eq!(
                default_voice_hotkey(Some(&tool.id)),
                parse_voice_hotkey(&tool.default_hotkey),
                "tool={}",
                tool.id
            );
            if tool.default_hotkey.is_empty() {
                assert!(
                    default_voice_hotkey(Some(&tool.id)).is_none(),
                    "目录里没有默认热键的工具不得有默认值: {}",
                    tool.id
                );
            }
        }
        assert!(default_voice_hotkey(None).is_none());
        // 认不出的 id（目录已删/换了名字）：没有默认热键，不回落硬编码组合键。
        assert!(default_voice_hotkey(Some("not-in-catalog")).is_none());
        assert!(default_voice_hotkey(Some("  ")).is_none());
    }

    /// 工具 id → 行为类别：目录里的 `kind` 决定；未知 id / 空串按 custom 或 None。
    /// 新增自定义工具时必须是 custom（只有自定义热键、无 IME 修复）。
    #[test]
    fn voice_tool_kind_comes_from_the_catalog_kind_field() {
        for tool in voice_tools::tools() {
            assert_eq!(
                voice_tool_kind(Some(&tool.id)),
                Some(VoiceToolKind::from_kind(&tool.kind)),
                "tool={}",
                tool.id
            );
        }
        assert_eq!(voice_tool_kind(None), None);
        assert_eq!(voice_tool_kind(Some("  ")), None);
        assert_eq!(
            voice_tool_kind(Some("not-in-catalog")),
            Some(VoiceToolKind::Custom)
        );
    }
}
