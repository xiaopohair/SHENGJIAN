use crate::events::TauriEventSink;
use crate::helper;
use crate::host::{
    DeviceStatus, HostCapabilities, HostError, HostPlatform, MacroInfo, MacroRunStatusView,
    TranscriptRecord,
};
use shengjian_app::CoreEngine;
use shengjian_domain::{MacroLibraryDocument, VoiceInputTool};
use shengjian_windows::common_phrases::{current_common_phrase_host, CommonPhraseAccess};
use shengjian_windows::macro_runtime::{macro_key_to_key_code, MacroRunReport, MacroStartError};
use shengjian_windows::send_input::{KeyChord, KeyCode};
use shengjian_windows::{DiagnosticLogMetadata, WindowsPlatform};
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
    current_tool: Mutex<Option<VoiceInputTool>>,
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
        let current_tool = *self.current_tool.lock().expect("voice tool lock");
        let hotkey = custom.or_else(|| default_voice_hotkey(current_tool));
        self.platform.set_voice_hold_hotkey(hotkey);
    }
}

fn default_voice_hotkey(tool: Option<VoiceInputTool>) -> Option<KeyChord> {
    match tool {
        Some(VoiceInputTool::Wechat) => Some(KeyChord {
            keys: vec![KeyCode::LeftControl, KeyCode::LeftWindows],
        }),
        Some(VoiceInputTool::Doubao) => Some(KeyChord {
            keys: vec![KeyCode::RightAlt],
        }),
        Some(VoiceInputTool::Vokie) | Some(VoiceInputTool::Other) | None => None,
    }
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

    fn set_voice_input_tool(&self, tool: Option<VoiceInputTool>) {
        *self.current_tool.lock().expect("voice tool lock") = tool;
        self.platform.set_voice_input_tool(tool);
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

    fn set_voice_input_tool_name(&self, tool: Option<&str>) {
        let parsed = match tool {
            Some("wechat") => Some(VoiceInputTool::Wechat),
            Some("doubao") => Some(VoiceInputTool::Doubao),
            Some("vokie") => Some(VoiceInputTool::Vokie),
            Some("other") => Some(VoiceInputTool::Other),
            _ => None,
        };
        self.set_voice_input_tool(parsed);
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
            .map_err(|error| {
                let code = match error {
                    MacroStartError::AlreadyRunning => "macro-already-running",
                    MacroStartError::NotFound => "macro-not-found",
                    MacroStartError::Invalid(_) => "macro-invalid",
                };
                HostError::new(code, code)
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_and_custom_hotkeys() {
        let wechat = parse_voice_hotkey(&["ControlLeft".to_owned(), "MetaLeft".to_owned()])
            .expect("wechat hotkey");
        assert_eq!(
            wechat.keys,
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

    #[test]
    fn default_hotkeys_follow_input_tool() {
        assert_eq!(
            default_voice_hotkey(Some(VoiceInputTool::Wechat)).map(|chord| chord.keys),
            Some(vec![KeyCode::LeftControl, KeyCode::LeftWindows])
        );
        assert_eq!(
            default_voice_hotkey(Some(VoiceInputTool::Doubao)).map(|chord| chord.keys),
            Some(vec![KeyCode::RightAlt])
        );
        assert!(default_voice_hotkey(Some(VoiceInputTool::Other)).is_none());
        assert!(default_voice_hotkey(None).is_none());
    }
}
