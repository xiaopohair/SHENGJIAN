//! Windows adapters for the port boundary.
//!
//! The adapter translates the existing platform state into the language-neutral
//! contract. It does not own the BLE worker, audio worker or input hooks; those
//! stay in `shengjian-windows`.

mod legacy;

pub use legacy::{migrate_button_mappings, migrate_button_profiles_document};

use shengjian_app::{
    ActionSinkPort, AudioSinkPort, ButtonMappingStorePort, ButtonProfilesStorePort,
    ButtonSourcePort, CoreEngine, CoreEnginePorts, DeviceCatalog, EventSink, ForegroundPort,
    RemoteLinkPort, SettingsDocument, SettingsStorePort, VoiceSourcePort,
    SETTINGS_NAMESPACE_SCHEMA_VERSION,
};
use shengjian_contract::{
    AudioEndpoint as ContractAudioEndpoint, AudioSnapshot as ContractAudioSnapshot, AudioState,
    ButtonEdge as ContractButtonEdge, ButtonMappingSet, ButtonProfilesDocument, ConnectionPhase,
    ConnectionSnapshot, CoreResult, DeviceId, Direction, EngineError, ErrorDomain, Generation,
    LogicalAction, LogicalButton, MouseActionKind, RemoteCandidate, RemoteId, VoiceState,
    BUTTON_PROFILES_SCHEMA_VERSION,
};
use shengjian_domain::AppSettings;
use shengjian_windows::common_phrases::{
    current_common_phrase_host, CommonPhraseHost, PHRASE_SOURCE_DEFAULT,
};
use shengjian_windows::macro_runtime::MacroStartError;
use shengjian_windows::raw_input::RemoteButton;
use shengjian_windows::send_input::{
    ButtonAction as PlatformButtonAction, KeyChord, KeyCode, MouseClickKind, MoveDirection,
    ScrollDirection,
};
use shengjian_windows::{
    AudioEndpoint as PlatformAudioEndpoint, AudioPhase, AudioSnapshot as PlatformAudioSnapshot,
    ConnectionSnapshot as PlatformConnectionSnapshot, PairedRemote, PlatformError, RemoteModel,
    WindowsPlatform,
};
use std::collections::HashMap;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// The narrow slice of the Windows platform that the remote-link adapter needs.
///
/// It is a trait so the adapter can be tested without BLE hardware; the real
/// implementation is [`WindowsPlatform`].
pub trait PlatformRemote: Send + Sync {
    fn platform_snapshot(&self) -> PlatformConnectionSnapshot;
    fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError>;
    fn connect_remote(
        &self,
        device_id: String,
    ) -> Result<PlatformConnectionSnapshot, PlatformError>;
    fn disconnect_remote(&self) -> Result<PlatformConnectionSnapshot, PlatformError>;
}

impl PlatformRemote for WindowsPlatform {
    fn platform_snapshot(&self) -> PlatformConnectionSnapshot {
        WindowsPlatform::connection_snapshot(self)
    }

    fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError> {
        WindowsPlatform::scan_paired_remotes(self)
    }

    fn connect_remote(
        &self,
        device_id: String,
    ) -> Result<PlatformConnectionSnapshot, PlatformError> {
        WindowsPlatform::connect_remote(self, device_id)
    }

    fn disconnect_remote(&self) -> Result<PlatformConnectionSnapshot, PlatformError> {
        WindowsPlatform::disconnect_remote(self)
    }
}

pub struct WindowsRemoteLink<P: PlatformRemote = WindowsPlatform> {
    platform: Arc<P>,
    last_remote: Mutex<Option<RemoteId>>,
    platform_ids: Mutex<HashMap<RemoteId, String>>,
}

impl<P: PlatformRemote> WindowsRemoteLink<P> {
    pub fn new(platform: Arc<P>) -> Self {
        Self {
            platform,
            last_remote: Mutex::new(None),
            platform_ids: Mutex::new(HashMap::new()),
        }
    }

    fn last_remote(&self) -> Option<RemoteId> {
        self.last_remote.lock().expect("last remote").clone()
    }
}

impl WindowsRemoteLink<WindowsPlatform> {
    /// Construct a link over the real Windows platform.
    pub fn live() -> Self {
        Self::new(Arc::new(WindowsPlatform::default()))
    }
}

impl<P: PlatformRemote> RemoteLinkPort for WindowsRemoteLink<P> {
    fn snapshot(&self) -> ConnectionSnapshot {
        snapshot_to_contract(
            self.platform.platform_snapshot(),
            self.last_remote().as_ref(),
        )
    }

    fn scan(&self) -> CoreResult<Vec<RemoteCandidate>> {
        let remotes = self
            .platform
            .scan_paired_remotes()
            .map_err(map_platform_error)?;
        let mut platform_ids = self.platform_ids.lock().expect("platform ids");
        platform_ids.clear();
        Ok(remotes
            .into_iter()
            .map(|remote| {
                let opaque = opaque_remote_id(&remote.id, &remote.token);
                platform_ids.insert(opaque.clone(), remote.id.clone());
                candidate_from_platform(remote, opaque)
            })
            .collect())
    }

    fn connect(&self, remote_id: &RemoteId) -> CoreResult<ConnectionSnapshot> {
        // RemoteId is the opaque scan token; the platform device path is kept
        // only inside this adapter and never emitted in contract events.
        let platform_id = self
            .platform_ids
            .lock()
            .expect("platform ids")
            .get(remote_id)
            .cloned()
            .unwrap_or_else(|| remote_id.as_str().to_owned());
        match self.platform.connect_remote(platform_id) {
            Ok(snapshot) => {
                *self.last_remote.lock().expect("last remote") = Some(remote_id.clone());
                Ok(snapshot_to_contract(snapshot, Some(remote_id)))
            }
            Err(error) => Err(map_platform_error(error)),
        }
    }

    fn disconnect(&self) -> CoreResult<ConnectionSnapshot> {
        match self.platform.disconnect_remote() {
            Ok(snapshot) => {
                *self.last_remote.lock().expect("last remote") = None;
                Ok(snapshot_to_contract(snapshot, None))
            }
            Err(error) => Err(map_platform_error(error)),
        }
    }
}

fn opaque_remote_id(platform_id: &str, token: &str) -> RemoteId {
    if !token.trim().is_empty() {
        return RemoteId::new(token);
    }
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    platform_id.hash(&mut hasher);
    RemoteId::new(format!("remote-{:016x}", hasher.finish()))
}

fn candidate_from_platform(remote: PairedRemote, opaque_id: RemoteId) -> RemoteCandidate {
    let token = (!remote.token.is_empty()).then_some(remote.token.clone());
    RemoteCandidate {
        remote_id: opaque_id,
        name: remote.name,
        model: model_label(remote.model),
        device_id: device_id_for(remote.model),
        is_supported: remote.is_supported_candidate || remote.model != RemoteModel::Unknown,
        token,
    }
}

fn snapshot_to_contract(
    snapshot: PlatformConnectionSnapshot,
    last_remote: Option<&RemoteId>,
) -> ConnectionSnapshot {
    let remote = snapshot.remote_name.as_ref().map(|name| RemoteCandidate {
        remote_id: last_remote
            .cloned()
            .or_else(|| snapshot.remote_token.clone().map(RemoteId::new))
            .unwrap_or_else(|| RemoteId::new("connected")),
        name: name.clone(),
        model: model_label(snapshot.remote_model),
        device_id: device_id_for(snapshot.remote_model),
        is_supported: snapshot.remote_model != RemoteModel::Unknown,
        token: snapshot.remote_token.clone(),
    });
    ConnectionSnapshot {
        phase: phase_to_contract(snapshot.phase),
        remote,
        generation: Generation::from(snapshot.generation),
        last_error: snapshot
            .last_error
            .as_ref()
            .map(|_| EngineError::new(ErrorDomain::Bluetooth, "ble-error")),
    }
}

fn phase_to_contract(phase: shengjian_windows::ConnectionPhase) -> ConnectionPhase {
    use shengjian_windows::ConnectionPhase as PlatformPhase;
    match phase {
        PlatformPhase::Idle => ConnectionPhase::Idle,
        PlatformPhase::Connecting
        | PlatformPhase::Discovering
        | PlatformPhase::AwaitingCapabilities => ConnectionPhase::Connecting,
        PlatformPhase::Ready | PlatformPhase::Streaming | PlatformPhase::Draining => {
            ConnectionPhase::Ready
        }
        PlatformPhase::Reconnecting => ConnectionPhase::Reconnecting,
        PlatformPhase::Suspended | PlatformPhase::Disconnected => ConnectionPhase::Disconnected,
        PlatformPhase::Failed => ConnectionPhase::Failed,
    }
}

fn model_label(model: RemoteModel) -> Option<String> {
    match model {
        RemoteModel::Rc001 => Some("RC001".to_owned()),
        RemoteModel::Rc003 => Some("RC003".to_owned()),
        RemoteModel::Unknown => None,
    }
}

fn device_id_for(model: RemoteModel) -> Option<DeviceId> {
    match model {
        RemoteModel::Rc001 => Some(DeviceId::new("xiaomi-rc001")),
        RemoteModel::Rc003 => Some(DeviceId::new("xiaomi-rc003")),
        RemoteModel::Unknown => None,
    }
}

fn map_platform_error(error: PlatformError) -> EngineError {
    use ErrorDomain::{Audio, Bluetooth, Configuration, Input, Internal};
    let (domain, code, retryable) = match error {
        PlatformError::UnsupportedPlatform => (Configuration, "unsupported-platform", false),
        PlatformError::WindowsApi(_) => (Internal, "windows-api-failed", false),
        PlatformError::WorkerUnavailable => (Bluetooth, "worker-unavailable", true),
        PlatformError::OperationTimedOut => (Bluetooth, "operation-timeout", true),
        PlatformError::VoiceServiceMissing => (Bluetooth, "voice-service-missing", false),
        PlatformError::VoiceCharacteristicMissing(_) => {
            (Bluetooth, "voice-characteristic-missing", false)
        }
        PlatformError::Gatt(_) => (Bluetooth, "gatt-failed", true),
        PlatformError::GattUnreachable(_) => (Bluetooth, "gatt-unreachable", true),
        PlatformError::GattProtocolError(_) => (Bluetooth, "gatt-protocol-error", false),
        PlatformError::GattAccessDenied(_) => (Bluetooth, "gatt-access-denied", false),
        PlatformError::Protocol(_) => (Bluetooth, "protocol-failed", false),
        PlatformError::AudioWorkerUnavailable => (Audio, "audio-worker-unavailable", true),
        PlatformError::AudioOperationTimedOut => (Audio, "audio-operation-timeout", true),
        PlatformError::AudioEndpointNotSelected => {
            (Configuration, "audio-endpoint-not-selected", false)
        }
        PlatformError::AudioBusy => (Audio, "audio-busy", true),
        PlatformError::AudioSessionMismatch => (Audio, "audio-session-mismatch", true),
        PlatformError::AudioSessionInterrupted => (Audio, "audio-session-interrupted", true),
        PlatformError::AudioQueueOverflow => (Audio, "audio-queue-overflow", true),
        PlatformError::AudioDeviceInvalidated => (Audio, "audio-device-invalidated", true),
        PlatformError::Audio(_) => (Audio, "audio-failed", false),
        PlatformError::BleCleanup(_) => (Bluetooth, "ble-cleanup-failed", true),
        PlatformError::RawInput(_) => (Input, "raw-input-failed", false),
        PlatformError::SendInput(_) => (Input, "send-input-failed", false),
    };
    EngineError {
        domain,
        code: code.to_owned(),
        retryable,
    }
}

/// The audio slice of the Windows platform used by [`WindowsAudioSink`].
///
/// Kept separate from [`PlatformRemote`] so a fake can test audio mapping
/// without a BLE backend.
pub trait PlatformAudio: Send + Sync {
    fn platform_audio_snapshot(&self) -> PlatformAudioSnapshot;
    fn list_audio_endpoints(&self) -> Result<Vec<PlatformAudioEndpoint>, PlatformError>;
    fn select_audio_endpoint(
        &self,
        endpoint_id: String,
    ) -> Result<PlatformAudioSnapshot, PlatformError>;
    fn set_gain_db(&self, gain_db: f32);
    fn gain_db(&self) -> f32;
}

impl PlatformAudio for WindowsPlatform {
    fn platform_audio_snapshot(&self) -> PlatformAudioSnapshot {
        WindowsPlatform::audio_snapshot(self)
    }

    fn list_audio_endpoints(&self) -> Result<Vec<PlatformAudioEndpoint>, PlatformError> {
        WindowsPlatform::list_audio_endpoints(self)
    }

    fn select_audio_endpoint(
        &self,
        endpoint_id: String,
    ) -> Result<PlatformAudioSnapshot, PlatformError> {
        WindowsPlatform::select_audio_endpoint(self, endpoint_id)
    }

    fn set_gain_db(&self, gain_db: f32) {
        WindowsPlatform::set_gain_db(self, gain_db);
    }

    fn gain_db(&self) -> f32 {
        WindowsPlatform::gain_db(self)
    }
}

pub struct WindowsAudioSink<P: PlatformAudio = WindowsPlatform> {
    platform: Arc<P>,
}

impl<P: PlatformAudio> WindowsAudioSink<P> {
    pub fn new(platform: Arc<P>) -> Self {
        Self { platform }
    }
}

impl WindowsAudioSink<WindowsPlatform> {
    pub fn live() -> Self {
        Self::new(Arc::new(WindowsPlatform::default()))
    }
}

impl<P: PlatformAudio> AudioSinkPort for WindowsAudioSink<P> {
    fn snapshot(&self) -> ContractAudioSnapshot {
        audio_snapshot_to_contract(
            self.platform.platform_audio_snapshot(),
            self.platform.gain_db(),
        )
    }

    fn list_endpoints(&self) -> CoreResult<Vec<ContractAudioEndpoint>> {
        self.platform
            .list_audio_endpoints()
            .map(|endpoints| {
                endpoints
                    .into_iter()
                    .map(audio_endpoint_to_contract)
                    .collect()
            })
            .map_err(map_platform_error)
    }

    fn select_endpoint(&self, endpoint_id: &str) -> CoreResult<ContractAudioSnapshot> {
        self.platform
            .select_audio_endpoint(endpoint_id.to_owned())
            .map(|snapshot| audio_snapshot_to_contract(snapshot, self.platform.gain_db()))
            .map_err(map_platform_error)
    }

    fn set_gain_db(&self, gain_db: i16) -> CoreResult<ContractAudioSnapshot> {
        self.platform.set_gain_db(f32::from(gain_db));
        Ok(audio_snapshot_to_contract(
            self.platform.platform_audio_snapshot(),
            self.platform.gain_db(),
        ))
    }
}

fn audio_endpoint_to_contract(endpoint: PlatformAudioEndpoint) -> ContractAudioEndpoint {
    ContractAudioEndpoint {
        id: endpoint.id,
        name: endpoint.name,
        recommended: endpoint.is_virtual_cable_candidate,
    }
}

fn audio_snapshot_to_contract(
    snapshot: PlatformAudioSnapshot,
    gain_db: f32,
) -> ContractAudioSnapshot {
    let endpoint = snapshot
        .selected_endpoint_id
        .as_ref()
        .map(|id| ContractAudioEndpoint {
            id: id.clone(),
            name: snapshot
                .selected_endpoint_name
                .clone()
                .unwrap_or_else(|| id.clone()),
            recommended: snapshot
                .selected_endpoint_name
                .as_deref()
                .is_some_and(is_virtual_cable_name),
        });
    ContractAudioSnapshot {
        state: audio_state_to_contract(snapshot.phase),
        endpoint,
        format: None,
        gain_db: gain_db.round().clamp(0.0, 24.0) as i16,
        generation: Generation::from(snapshot.generation),
        last_error: snapshot
            .last_error
            .as_ref()
            .map(|_| EngineError::new(ErrorDomain::Audio, "audio-error")),
    }
}

fn audio_state_to_contract(phase: AudioPhase) -> AudioState {
    match phase {
        AudioPhase::Unconfigured => AudioState::Unconfigured,
        AudioPhase::Ready => AudioState::Ready,
        AudioPhase::Streaming => AudioState::Streaming,
        AudioPhase::Draining => AudioState::Draining,
        AudioPhase::Failed => AudioState::Failed,
        AudioPhase::Unsupported => AudioState::Unsupported,
    }
}

fn is_virtual_cable_name(name: &str) -> bool {
    name.to_ascii_lowercase().contains("cable")
}

/// Action execution slice of the Windows platform.
///
/// It accepts contract-level requests so the adapter can be tested without
/// SendInput/UIA. The real backend maps each request to the existing platform
/// APIs; it does not reimplement injection or focus logic.
pub trait PlatformActions: Send + Sync {
    fn shortcut(&self, keys: &[String]) -> CoreResult<()>;
    fn focus_input(&self) -> CoreResult<()>;
    fn open_app(&self, target: &str) -> CoreResult<()>;
    fn run_macro(&self, macro_id: &str) -> CoreResult<()>;
    fn scroll(&self, direction: Direction, steps: u16) -> CoreResult<()>;
    fn move_mouse(&self, direction: Direction, distance: u16) -> CoreResult<()>;
    fn click_mouse(&self) -> CoreResult<()>;
    fn open_common_phrases(&self) -> CoreResult<()>;
}

pub struct WindowsPlatformActions {
    platform: Arc<WindowsPlatform>,
}

impl WindowsPlatformActions {
    pub fn new(platform: Arc<WindowsPlatform>) -> Self {
        Self { platform }
    }

    pub fn live() -> Self {
        Self::new(Arc::new(WindowsPlatform::default()))
    }
}

impl PlatformActions for WindowsPlatformActions {
    fn shortcut(&self, keys: &[String]) -> CoreResult<()> {
        let chord = parse_chord(keys)?;
        self.platform
            .test_shortcut(chord)
            .map(|_| ())
            .map_err(map_platform_error)
    }

    fn focus_input(&self) -> CoreResult<()> {
        self.platform.test_focus_input().map_err(map_platform_error)
    }

    fn open_app(&self, target: &str) -> CoreResult<()> {
        if target.trim().is_empty() {
            return Err(config_error("empty-app-target"));
        }
        self.platform
            .test_app_focus(target)
            .map_err(map_platform_error)
    }

    fn run_macro(&self, macro_id: &str) -> CoreResult<()> {
        self.platform
            .macro_runtime()
            .start(macro_id)
            .map_err(map_macro_error)
    }

    fn scroll(&self, direction: Direction, steps: u16) -> CoreResult<()> {
        let direction = match direction {
            Direction::Up => ScrollDirection::Up,
            Direction::Down => ScrollDirection::Down,
            Direction::Left | Direction::Right => {
                return Err(config_error("scroll-direction-must-be-vertical"));
            }
        };
        self.platform
            .test_scroll(direction, steps)
            .map(|_| ())
            .map_err(map_platform_error)
    }

    fn move_mouse(&self, direction: Direction, distance: u16) -> CoreResult<()> {
        let direction = match direction {
            Direction::Up => MoveDirection::Up,
            Direction::Down => MoveDirection::Down,
            Direction::Left => MoveDirection::Left,
            Direction::Right => MoveDirection::Right,
        };
        self.platform
            .test_mouse_action(PlatformButtonAction::MouseMove {
                direction,
                distance,
            })
            .map(|_| ())
            .map_err(map_platform_error)
    }

    fn click_mouse(&self) -> CoreResult<()> {
        self.platform
            .test_mouse_action(PlatformButtonAction::MouseClick {
                kind: MouseClickKind::Left,
            })
            .map(|_| ())
            .map_err(map_platform_error)
    }

    fn open_common_phrases(&self) -> CoreResult<()> {
        let host: Arc<dyn CommonPhraseHost> = current_common_phrase_host();
        if host.open(PHRASE_SOURCE_DEFAULT) {
            Ok(())
        } else {
            Err(EngineError::new(
                ErrorDomain::Configuration,
                "common-phrases-unavailable",
            ))
        }
    }
}

pub struct WindowsActionSink<P: PlatformActions = WindowsPlatformActions> {
    backend: Arc<P>,
}

impl<P: PlatformActions> WindowsActionSink<P> {
    pub fn new(backend: Arc<P>) -> Self {
        Self { backend }
    }
}

impl WindowsActionSink<WindowsPlatformActions> {
    pub fn live() -> Self {
        Self::new(Arc::new(WindowsPlatformActions::live()))
    }
}

impl<P: PlatformActions> ActionSinkPort for WindowsActionSink<P> {
    fn execute(&self, action: &LogicalAction) -> CoreResult<()> {
        match action {
            LogicalAction::Disabled => Ok(()),
            LogicalAction::Shortcut { keys, .. } => self.backend.shortcut(keys),
            LogicalAction::OpenApp { app, .. } => self.backend.open_app(app),
            LogicalAction::FocusInput => self.backend.focus_input(),
            LogicalAction::RunMacro { macro_id } => self.backend.run_macro(macro_id),
            LogicalAction::CommonPhrase => self.backend.open_common_phrases(),
            LogicalAction::Mouse {
                kind,
                direction,
                amount,
            } => self.execute_mouse(*kind, *direction, *amount),
        }
    }
}

impl<P: PlatformActions> WindowsActionSink<P> {
    fn execute_mouse(
        &self,
        kind: MouseActionKind,
        direction: Option<Direction>,
        amount: u16,
    ) -> CoreResult<()> {
        match kind {
            MouseActionKind::Click => self.backend.click_mouse(),
            MouseActionKind::Scroll | MouseActionKind::Move => {
                if !(1..=100).contains(&amount) {
                    return Err(config_error("mouse-amount-out-of-range"));
                }
                let direction =
                    direction.ok_or_else(|| config_error("mouse-direction-required"))?;
                match kind {
                    MouseActionKind::Scroll => self.backend.scroll(direction, amount),
                    MouseActionKind::Move => self.backend.move_mouse(direction, amount),
                    MouseActionKind::Click => unreachable!(),
                }
            }
        }
    }
}

fn config_error(code: &str) -> EngineError {
    EngineError::new(ErrorDomain::Configuration, code)
}

fn parse_chord(keys: &[String]) -> CoreResult<KeyChord> {
    let mut parsed = Vec::new();
    for raw in keys {
        for part in raw.split('+') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            parsed.push(parse_key(part)?);
        }
    }
    if parsed.is_empty() {
        return Err(config_error("shortcut-empty"));
    }
    Ok(KeyChord { keys: parsed })
}

fn parse_key(value: &str) -> CoreResult<KeyCode> {
    let normalized = value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '_', '-'], "");
    let key = match normalized.as_str() {
        "ctrl" | "control" => KeyCode::Control,
        "leftctrl" | "leftcontrol" => KeyCode::LeftControl,
        "rightctrl" | "rightcontrol" => KeyCode::RightControl,
        "shift" => KeyCode::Shift,
        "leftshift" => KeyCode::LeftShift,
        "rightshift" => KeyCode::RightShift,
        "alt" => KeyCode::Alt,
        "leftalt" => KeyCode::LeftAlt,
        "rightalt" => KeyCode::RightAlt,
        "win" | "windows" | "meta" | "leftwin" | "leftwindows" => KeyCode::LeftWindows,
        "rightwin" | "rightwindows" => KeyCode::RightWindows,
        "enter" | "return" => KeyCode::Enter,
        "esc" | "escape" => KeyCode::Escape,
        "space" => KeyCode::Space,
        "tab" => KeyCode::Tab,
        "backspace" => KeyCode::Backspace,
        "delete" | "del" => KeyCode::Delete,
        "insert" | "ins" => KeyCode::Insert,
        "home" => KeyCode::Home,
        "end" => KeyCode::End,
        "pageup" | "pgup" => KeyCode::PageUp,
        "pagedown" | "pgdn" => KeyCode::PageDown,
        "up" => KeyCode::Up,
        "down" => KeyCode::Down,
        "left" => KeyCode::Left,
        "right" => KeyCode::Right,
        "apps" | "menu" | "contextmenu" => KeyCode::Apps,
        "volumemute" | "mute" => KeyCode::VolumeMute,
        "volumedown" => KeyCode::VolumeDown,
        "volumeup" => KeyCode::VolumeUp,
        "mediaprev" | "previous" => KeyCode::MediaPrev,
        "medianext" | "next" => KeyCode::MediaNext,
        "mediaplaypause" | "playpause" => KeyCode::MediaPlayPause,
        "numpad0" => KeyCode::Numpad0,
        "numpad1" => KeyCode::Numpad1,
        "numpad2" => KeyCode::Numpad2,
        "numpad3" => KeyCode::Numpad3,
        "numpad4" => KeyCode::Numpad4,
        "numpad5" => KeyCode::Numpad5,
        "numpad6" => KeyCode::Numpad6,
        "numpad7" => KeyCode::Numpad7,
        "numpad8" => KeyCode::Numpad8,
        "numpad9" => KeyCode::Numpad9,
        "numpadmultiply" => KeyCode::NumpadMultiply,
        "numpadadd" => KeyCode::NumpadAdd,
        "numpadsubtract" => KeyCode::NumpadSubtract,
        "numpaddecimal" => KeyCode::NumpadDecimal,
        "numpaddivide" => KeyCode::NumpadDivide,
        "numlock" => KeyCode::NumLock,
        "digit0" => KeyCode::Digit0,
        "digit1" => KeyCode::Digit1,
        "digit2" => KeyCode::Digit2,
        "digit3" => KeyCode::Digit3,
        "digit4" => KeyCode::Digit4,
        "digit5" => KeyCode::Digit5,
        "digit6" => KeyCode::Digit6,
        "digit7" => KeyCode::Digit7,
        "digit8" => KeyCode::Digit8,
        "digit9" => KeyCode::Digit9,
        ";" | "semicolon" | "oem1" => KeyCode::Oem1,
        "=" | "equal" | "plus" | "oemplus" => KeyCode::OemPlus,
        "," | "comma" | "oemcomma" => KeyCode::OemComma,
        "-" | "minus" | "oemminus" => KeyCode::OemMinus,
        "." | "period" | "oemperiod" => KeyCode::OemPeriod,
        "/" | "slash" | "oem2" => KeyCode::Oem2,
        "`" | "backquote" | "grave" | "oem3" => KeyCode::Oem3,
        "[" | "leftbracket" | "oem4" => KeyCode::Oem4,
        "\\" | "backslash" | "oem5" => KeyCode::Oem5,
        "]" | "rightbracket" | "oem6" => KeyCode::Oem6,
        "'" | "quote" | "apostrophe" | "oem7" => KeyCode::Oem7,
        "oem102" => KeyCode::Oem102,
        value if value.len() == 1 && value.chars().all(|ch| ch.is_ascii_alphabetic()) => {
            return letter_key(value.chars().next().expect("one char"));
        }
        value if value.len() == 1 && value.chars().all(|ch| ch.is_ascii_digit()) => {
            return digit_key(value.chars().next().expect("one digit"));
        }
        value if value.starts_with('f') && value[1..].parse::<u8>().is_ok() => {
            return function_key(value[1..].parse().expect("parsed"));
        }
        _ => return Err(config_error("unsupported-key")),
    };
    Ok(key)
}

fn letter_key(ch: char) -> CoreResult<KeyCode> {
    use KeyCode::*;
    let key = match ch {
        'a' => A,
        'b' => B,
        'c' => C,
        'd' => D,
        'e' => E,
        'f' => F,
        'g' => G,
        'h' => H,
        'i' => I,
        'j' => J,
        'k' => K,
        'l' => L,
        'm' => M,
        'n' => N,
        'o' => O,
        'p' => P,
        'q' => Q,
        'r' => R,
        's' => S,
        't' => T,
        'u' => U,
        'v' => V,
        'w' => W,
        'x' => X,
        'y' => Y,
        'z' => Z,
        _ => return Err(config_error("unsupported-key")),
    };
    Ok(key)
}

fn digit_key(ch: char) -> CoreResult<KeyCode> {
    use KeyCode::*;
    let key = match ch {
        '0' => Digit0,
        '1' => Digit1,
        '2' => Digit2,
        '3' => Digit3,
        '4' => Digit4,
        '5' => Digit5,
        '6' => Digit6,
        '7' => Digit7,
        '8' => Digit8,
        '9' => Digit9,
        _ => return Err(config_error("unsupported-key")),
    };
    Ok(key)
}

fn function_key(number: u8) -> CoreResult<KeyCode> {
    use KeyCode::*;
    let key = match number {
        1 => F1,
        2 => F2,
        3 => F3,
        4 => F4,
        5 => F5,
        6 => F6,
        7 => F7,
        8 => F8,
        9 => F9,
        10 => F10,
        11 => F11,
        12 => F12,
        _ => return Err(config_error("unsupported-key")),
    };
    Ok(key)
}

fn map_macro_error(error: MacroStartError) -> EngineError {
    let code = match error {
        MacroStartError::AlreadyRunning => "macro-already-running",
        MacroStartError::NotFound => "macro-not-found",
        MacroStartError::Invalid(_) => "macro-invalid",
    };
    EngineError::new(ErrorDomain::Input, code)
}

/// Semantic button edges from the platform, before gesture recognition.
pub trait PlatformButtonSource: Send + Sync {
    fn subscribe_platform_edges(&self, callback: Arc<dyn Fn(RemoteButton, bool) + Send + Sync>);
}

impl PlatformButtonSource for WindowsPlatform {
    fn subscribe_platform_edges(&self, callback: Arc<dyn Fn(RemoteButton, bool) + Send + Sync>) {
        self.subscribe_button_edges(Arc::new(move |edge| {
            callback(edge.button, edge.is_pressed);
        }));
    }
}

pub struct WindowsButtonSource<P: PlatformButtonSource = WindowsPlatform> {
    platform: Arc<P>,
}

impl<P: PlatformButtonSource> WindowsButtonSource<P> {
    pub fn new(platform: Arc<P>) -> Self {
        Self { platform }
    }
}

impl WindowsButtonSource<WindowsPlatform> {
    pub fn live() -> Self {
        Self::new(Arc::new(WindowsPlatform::default()))
    }
}

impl<P: PlatformButtonSource> ButtonSourcePort for WindowsButtonSource<P> {
    fn subscribe_edges(&self, sink: Arc<dyn Fn(ContractButtonEdge) + Send + Sync>) {
        self.platform
            .subscribe_platform_edges(Arc::new(move |button, pressed| {
                sink(ContractButtonEdge {
                    button: map_button(button),
                    pressed,
                    at_ms: now_ms(),
                });
            }));
    }
}

pub(crate) fn map_button(button: RemoteButton) -> LogicalButton {
    match button {
        RemoteButton::Back => LogicalButton::Back,
        RemoteButton::Ok => LogicalButton::Ok,
        RemoteButton::Tv => LogicalButton::Tv,
        RemoteButton::Home => LogicalButton::Home,
        RemoteButton::Right => LogicalButton::Right,
        RemoteButton::Left => LogicalButton::Left,
        RemoteButton::Down => LogicalButton::Down,
        RemoteButton::Up => LogicalButton::Up,
        RemoteButton::Menu => LogicalButton::Menu,
        RemoteButton::Power => LogicalButton::Power,
        RemoteButton::VolumeMute => LogicalButton::VolumeMute,
        RemoteButton::VolumeUp => LogicalButton::VolumeUp,
        RemoteButton::VolumeDown => LogicalButton::VolumeDown,
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

/// Foreground executable provider for profile auto-apply.
pub struct WindowsForeground;

impl ForegroundPort for WindowsForeground {
    fn executable(&self) -> Option<String> {
        #[cfg(windows)]
        {
            shengjian_windows::focus_windows::foreground_executable_path()
                .and_then(|path| path.rsplit(['\\', '/']).next().map(str::to_owned))
                .filter(|name| !name.is_empty())
        }
        #[cfg(not(windows))]
        {
            None
        }
    }
}

/// Polled platform voice state for the live engine ticker.
pub struct WindowsVoiceSource {
    platform: Arc<WindowsPlatform>,
}

impl WindowsVoiceSource {
    pub fn new(platform: Arc<WindowsPlatform>) -> Self {
        Self { platform }
    }

    pub fn live() -> Self {
        Self::new(Arc::new(WindowsPlatform::default()))
    }
}

impl VoiceSourcePort for WindowsVoiceSource {
    fn state(&self) -> VoiceState {
        match self.platform.connection_snapshot().voice_state {
            shengjian_domain::VoiceSessionState::Idle => VoiceState::Idle,
            shengjian_domain::VoiceSessionState::Streaming => VoiceState::Streaming,
            shengjian_domain::VoiceSessionState::Draining => VoiceState::Draining,
        }
    }
}

/// File-backed settings store for the new neutral namespace.
///
/// New writes are a `SettingsDocument` (`global` + `devices.<id>.<platform>`).
/// Reading a legacy flat `AppSettings` file is supported and becomes `global`
/// with no device overrides.
pub struct WindowsSettingsStore {
    path: PathBuf,
}

impl WindowsSettingsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn live() -> Self {
        Self::new(default_settings_path())
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn write_document(&self, document: &SettingsDocument) -> CoreResult<()> {
        let contents = serde_json::to_string_pretty(&document.clone().normalized())
            .map_err(|_| config_error("settings-serialize-failed"))?;
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|_| config_error("settings-mkdir-failed"))?;
        }
        let temp = self
            .path
            .with_extension(format!("json.{}.tmp", std::process::id()));
        fs::write(&temp, contents).map_err(|_| config_error("settings-write-failed"))?;
        if fs::rename(&temp, &self.path).is_err() {
            // Some Windows filesystems/handles can reject rename-over-existing;
            // fall back to copy + remove so settings remain writable.
            if fs::copy(&temp, &self.path).is_err() {
                let _ = fs::remove_file(&temp);
                return Err(config_error("settings-commit-failed"));
            }
            let _ = fs::remove_file(&temp);
        }
        Ok(())
    }
}

impl SettingsStorePort for WindowsSettingsStore {
    fn load(&self) -> CoreResult<AppSettings> {
        let document = self.load_document()?;
        let selected = document.global.selected_remote_id.clone();
        Ok(document.resolve(selected.as_deref(), "windows"))
    }

    fn save(&self, settings: &AppSettings) -> CoreResult<()> {
        let mut document = self
            .load_document()
            .unwrap_or_else(|_| SettingsDocument::from_flat(AppSettings::default()));
        document.global = settings.clone();
        self.write_document(&document)
    }

    fn load_document(&self) -> CoreResult<SettingsDocument> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                if let Ok(document) = serde_json::from_str::<SettingsDocument>(&contents) {
                    if document.schema_version > SETTINGS_NAMESPACE_SCHEMA_VERSION {
                        let _ = backup_for_rejected_schema(&self.path);
                        return Err(config_error("settings-newer-schema"));
                    }
                    return Ok(document.normalized());
                }
                if let Ok(settings) = serde_json::from_str::<AppSettings>(&contents) {
                    return Ok(SettingsDocument::from_flat(settings).normalized());
                }
                Err(config_error("settings-invalid"))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Ok(SettingsDocument::from_flat(AppSettings::default()))
            }
            Err(_) => Err(config_error("settings-read-failed")),
        }
    }

    fn save_document(&self, document: &SettingsDocument) -> CoreResult<()> {
        self.write_document(document)
    }
}

fn default_settings_path() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Shengjian").join("settings.json")
}

pub struct WindowsProfileStore {
    path: PathBuf,
}

impl WindowsProfileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn live() -> Self {
        Self::new(default_core_path("button-profiles.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ButtonProfilesStorePort for WindowsProfileStore {
    fn load(&self) -> CoreResult<Option<ButtonProfilesDocument>> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => {
                let document = serde_json::from_str::<ButtonProfilesDocument>(&contents)
                    .map_err(|_| config_error("profile-store-invalid"))?;
                if document.schema_version > BUTTON_PROFILES_SCHEMA_VERSION {
                    let _ = backup_for_rejected_schema(&self.path);
                    return Err(config_error("profile-store-newer-schema"));
                }
                Ok(Some(document.normalized()))
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(config_error("profile-store-read-failed")),
        }
    }

    fn save(&self, document: &ButtonProfilesDocument) -> CoreResult<()> {
        let contents = serde_json::to_string_pretty(&document.clone().normalized())
            .map_err(|_| config_error("profile-store-serialize-failed"))?;
        write_json_atomic(&self.path, contents, "profile-store")
    }
}

pub struct WindowsMappingStore {
    path: PathBuf,
}

impl WindowsMappingStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn live() -> Self {
        Self::new(default_core_path("button-mappings.json"))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl ButtonMappingStorePort for WindowsMappingStore {
    fn load(&self) -> CoreResult<Option<ButtonMappingSet>> {
        match fs::read_to_string(&self.path) {
            Ok(contents) => serde_json::from_str(&contents)
                .map(Some)
                .map_err(|_| config_error("mapping-store-invalid")),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(_) => Err(config_error("mapping-store-read-failed")),
        }
    }

    fn save(&self, mappings: &ButtonMappingSet) -> CoreResult<()> {
        let contents = serde_json::to_string_pretty(mappings)
            .map_err(|_| config_error("mapping-store-serialize-failed"))?;
        write_json_atomic(&self.path, contents, "mapping-store")
    }
}

fn default_core_path(file: &str) -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("Shengjian").join(file)
}

fn write_json_atomic(path: &Path, contents: String, prefix: &str) -> CoreResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| config_error(&format!("{prefix}-mkdir-failed")))?;
    }
    let temp = path.with_extension(format!(
        "json.{}.{}.tmp",
        std::process::id(),
        TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&temp, contents).map_err(|_| config_error(&format!("{prefix}-write-failed")))?;
    if fs::rename(&temp, path).is_err() {
        if fs::copy(&temp, path).is_err() {
            let _ = fs::remove_file(&temp);
            return Err(config_error(&format!("{prefix}-commit-failed")));
        }
        let _ = fs::remove_file(&temp);
    }
    Ok(())
}

fn backup_for_rejected_schema(path: &Path) -> std::io::Result<()> {
    fs::copy(path, path.with_extension("json.bak"))?;
    Ok(())
}

/// Build a live in-process core engine with one shared `WindowsPlatform`.
///
/// This is intentionally a convenience for the Windows host. It starts all
/// platform subsystems (BLE worker, audio runtime, input hooks), so callers
/// should create exactly one engine per process.
pub fn live_core_engine(sink: Arc<dyn EventSink>) -> CoreResult<CoreEngine> {
    Ok(live_core_engine_with_platform(sink)?.0)
}

/// Same as [`live_core_engine`], but also returns the shared platform for
/// diagnostics/smoke probes.
pub fn live_core_engine_with_platform(
    sink: Arc<dyn EventSink>,
) -> CoreResult<(CoreEngine, Arc<WindowsPlatform>)> {
    let platform = Arc::new(WindowsPlatform::default());
    // WindowsPlatform::default creates the runtimes but does not start Raw
    // Input; the original host did that explicitly at app boot. Without this,
    // button edges never reach ButtonSourcePort.
    platform.start_raw_input().map_err(map_platform_error)?;
    let catalog = DeviceCatalog::bundled().map_err(|_| config_error("profile-catalog-invalid"))?;
    let remote = Arc::new(WindowsRemoteLink::new(Arc::clone(&platform)));
    let audio = Arc::new(WindowsAudioSink::new(Arc::clone(&platform)));
    let actions = Arc::new(WindowsActionSink::new(Arc::new(
        WindowsPlatformActions::new(Arc::clone(&platform)),
    )));
    let settings = Arc::new(WindowsSettingsStore::live());
    let engine = CoreEngine::new(
        sink,
        CoreEnginePorts {
            remote,
            audio,
            actions,
            settings,
            foreground: Arc::new(WindowsForeground),
            voice: Some(Arc::new(WindowsVoiceSource::new(Arc::clone(&platform)))),
        },
        catalog,
    );
    engine.wire_profile_store(Arc::new(WindowsProfileStore::live()));
    engine.wire_mapping_store(Arc::new(WindowsMappingStore::live()));
    engine.load_persisted_button_config();
    engine.wire_button_source(Arc::new(WindowsButtonSource::new(Arc::clone(&platform))));
    Ok((engine, platform))
}

#[cfg(test)]
mod tests {
    use super::*;
    use shengjian_contract::{ConnectionPhase, ErrorDomain};
    use shengjian_windows::{ConnectionPhase as PlatformPhase, PlatformError, RemoteModel};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakePlatform {
        snapshot: Mutex<PlatformConnectionSnapshot>,
        remotes: Mutex<Vec<PairedRemote>>,
        connect_error: Mutex<Option<PlatformError>>,
    }

    impl FakePlatform {
        fn with_remote(mut self, remote: PairedRemote) -> Self {
            self.remotes.get_mut().expect("remotes").push(remote);
            self
        }
    }

    impl PlatformRemote for FakePlatform {
        fn platform_snapshot(&self) -> PlatformConnectionSnapshot {
            self.snapshot.lock().expect("snapshot").clone()
        }

        fn scan_paired_remotes(&self) -> Result<Vec<PairedRemote>, PlatformError> {
            Ok(self.remotes.lock().expect("remotes").clone())
        }

        fn connect_remote(
            &self,
            _device_id: String,
        ) -> Result<PlatformConnectionSnapshot, PlatformError> {
            if let Some(error) = self.connect_error.lock().expect("error").take() {
                return Err(error);
            }
            let mut snapshot = self.snapshot.lock().expect("snapshot");
            snapshot.phase = PlatformPhase::Ready;
            snapshot.remote_name = Some("Mi Remote".to_owned());
            snapshot.remote_model = RemoteModel::Rc003;
            snapshot.remote_token = Some("token-1".to_owned());
            snapshot.generation += 1;
            Ok(snapshot.clone())
        }

        fn disconnect_remote(&self) -> Result<PlatformConnectionSnapshot, PlatformError> {
            let mut snapshot = self.snapshot.lock().expect("snapshot");
            snapshot.phase = PlatformPhase::Disconnected;
            snapshot.remote_name = None;
            snapshot.remote_model = RemoteModel::Unknown;
            snapshot.remote_token = None;
            snapshot.generation += 1;
            Ok(snapshot.clone())
        }
    }

    fn paired_remote() -> PairedRemote {
        PairedRemote {
            id: "device-1".to_owned(),
            name: "Mi Remote".to_owned(),
            model: RemoteModel::Rc003,
            is_supported_candidate: true,
            token: "token-1".to_owned(),
        }
    }

    #[test]
    fn scan_maps_platform_identity_to_contract() {
        let platform = FakePlatform::default().with_remote(paired_remote());
        let link = WindowsRemoteLink::new(Arc::new(platform));
        let candidates = link.scan().expect("scan");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].remote_id.as_str(), "token-1");
        assert_eq!(candidates[0].model.as_deref(), Some("RC003"));
        assert_eq!(
            candidates[0].device_id.as_ref().map(DeviceId::as_str),
            Some("xiaomi-rc003")
        );
        assert_eq!(candidates[0].token.as_deref(), Some("token-1"));
    }

    #[test]
    fn connect_maps_snapshot_and_reuses_remote_id() {
        let platform = FakePlatform::default().with_remote(paired_remote());
        let link = WindowsRemoteLink::new(Arc::new(platform));
        let snapshot = link.connect(&RemoteId::new("device-1")).expect("connect");
        assert_eq!(snapshot.phase, ConnectionPhase::Ready);
        assert_eq!(snapshot.generation.value(), 1);
        let remote = snapshot.remote.expect("remote");
        assert_eq!(remote.remote_id.as_str(), "device-1");
        assert_eq!(remote.model.as_deref(), Some("RC003"));
    }

    #[test]
    fn disconnect_clears_remote_and_advances_generation() {
        let platform = FakePlatform::default().with_remote(paired_remote());
        let link = WindowsRemoteLink::new(Arc::new(platform));
        link.connect(&RemoteId::new("device-1")).expect("connect");
        let snapshot = link.disconnect().expect("disconnect");
        assert_eq!(snapshot.phase, ConnectionPhase::Disconnected);
        assert!(snapshot.remote.is_none());
        assert_eq!(snapshot.generation.value(), 2);
    }

    #[test]
    fn platform_timeout_becomes_a_retryable_bluetooth_error() {
        let platform = FakePlatform::default();
        *platform.connect_error.lock().expect("error") = Some(PlatformError::OperationTimedOut);
        let link = WindowsRemoteLink::new(Arc::new(platform));
        let error = link
            .connect(&RemoteId::new("device-1"))
            .expect_err("must fail");
        assert_eq!(error.domain, ErrorDomain::Bluetooth);
        assert_eq!(error.code, "operation-timeout");
        assert!(error.retryable);
    }
}

#[cfg(test)]
mod audio_tests {
    use super::*;
    use shengjian_contract::{AudioState, ErrorDomain};
    use shengjian_windows::{
        AudioEndpoint as PlatformAudioEndpoint, AudioPhase, AudioSnapshot as PlatformAudioSnapshot,
        PlatformError,
    };
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeAudio {
        snapshot: Mutex<PlatformAudioSnapshot>,
        endpoints: Mutex<Vec<PlatformAudioEndpoint>>,
        gain: Mutex<f32>,
        select_error: Mutex<Option<PlatformError>>,
    }

    impl FakeAudio {
        fn with_endpoints() -> Self {
            let fake = Self::default();
            *fake.endpoints.lock().expect("endpoints") = vec![
                PlatformAudioEndpoint {
                    id: "cable-id".to_owned(),
                    name: "CABLE Input".to_owned(),
                    is_virtual_cable_candidate: true,
                },
                PlatformAudioEndpoint {
                    id: "speaker-id".to_owned(),
                    name: "Speakers".to_owned(),
                    is_virtual_cable_candidate: false,
                },
            ];
            fake
        }
    }

    impl PlatformAudio for FakeAudio {
        fn platform_audio_snapshot(&self) -> PlatformAudioSnapshot {
            self.snapshot.lock().expect("snapshot").clone()
        }

        fn list_audio_endpoints(&self) -> Result<Vec<PlatformAudioEndpoint>, PlatformError> {
            Ok(self.endpoints.lock().expect("endpoints").clone())
        }

        fn select_audio_endpoint(
            &self,
            endpoint_id: String,
        ) -> Result<PlatformAudioSnapshot, PlatformError> {
            if let Some(error) = self.select_error.lock().expect("error").take() {
                return Err(error);
            }
            let endpoint = self
                .endpoints
                .lock()
                .expect("endpoints")
                .iter()
                .find(|endpoint| endpoint.id == endpoint_id)
                .cloned()
                .ok_or_else(|| PlatformError::Audio("endpoint not found".to_owned()))?;
            let mut snapshot = self.snapshot.lock().expect("snapshot");
            snapshot.phase = AudioPhase::Ready;
            snapshot.selected_endpoint_id = Some(endpoint.id);
            snapshot.selected_endpoint_name = Some(endpoint.name);
            snapshot.generation += 1;
            Ok(snapshot.clone())
        }

        fn set_gain_db(&self, gain_db: f32) {
            *self.gain.lock().expect("gain") = gain_db;
        }

        fn gain_db(&self) -> f32 {
            *self.gain.lock().expect("gain")
        }
    }

    #[test]
    fn snapshot_maps_phase_endpoint_and_gain() {
        let fake = FakeAudio {
            snapshot: Mutex::new(PlatformAudioSnapshot {
                phase: AudioPhase::Ready,
                selected_endpoint_id: Some("cable-id".to_owned()),
                selected_endpoint_name: Some("CABLE Input".to_owned()),
                generation: 3,
                ..PlatformAudioSnapshot::default()
            }),
            gain: Mutex::new(12.0),
            ..FakeAudio::default()
        };
        let sink = WindowsAudioSink::new(Arc::new(fake));
        let snapshot = sink.snapshot();
        assert_eq!(snapshot.state, AudioState::Ready);
        assert_eq!(snapshot.generation.value(), 3);
        assert_eq!(snapshot.gain_db, 12);
        let endpoint = snapshot.endpoint.expect("endpoint");
        assert_eq!(endpoint.name, "CABLE Input");
        assert!(endpoint.recommended);
    }

    #[test]
    fn list_and_select_map_recommendation() {
        let sink = WindowsAudioSink::new(Arc::new(FakeAudio::with_endpoints()));
        let endpoints = sink.list_endpoints().expect("list");
        assert_eq!(endpoints.len(), 2);
        assert!(endpoints[0].recommended);
        assert!(!endpoints[1].recommended);

        let selected = sink.select_endpoint("cable-id").expect("select");
        assert_eq!(selected.state, AudioState::Ready);
        assert_eq!(selected.endpoint.expect("endpoint").id, "cable-id");
    }

    #[test]
    fn gain_is_clamped_to_contract_range() {
        let sink = WindowsAudioSink::new(Arc::new(FakeAudio::default()));
        assert_eq!(sink.set_gain_db(30).expect("gain high").gain_db, 24);
        assert_eq!(sink.set_gain_db(-5).expect("gain low").gain_db, 0);
    }

    #[test]
    fn audio_timeout_becomes_retryable_audio_error() {
        let fake = FakeAudio::with_endpoints();
        *fake.select_error.lock().expect("error") = Some(PlatformError::AudioOperationTimedOut);
        let sink = WindowsAudioSink::new(Arc::new(fake));
        let error = sink.select_endpoint("cable-id").expect_err("must fail");
        assert_eq!(error.domain, ErrorDomain::Audio);
        assert_eq!(error.code, "audio-operation-timeout");
        assert!(error.retryable);
    }
}

#[cfg(test)]
mod action_tests {
    use super::*;
    use shengjian_contract::{Direction, ErrorDomain, LogicalAction, MouseActionKind};
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeActions {
        calls: Mutex<Vec<String>>,
        fail: Mutex<Option<EngineError>>,
    }

    impl FakeActions {
        fn record(&self, call: String) -> CoreResult<()> {
            if let Some(error) = self.fail.lock().expect("fail").take() {
                return Err(error);
            }
            self.calls.lock().expect("calls").push(call);
            Ok(())
        }

        fn calls(&self) -> Vec<String> {
            self.calls.lock().expect("calls").clone()
        }
    }

    impl PlatformActions for FakeActions {
        fn shortcut(&self, keys: &[String]) -> CoreResult<()> {
            self.record(format!("shortcut:{}", keys.join("+")))
        }

        fn focus_input(&self) -> CoreResult<()> {
            self.record("focus".to_owned())
        }

        fn open_app(&self, target: &str) -> CoreResult<()> {
            self.record(format!("open:{target}"))
        }

        fn run_macro(&self, macro_id: &str) -> CoreResult<()> {
            self.record(format!("macro:{macro_id}"))
        }

        fn scroll(&self, direction: Direction, steps: u16) -> CoreResult<()> {
            self.record(format!("scroll:{direction:?}:{steps}"))
        }

        fn move_mouse(&self, direction: Direction, distance: u16) -> CoreResult<()> {
            self.record(format!("move:{direction:?}:{distance}"))
        }

        fn click_mouse(&self) -> CoreResult<()> {
            self.record("click".to_owned())
        }

        fn open_common_phrases(&self) -> CoreResult<()> {
            self.record("phrases".to_owned())
        }
    }

    fn sink(fake: &Arc<FakeActions>) -> WindowsActionSink<FakeActions> {
        WindowsActionSink::new(Arc::clone(fake))
    }

    #[test]
    fn parses_common_names_and_chords() {
        let chord = parse_chord(&["Ctrl+Enter".to_owned(), "F12".to_owned()]).expect("chord");
        assert_eq!(
            chord.keys,
            vec![KeyCode::Control, KeyCode::Enter, KeyCode::F12]
        );
        assert_eq!(parse_key(";").expect("semicolon"), KeyCode::Oem1);
        assert_eq!(parse_key("RightAlt").expect("alt"), KeyCode::RightAlt);
        assert_eq!(parse_key("numpad_5").expect("numpad"), KeyCode::Numpad5);
        let error = parse_key("definitely-not-a-key").expect_err("unknown");
        assert_eq!(error.domain, ErrorDomain::Configuration);
        assert_eq!(error.code, "unsupported-key");
    }

    #[test]
    fn action_sink_routes_every_contract_variant() {
        let fake = Arc::new(FakeActions::default());
        let actions = sink(&fake);
        actions.execute(&LogicalAction::Disabled).expect("disabled");
        actions
            .execute(&LogicalAction::Shortcut {
                label: "Enter".to_owned(),
                keys: vec!["Ctrl".to_owned(), "Enter".to_owned()],
            })
            .expect("shortcut");
        actions
            .execute(&LogicalAction::OpenApp {
                app: "wechat".to_owned(),
                focus: shengjian_contract::FocusStrategy::Direct,
            })
            .expect("open");
        actions.execute(&LogicalAction::FocusInput).expect("focus");
        actions
            .execute(&LogicalAction::RunMacro {
                macro_id: "macro-1".to_owned(),
            })
            .expect("macro");
        actions
            .execute(&LogicalAction::CommonPhrase)
            .expect("phrases");
        actions
            .execute(&LogicalAction::Mouse {
                kind: MouseActionKind::Scroll,
                direction: Some(Direction::Down),
                amount: 20,
            })
            .expect("scroll");
        actions
            .execute(&LogicalAction::Mouse {
                kind: MouseActionKind::Move,
                direction: Some(Direction::Left),
                amount: 30,
            })
            .expect("move");
        actions
            .execute(&LogicalAction::Mouse {
                kind: MouseActionKind::Click,
                direction: None,
                amount: 0,
            })
            .expect("click");

        assert_eq!(
            fake.calls(),
            vec![
                "shortcut:Ctrl+Enter",
                "open:wechat",
                "focus",
                "macro:macro-1",
                "phrases",
                "scroll:Down:20",
                "move:Left:30",
                "click",
            ]
        );
    }

    #[test]
    fn rejects_invalid_mouse_actions_before_backend() {
        let fake = Arc::new(FakeActions::default());
        let actions = sink(&fake);
        let error = actions
            .execute(&LogicalAction::Mouse {
                kind: MouseActionKind::Move,
                direction: Some(Direction::Up),
                amount: 0,
            })
            .expect_err("zero");
        assert_eq!(error.code, "mouse-amount-out-of-range");
        let error = actions
            .execute(&LogicalAction::Mouse {
                kind: MouseActionKind::Scroll,
                direction: None,
                amount: 20,
            })
            .expect_err("direction");
        assert_eq!(error.code, "mouse-direction-required");
        assert!(fake.calls().is_empty());
    }

    #[test]
    fn backend_error_is_returned() {
        let fake = Arc::new(FakeActions::default());
        *fake.fail.lock().expect("fail") =
            Some(EngineError::new(ErrorDomain::Input, "send-input-failed"));
        let actions = sink(&fake);
        let error = actions
            .execute(&LogicalAction::FocusInput)
            .expect_err("backend failure");
        assert_eq!(error.code, "send-input-failed");
    }
}

#[cfg(test)]
mod button_source_tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct FakeButtonPlatform {
        callback: Mutex<Option<Arc<dyn Fn(RemoteButton, bool) + Send + Sync>>>,
    }

    impl FakeButtonPlatform {
        fn emit(&self, button: RemoteButton, pressed: bool) {
            let callback = self.callback.lock().expect("callback").clone();
            if let Some(callback) = callback {
                callback(button, pressed);
            }
        }
    }

    impl PlatformButtonSource for FakeButtonPlatform {
        fn subscribe_platform_edges(
            &self,
            callback: Arc<dyn Fn(RemoteButton, bool) + Send + Sync>,
        ) {
            *self.callback.lock().expect("callback") = Some(callback);
        }
    }

    #[test]
    fn button_source_maps_all_buttons_and_preserves_edges() {
        let platform = Arc::new(FakeButtonPlatform::default());
        let source = WindowsButtonSource::new(Arc::clone(&platform));
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_sink = Arc::clone(&seen);
        source.subscribe_edges(Arc::new(move |edge| {
            seen_sink.lock().expect("edges").push(edge);
        }));

        let pairs = [
            (RemoteButton::Back, LogicalButton::Back),
            (RemoteButton::Ok, LogicalButton::Ok),
            (RemoteButton::Tv, LogicalButton::Tv),
            (RemoteButton::Home, LogicalButton::Home),
            (RemoteButton::Right, LogicalButton::Right),
            (RemoteButton::Left, LogicalButton::Left),
            (RemoteButton::Down, LogicalButton::Down),
            (RemoteButton::Up, LogicalButton::Up),
            (RemoteButton::Menu, LogicalButton::Menu),
            (RemoteButton::Power, LogicalButton::Power),
            (RemoteButton::VolumeMute, LogicalButton::VolumeMute),
            (RemoteButton::VolumeUp, LogicalButton::VolumeUp),
            (RemoteButton::VolumeDown, LogicalButton::VolumeDown),
        ];
        for (platform_button, _) in pairs {
            platform.emit(platform_button, true);
            platform.emit(platform_button, false);
        }

        let edges = seen.lock().expect("edges");
        assert_eq!(edges.len(), pairs.len() * 2);
        for (index, (_, logical)) in pairs.iter().enumerate() {
            assert_eq!(edges[index * 2].button, *logical);
            assert!(edges[index * 2].pressed);
            assert_eq!(edges[index * 2 + 1].button, *logical);
            assert!(!edges[index * 2 + 1].pressed);
            assert!(edges[index * 2].at_ms > 0);
        }
    }
}

#[cfg(test)]
mod button_store_tests {
    use super::*;

    fn temporary_dir() -> PathBuf {
        std::env::temp_dir().join(format!(
            "shengjian-button-store-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ))
    }

    #[test]
    fn profile_store_round_trips_and_missing_is_none() {
        let dir = temporary_dir();
        let store = WindowsProfileStore::new(dir.join("button-profiles.json"));
        assert!(store.load().expect("missing").is_none());
        let document = ButtonProfilesDocument::default();
        store.save(&document).expect("save");
        assert_eq!(store.load().expect("load"), Some(document));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn mapping_store_round_trips_and_missing_is_none() {
        let dir = temporary_dir();
        let store = WindowsMappingStore::new(dir.join("button-mappings.json"));
        assert!(store.load().expect("missing").is_none());
        let mappings = ButtonMappingSet::default();
        store.save(&mappings).expect("save");
        assert_eq!(store.load().expect("load"), Some(mappings));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn newer_profile_schema_is_rejected_and_backed_up() {
        let dir = temporary_dir();
        fs::create_dir_all(&dir).expect("dir");
        let path = dir.join("button-profiles.json");
        let mut document = ButtonProfilesDocument::default();
        document.schema_version = BUTTON_PROFILES_SCHEMA_VERSION + 1;
        fs::write(&path, serde_json::to_string(&document).expect("json")).expect("write");

        let error = WindowsProfileStore::new(path.clone())
            .load()
            .expect_err("newer schema");
        assert_eq!(error.code, "profile-store-newer-schema");
        assert!(path.with_extension("json.bak").is_file());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn corrupt_button_store_files_are_stable_configuration_errors() {
        let dir = temporary_dir();
        fs::create_dir_all(&dir).expect("dir");
        let profile_path = dir.join("button-profiles.json");
        let mapping_path = dir.join("button-mappings.json");
        fs::write(&profile_path, b"not json").expect("profile write");
        fs::write(&mapping_path, b"not json").expect("mapping write");

        let profile_error = WindowsProfileStore::new(profile_path)
            .load()
            .expect_err("profile invalid");
        assert_eq!(profile_error.domain, ErrorDomain::Configuration);
        assert_eq!(profile_error.code, "profile-store-invalid");

        let mapping_error = WindowsMappingStore::new(mapping_path)
            .load()
            .expect_err("mapping invalid");
        assert_eq!(mapping_error.domain, ErrorDomain::Configuration);
        assert_eq!(mapping_error.code, "mapping-store-invalid");
        let _ = fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod settings_tests {
    use super::*;

    fn temporary_store() -> (WindowsSettingsStore, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "shengjian-settings-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or(0)
        ));
        let store = WindowsSettingsStore::new(dir.join("settings.json"));
        (store, dir)
    }

    #[test]
    fn missing_settings_load_defaults_and_round_trip() {
        let (store, dir) = temporary_store();
        assert_eq!(
            store.load().expect("default").schema_version,
            AppSettings::default().schema_version
        );
        let settings = AppSettings {
            gain_db: 12.0,
            ..AppSettings::default()
        };
        store.save(&settings).expect("save");
        assert_eq!(store.load().expect("load").gain_db, 12.0);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn newer_settings_schema_is_rejected_and_backed_up() {
        let (store, dir) = temporary_store();
        fs::create_dir_all(&dir).expect("dir");
        let mut document = SettingsDocument::default();
        document.schema_version = SETTINGS_NAMESPACE_SCHEMA_VERSION + 1;
        fs::write(
            store.path(),
            serde_json::to_string(&document).expect("json"),
        )
        .expect("write");

        let error = store.load_document().expect_err("newer schema");
        assert_eq!(error.code, "settings-newer-schema");
        assert!(store.path().with_extension("json.bak").is_file());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_settings_file_is_a_stable_configuration_error() {
        let (store, dir) = temporary_store();
        fs::create_dir_all(&dir).expect("dir");
        fs::write(store.path(), b"not json").expect("write");
        let error = store.load().expect_err("invalid");
        assert_eq!(error.domain, ErrorDomain::Configuration);
        assert_eq!(error.code, "settings-invalid");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn namespaced_document_round_trips_and_resolves_selected_device() {
        use shengjian_app::DeviceSettingsOverride;

        let (store, dir) = temporary_store();
        let mut document = SettingsDocument::from_flat(AppSettings::default());
        document.global.selected_remote_id = Some("xiaomi-rc003".to_owned());
        document.set_override(
            "xiaomi-rc003",
            "windows",
            DeviceSettingsOverride {
                gain_db: Some(12),
                ..DeviceSettingsOverride::default()
            },
        );
        store.save_document(&document).expect("save document");

        let loaded = store.load_document().expect("load document");
        assert_eq!(loaded.devices["xiaomi-rc003"]["windows"].gain_db, Some(12));
        assert_eq!(store.load().expect("effective").gain_db, 12.0);
        let other = loaded.resolve(Some("xiaomi-rc001"), "windows");
        assert_eq!(other.gain_db, document.global.gain_db);
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn legacy_flat_file_becomes_global_document() {
        let (store, dir) = temporary_store();
        fs::create_dir_all(&dir).expect("dir");
        let legacy = AppSettings {
            gain_db: 7.0,
            ..AppSettings::default()
        };
        fs::write(
            store.path(),
            serde_json::to_string_pretty(&legacy).expect("json"),
        )
        .expect("write legacy");
        let document = store.load_document().expect("load legacy");
        assert!(document.devices.is_empty());
        assert_eq!(document.global.gain_db, 7.0);
        assert_eq!(store.load().expect("effective").gain_db, 7.0);
        let _ = fs::remove_dir_all(dir);
    }
}
