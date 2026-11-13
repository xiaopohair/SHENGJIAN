use crate::error::EngineError;
use crate::ids::{DeviceId, Generation, RemoteId};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteCandidate {
    pub remote_id: RemoteId,
    pub name: String,
    pub model: Option<String>,
    pub device_id: Option<DeviceId>,
    pub is_supported: bool,
    /// Opaque stable token for matching a scan row with the connected device.
    /// Platform identity (Bluetooth address / HID path) never enters the contract.
    #[serde(default)]
    pub token: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectionPhase {
    Idle,
    Scanning,
    Connecting,
    Ready,
    Reconnecting,
    Disconnecting,
    Disconnected,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionSnapshot {
    pub phase: ConnectionPhase,
    pub remote: Option<RemoteCandidate>,
    pub generation: Generation,
    pub last_error: Option<EngineError>,
}

impl Default for ConnectionSnapshot {
    fn default() -> Self {
        Self {
            phase: ConnectionPhase::Idle,
            remote: None,
            generation: Generation::initial(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VoiceState {
    Idle,
    Streaming,
    Draining,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceSnapshot {
    pub state: VoiceState,
    pub generation: Generation,
    pub input_tool: Option<String>,
    pub hold_hotkey: Vec<String>,
    pub last_error: Option<EngineError>,
}

impl Default for VoiceSnapshot {
    fn default() -> Self {
        Self {
            state: VoiceState::Idle,
            generation: Generation::initial(),
            input_tool: None,
            hold_hotkey: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AudioState {
    Unconfigured,
    Ready,
    Streaming,
    Draining,
    Failed,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioEndpoint {
    pub id: String,
    pub name: String,
    pub recommended: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioFormat {
    pub sample_rate_hz: u32,
    pub channels: u8,
    pub bits_per_sample: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioSnapshot {
    pub state: AudioState,
    pub endpoint: Option<AudioEndpoint>,
    pub format: Option<AudioFormat>,
    pub gain_db: i16,
    pub generation: Generation,
    pub last_error: Option<EngineError>,
}

impl Default for AudioSnapshot {
    fn default() -> Self {
        Self {
            state: AudioState::Unconfigured,
            endpoint: None,
            format: None,
            gain_db: 0,
            generation: Generation::initial(),
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LogicalButton {
    Back,
    Ok,
    Tv,
    Home,
    Right,
    Left,
    Down,
    Up,
    Menu,
    Power,
    VolumeMute,
    VolumeUp,
    VolumeDown,
}

impl LogicalButton {
    pub const ALL: [Self; 13] = [
        Self::Back,
        Self::Ok,
        Self::Tv,
        Self::Home,
        Self::Right,
        Self::Left,
        Self::Down,
        Self::Up,
        Self::Menu,
        Self::Power,
        Self::VolumeMute,
        Self::VolumeUp,
        Self::VolumeDown,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ButtonTrigger {
    Single,
    Double,
    Long,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonEdge {
    pub button: LogicalButton,
    pub pressed: bool,
    pub at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonGesture {
    pub button: LogicalButton,
    pub trigger: ButtonTrigger,
    pub at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FocusStrategy {
    Direct,
    LearnedElement,
    Shortcut,
    RecordedPoint,
}

impl Default for FocusStrategy {
    fn default() -> Self {
        Self::Direct
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MouseActionKind {
    Scroll,
    Move,
    Click,
    RightClick,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum LogicalAction {
    Disabled,
    Shortcut {
        label: String,
        keys: Vec<String>,
    },
    OpenApp {
        app: String,
        #[serde(default)]
        focus: FocusStrategy,
    },
    FocusInput,
    RunMacro {
        macro_id: String,
    },
    Mouse {
        kind: MouseActionKind,
        direction: Option<Direction>,
        amount: u16,
    },
    CommonPhrase,
    /// 系统动作：音量/媒体/退出等**单键**动作。
    ///
    /// `key` 语义键名与宏系统动作一致（`volume_up` / `volume_down` /
    /// `volume_mute` / `media_play_pause` / `media_prev` / `media_next` /
    /// `escape`…）；执行层按单键快捷键注入（等价于 `Shortcut { keys: [key] }`），
    /// 未知键名交给底层 KeyCode 解析失败。
    SystemAction {
        key: String,
    },
}

impl LogicalAction {
    pub fn is_disabled(&self) -> bool {
        matches!(self, Self::Disabled)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonActions {
    pub single: LogicalAction,
    pub double: LogicalAction,
    pub long: LogicalAction,
    pub hold_repeat: Option<ButtonTrigger>,
    /// OK 键在最近一次遥控器移动后的窗口内按 OK 时，直接点击光标位置并吞掉
    /// 本次按压；只对 OK 生效。
    #[serde(default)]
    pub ok_context_click: bool,
}

impl Default for ButtonActions {
    fn default() -> Self {
        Self {
            single: LogicalAction::Disabled,
            double: LogicalAction::Disabled,
            long: LogicalAction::Disabled,
            hold_repeat: None,
            ok_context_click: false,
        }
    }
}

impl ButtonActions {
    pub fn action(&self, trigger: ButtonTrigger) -> &LogicalAction {
        match trigger {
            ButtonTrigger::Single => &self.single,
            ButtonTrigger::Double => &self.double,
            ButtonTrigger::Long => &self.long,
        }
    }

    pub fn any_configured(&self) -> bool {
        !self.single.is_disabled() || !self.double.is_disabled() || !self.long.is_disabled()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonMappingSet {
    pub enabled: bool,
    pub actions: BTreeMap<LogicalButton, ButtonActions>,
}

impl Default for ButtonMappingSet {
    fn default() -> Self {
        Self {
            enabled: true,
            actions: BTreeMap::new(),
        }
    }
}

impl ButtonMappingSet {
    pub fn action(&self, button: LogicalButton, trigger: ButtonTrigger) -> LogicalAction {
        if !self.enabled {
            return LogicalAction::Disabled;
        }
        self.actions
            .get(&button)
            .map(|actions| actions.action(trigger).clone())
            .unwrap_or(LogicalAction::Disabled)
    }

    pub fn set_actions(&mut self, button: LogicalButton, actions: ButtonActions) {
        self.actions.insert(button, actions);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineCapabilities {
    pub bluetooth: bool,
    pub audio: bool,
    pub full_key_capture: bool,
    pub profiles: Vec<DeviceId>,
}

impl Default for EngineCapabilities {
    fn default() -> Self {
        Self {
            bluetooth: false,
            audio: false,
            full_key_capture: false,
            profiles: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MacroRunState {
    Idle,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusResult {
    pub success: bool,
    pub target: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum CoreEvent {
    #[serde(rename = "connection.changed")]
    ConnectionChanged { snapshot: ConnectionSnapshot },
    #[serde(rename = "voice.changed")]
    VoiceChanged { snapshot: VoiceSnapshot },
    #[serde(rename = "audio.changed")]
    AudioChanged { snapshot: AudioSnapshot },
    #[serde(rename = "button.edge")]
    ButtonEdge { edge: ButtonEdge },
    #[serde(rename = "button.gesture")]
    ButtonGesture { gesture: ButtonGesture },
    #[serde(rename = "focus.result")]
    FocusResult { result: FocusResult },
    #[serde(rename = "macro.runStatus")]
    MacroRunStatus {
        macro_id: String,
        state: MacroRunState,
    },
    #[serde(rename = "helper.ownership")]
    HelperOwnership { available: bool, owned: bool },
    #[serde(rename = "engine.error")]
    Error { error: EngineError },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EventEnvelope {
    pub seq: u64,
    pub generation: Generation,
    pub event: CoreEvent,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_envelope_serializes_without_platform_types() {
        let envelope = EventEnvelope {
            seq: 7,
            generation: Generation::from(3),
            event: CoreEvent::ConnectionChanged {
                snapshot: ConnectionSnapshot {
                    phase: ConnectionPhase::Ready,
                    remote: None,
                    generation: Generation::from(3),
                    last_error: None,
                },
            },
        };
        let json = serde_json::to_value(&envelope).expect("serialize");
        assert_eq!(json["seq"], 7);
        assert_eq!(json["event"]["type"], "connection.changed");
        assert_eq!(json["event"]["snapshot"]["phase"], "ready");
    }

    /// 系统动作契约：`{ "type": "system-action", "key": "volume_up" }`（kebab tag + key 字段）。
    #[test]
    fn system_action_uses_kebab_type_and_key_field() {
        let action = LogicalAction::SystemAction {
            key: "volume_up".to_owned(),
        };
        let json = serde_json::to_value(&action).expect("serialize");
        assert_eq!(json["type"], "system-action");
        assert_eq!(json["key"], "volume_up");
        assert_eq!(
            json.as_object().expect("object").len(),
            2,
            "只应有 type/key 两个字段"
        );
        let round_trip: LogicalAction = serde_json::from_value(json).expect("deserialize");
        assert_eq!(round_trip, action);

        // 未知 key 在契约层原样保留（执行层解析失败，不在 JSON 校验期拒绝）。
        let unknown: LogicalAction = serde_json::from_value(
            serde_json::json!({"type": "system-action", "key": "not_a_key"}),
        )
        .expect("unknown key parses");
        assert_eq!(
            unknown,
            LogicalAction::SystemAction {
                key: "not_a_key".to_owned()
            }
        );
    }

    #[test]
    fn disabled_mapping_always_returns_disabled() {
        let mut mappings = ButtonMappingSet::default();
        mappings.enabled = false;
        mappings.set_actions(
            LogicalButton::Ok,
            ButtonActions {
                single: LogicalAction::Shortcut {
                    label: "Enter".to_owned(),
                    keys: vec!["Enter".to_owned()],
                },
                ..ButtonActions::default()
            },
        );
        assert!(mappings
            .action(LogicalButton::Ok, ButtonTrigger::Single)
            .is_disabled());
    }

    #[test]
    fn mapping_lookup_uses_requested_trigger() {
        let mut mappings = ButtonMappingSet::default();
        mappings.set_actions(
            LogicalButton::Ok,
            ButtonActions {
                single: LogicalAction::FocusInput,
                double: LogicalAction::OpenApp {
                    app: "wechat".to_owned(),
                    focus: FocusStrategy::Direct,
                },
                long: LogicalAction::Disabled,
                hold_repeat: None,
                ok_context_click: false,
            },
        );
        assert_eq!(
            mappings.action(LogicalButton::Ok, ButtonTrigger::Single),
            LogicalAction::FocusInput
        );
        assert_eq!(
            mappings.action(LogicalButton::Ok, ButtonTrigger::Double),
            LogicalAction::OpenApp {
                app: "wechat".to_owned(),
                focus: FocusStrategy::Direct,
            }
        );
    }
}
