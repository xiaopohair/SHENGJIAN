use crate::host::{
    DeviceStatus, HostCapabilities, HostError, HostPlatform, MacroRunStatusView, TranscriptRecord,
};
use crate::macros;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use shengjian_app::{EngineSnapshot, SettingsDocument};
use shengjian_contract::{
    AudioEndpoint, AudioSnapshot, ButtonProfilesDocument, ConnectionSnapshot, EngineError,
    RemoteCandidate, RemoteId,
};
use shengjian_domain::{AppSettings, MacroLibraryDocument};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonProfileState {
    pub document: ButtonProfilesDocument,
    pub active_profile_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    pub code: String,
    pub message: String,
    pub retryable: bool,
}

impl CommandError {
    fn new(code: &str, message: &str, retryable: bool) -> Self {
        Self {
            code: code.to_owned(),
            message: message.to_owned(),
            retryable,
        }
    }
}

impl From<EngineError> for CommandError {
    fn from(error: EngineError) -> Self {
        let code = error.code;
        Self {
            message: code.clone(),
            code,
            retryable: error.retryable,
        }
    }
}

impl From<HostError> for CommandError {
    fn from(error: HostError) -> Self {
        Self {
            code: error.code.to_owned(),
            message: error.message,
            retryable: false,
        }
    }
}

impl From<macros::MacroStoreError> for CommandError {
    fn from(error: macros::MacroStoreError) -> Self {
        Self {
            code: error.code.to_owned(),
            message: error.message,
            retryable: error.retryable,
        }
    }
}

#[tauri::command]
pub fn engine_snapshot(state: State<'_, AppState>) -> EngineSnapshot {
    state.engine.snapshots()
}

#[tauri::command]
pub fn engine_event_cursor(state: State<'_, AppState>) -> u64 {
    state.engine.event_publisher().last_seq()
}

#[tauri::command]
pub async fn scan_remotes(
    state: State<'_, AppState>,
) -> Result<Vec<RemoteCandidate>, CommandError> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.connection().scan())
        .await
        .map_err(|_| CommandError::new("command-join-failed", "scan task failed", true))?
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn connect_remote(
    state: State<'_, AppState>,
    remote_id: String,
) -> Result<ConnectionSnapshot, CommandError> {
    let engine = state.engine.clone();
    let remote_id = RemoteId::new(remote_id);
    let result =
        tauri::async_runtime::spawn_blocking(move || engine.connection().connect(&remote_id))
            .await
            .map_err(|_| CommandError::new("command-join-failed", "connect task failed", true))?
            .map_err(CommandError::from)?;
    let device_id = result
        .remote
        .as_ref()
        .and_then(|remote| remote.device_id.as_ref())
        .map(|device| device.as_str().to_owned());
    apply_effective_audio_settings(
        &state.engine,
        device_id.as_deref(),
        state.host.platform_id(),
        state.host.as_ref(),
    );
    select_default_audio_endpoint(&state.engine);
    Ok(result)
}

#[tauri::command]
pub async fn disconnect_remote(
    state: State<'_, AppState>,
) -> Result<ConnectionSnapshot, CommandError> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.disconnect())
        .await
        .map_err(|_| CommandError::new("command-join-failed", "disconnect task failed", true))?
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<SettingsDocument, CommandError> {
    state
        .engine
        .settings()
        .load_document()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn save_settings(
    state: State<'_, AppState>,
    document: SettingsDocument,
) -> Result<(), CommandError> {
    state
        .engine
        .settings()
        .save_document(&document)
        .map_err(CommandError::from)?;
    state
        .host
        .set_enhanced_capture_enabled(document.global.rc003_capture_enabled);
    state
        .host
        .set_transcript_capture_enabled(document.global.local_transcript_history_enabled);
    let device_id = state
        .engine
        .connection()
        .snapshot()
        .remote
        .and_then(|remote| remote.device_id)
        .map(|device| device.as_str().to_owned());
    apply_effective_audio_settings(
        &state.engine,
        device_id.as_deref(),
        state.host.platform_id(),
        state.host.as_ref(),
    );
    select_default_audio_endpoint(&state.engine);
    Ok(())
}

#[tauri::command]
pub fn open_diagnostic_log_folder(state: State<'_, AppState>) -> Result<(), CommandError> {
    state
        .host
        .open_diagnostic_log_folder()
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn open_external_url(url: String) -> Result<(), CommandError> {
    const RELEASE_PREFIX: &str = "https://github.com/xiaopohair/SHENGJIAN/";
    if !url.starts_with(RELEASE_PREFIX) || url.chars().any(char::is_whitespace) {
        return Err(CommandError::new(
            "invalid_url",
            "invalid release URL",
            false,
        ));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("rundll32.exe")
            .args(["url.dll,FileProtocolHandler", &url])
            .spawn()
            .map_err(|error| CommandError::new("open_url_failed", &error.to_string(), true))?;
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
        Err(CommandError::new(
            "unsupported_platform",
            "opening release URLs is only supported on Windows",
            false,
        ))
    }
}

#[tauri::command]
pub fn reset_settings(state: State<'_, AppState>) -> Result<(), CommandError> {
    let settings = AppSettings::default();
    let document = SettingsDocument::from_flat(settings.clone());
    state
        .engine
        .settings()
        .save_document(&document)
        .map_err(CommandError::from)?;
    state
        .host
        .set_enhanced_capture_enabled(settings.rc003_capture_enabled);
    state.host.set_voice_input_tool(settings.voice_input_tool);
    state.host.set_voice_hotkey(settings.voice_hotkey.clone());
    state
        .host
        .set_transcript_capture_enabled(settings.local_transcript_history_enabled);
    let platform_id = state.host.platform_id();
    apply_effective_audio_settings(&state.engine, None, platform_id, state.host.as_ref());
    select_default_audio_endpoint(&state.engine);
    Ok(())
}

#[tauri::command]
pub fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, CommandError> {
    app.autolaunch()
        .is_enabled()
        .map_err(|_| CommandError::new("autostart", "cannot read autostart state", false))
}

#[tauri::command]
pub fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), CommandError> {
    let manager = app.autolaunch();
    let result = if enabled {
        manager.enable()
    } else {
        manager.disable()
    };
    result.map_err(|_| CommandError::new("autostart", "cannot change autostart state", false))
}

#[tauri::command]
pub fn rc003_helper_installed(state: State<'_, AppState>) -> bool {
    state.host.helper_installed()
}

#[tauri::command]
pub fn install_rc003_helper(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> Result<(), CommandError> {
    state.host.install_helper(&app).map_err(CommandError::from)
}

#[tauri::command]
pub fn host_capabilities(state: State<'_, AppState>) -> HostCapabilities {
    state.host.capabilities()
}

#[tauri::command]
pub fn diagnostic_log(
    state: State<'_, AppState>,
    max_bytes: Option<usize>,
) -> Result<String, CommandError> {
    state
        .host
        .read_diagnostic_log(max_bytes.unwrap_or(128 * 1024))
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn export_diagnostic_bundle(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), CommandError> {
    state
        .host
        .export_diagnostic_bundle(&path)
        .map_err(CommandError::from)
}

#[tauri::command]
pub fn remote_battery(state: State<'_, AppState>) -> Option<u8> {
    state.host.battery_level()
}

#[tauri::command]
pub fn voice_level(state: State<'_, AppState>) -> f32 {
    state.host.voice_level_dbfs()
}

#[tauri::command]
pub fn foreground_executable(state: State<'_, AppState>) -> Option<String> {
    state.host.foreground_executable()
}

#[tauri::command]
pub fn resolve_app_target(path: String) -> Option<String> {
    shengjian_windows::app_launcher::target_executable_name(&path)
}

#[tauri::command]
pub fn device_status(state: State<'_, AppState>) -> Option<DeviceStatus> {
    state.host.device_status()
}

#[tauri::command]
pub async fn list_audio_endpoints(
    state: State<'_, AppState>,
) -> Result<Vec<AudioEndpoint>, CommandError> {
    let engine = state.engine.clone();
    tauri::async_runtime::spawn_blocking(move || engine.audio().list_endpoints())
        .await
        .map_err(|_| CommandError::new("command-join-failed", "list endpoints failed", true))?
        .map_err(CommandError::from)
}

#[tauri::command]
pub async fn select_audio_endpoint(
    state: State<'_, AppState>,
    endpoint_id: String,
) -> Result<AudioSnapshot, CommandError> {
    let engine = state.engine.clone();
    let persisted_endpoint_id = endpoint_id.clone();
    let snapshot =
        tauri::async_runtime::spawn_blocking(move || engine.audio().select_endpoint(&endpoint_id))
            .await
            .map_err(|_| CommandError::new("command-join-failed", "select endpoint failed", true))?
            .map_err(CommandError::from)?;

    let mut document = state
        .engine
        .settings()
        .load_document()
        .map_err(CommandError::from)?;
    document.global.audio_endpoint_id = Some(persisted_endpoint_id);
    state
        .engine
        .settings()
        .save_document(&document)
        .map_err(CommandError::from)?;
    Ok(snapshot)
}

#[tauri::command]
pub async fn set_gain_db(
    state: State<'_, AppState>,
    gain_db: i16,
) -> Result<AudioSnapshot, CommandError> {
    let engine = state.engine.clone();
    let snapshot =
        tauri::async_runtime::spawn_blocking(move || engine.audio().set_gain_db(gain_db))
            .await
            .map_err(|_| CommandError::new("command-join-failed", "set gain failed", true))?
            .map_err(CommandError::from)?;

    let mut document = state
        .engine
        .settings()
        .load_document()
        .map_err(CommandError::from)?;
    document.global.gain_db = f32::from(gain_db);
    state
        .engine
        .settings()
        .save_document(&document)
        .map_err(CommandError::from)?;
    Ok(snapshot)
}

#[tauri::command]
pub fn button_profile_state(state: State<'_, AppState>) -> ButtonProfileState {
    ButtonProfileState {
        document: state.engine.button_profiles(),
        active_profile_id: state.engine.active_button_profile_id(),
    }
}

#[tauri::command]
pub fn activate_button_profile(state: State<'_, AppState>, profile_id: String) -> bool {
    state.engine.activate_button_profile(&profile_id)
}

#[tauri::command]
pub fn save_button_profiles(
    state: State<'_, AppState>,
    document: ButtonProfilesDocument,
) -> Result<(), CommandError> {
    state.engine.set_button_profiles(document);
    Ok(())
}

#[tauri::command]
pub fn export_profiles(path: String, document: ButtonProfilesDocument) -> Result<(), CommandError> {
    let contents = serde_json::to_string_pretty(&document)
        .map_err(|_| CommandError::new("profile-export", "cannot encode profiles", false))?;
    std::fs::write(&path, contents)
        .map_err(|_| CommandError::new("profile-export", "cannot write profiles file", true))
}

#[tauri::command]
pub fn import_profiles(
    state: State<'_, AppState>,
    path: String,
) -> Result<ButtonProfilesDocument, CommandError> {
    let contents = std::fs::read_to_string(&path)
        .map_err(|_| CommandError::new("profile-import", "cannot read profiles file", true))?;
    let document = serde_json::from_str::<ButtonProfilesDocument>(&contents)
        .map_err(|_| CommandError::new("profile-import", "invalid profiles file", false))?;
    state.engine.set_button_profiles(document);
    Ok(state.engine.button_profiles())
}

#[tauri::command]
pub fn macro_library(state: State<'_, AppState>) -> Result<MacroLibraryDocument, CommandError> {
    macros::load(&state.macro_path).map_err(CommandError::from)
}

#[tauri::command]
pub fn save_macro_library(
    state: State<'_, AppState>,
    document: MacroLibraryDocument,
) -> Result<MacroLibraryDocument, CommandError> {
    let saved = macros::save(&state.macro_path, &document).map_err(CommandError::from)?;
    state.host.set_macro_library(saved.clone());
    Ok(saved)
}

#[tauri::command]
pub fn test_macro(state: State<'_, AppState>, macro_id: String) -> Result<(), CommandError> {
    state.host.test_macro(&macro_id).map_err(CommandError::from)
}

#[tauri::command]
pub fn cancel_macro(state: State<'_, AppState>, macro_id: String) -> bool {
    state.host.cancel_macro(&macro_id)
}

#[tauri::command]
pub fn macro_run_status(state: State<'_, AppState>, macro_id: String) -> MacroRunStatusView {
    state.host.macro_run_status(&macro_id)
}

#[tauri::command]
pub fn macro_run_history() -> Result<Vec<crate::macro_history::MacroHistoryEntry>, CommandError> {
    crate::macro_history::load()
        .map_err(|message| CommandError::new("macro-history", &message, true))
}

#[tauri::command]
pub fn clear_macro_run_history() -> Result<(), CommandError> {
    crate::macro_history::clear()
        .map_err(|message| CommandError::new("macro-history", &message, true))
}

#[tauri::command]
pub fn transcript_history() -> Result<Vec<TranscriptRecord>, CommandError> {
    crate::transcript_history::load()
        .map_err(|message| CommandError::new("transcript-history", &message, true))
}

#[tauri::command]
pub fn clear_transcript_history() -> Result<(), CommandError> {
    crate::transcript_history::clear()
        .map_err(|message| CommandError::new("transcript-history", &message, true))
}

#[tauri::command]
pub fn set_tray_language(app: tauri::AppHandle, language: String) -> Result<(), CommandError> {
    crate::tray::set_tray_language(&app, &language)
        .map_err(|message| CommandError::new("tray-language", &message, true))
}

#[tauri::command]
pub fn set_tray_update(app: tauri::AppHandle, version: Option<String>) -> Result<(), CommandError> {
    crate::tray::set_tray_update(&app, version)
        .map_err(|message| CommandError::new("tray-update", &message, true))
}

#[tauri::command]
pub fn export_macro_library(
    path: String,
    document: MacroLibraryDocument,
) -> Result<(), CommandError> {
    let contents = serde_json::to_string_pretty(&document)
        .map_err(|_| CommandError::new("macro-export", "cannot encode macro library", false))?;
    std::fs::write(&path, contents)
        .map_err(|_| CommandError::new("macro-export", "cannot write macro library", true))
}

#[tauri::command]
pub fn import_macro_library(
    state: State<'_, AppState>,
    path: String,
) -> Result<MacroLibraryDocument, CommandError> {
    let contents = std::fs::read_to_string(&path)
        .map_err(|_| CommandError::new("macro-import", "cannot read macro library", true))?;
    let document = serde_json::from_str::<MacroLibraryDocument>(&contents)
        .map_err(|_| CommandError::new("macro-import", "invalid macro library", false))?;
    let saved = macros::save(&state.macro_path, &document).map_err(CommandError::from)?;
    state.host.set_macro_library(saved.clone());
    Ok(saved)
}

#[tauri::command]
pub fn clear_button_profile_activation(state: State<'_, AppState>) {
    state.engine.clear_button_profile_activation();
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationBackup {
    format: String,
    version: u32,
    created_at_unix: u64,
    settings: SettingsDocument,
    button_profiles: ButtonProfilesDocument,
    macros: MacroLibraryDocument,
    #[serde(default)]
    transcripts: Vec<TranscriptRecord>,
    #[serde(default)]
    preferences: BackupPreferences,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPreferences {
    accent: Option<String>,
    last_remote_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationRestoreResult {
    profiles: usize,
    macros: usize,
    transcripts: usize,
    accent: Option<String>,
    last_remote_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceSupportView {
    id: String,
    display_name: String,
    model: String,
    status: String,
    notes: String,
    platforms: Vec<DevicePlatformView>,
    capabilities: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DevicePlatformView {
    platform: String,
    status: String,
}

#[tauri::command]
pub fn device_support() -> Result<Vec<DeviceSupportView>, CommandError> {
    const PROFILES: [&str; 2] = [
        include_str!("../../../../profiles/xiaomi-rc003.json"),
        include_str!("../../../../profiles/xiaomi-rc001.json"),
    ];
    PROFILES
        .iter()
        .map(|raw| {
            let value: serde_json::Value = serde_json::from_str(raw).map_err(|_| {
                CommandError::new("device-profile", "invalid device profile", false)
            })?;
            let platforms = value
                .get("platforms")
                .and_then(|platforms| platforms.as_array())
                .map(|platforms| {
                    platforms
                        .iter()
                        .filter_map(|platform| {
                            Some(DevicePlatformView {
                                platform: platform.get("platform")?.as_str()?.to_owned(),
                                status: platform.get("status")?.as_str()?.to_owned(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            let capabilities = value
                .get("capabilities")
                .and_then(|capabilities| capabilities.as_array())
                .map(|capabilities| {
                    capabilities
                        .iter()
                        .filter_map(|capability| capability.as_str().map(str::to_owned))
                        .collect()
                })
                .unwrap_or_default();
            Ok(DeviceSupportView {
                id: value
                    .get("id")
                    .and_then(|id| id.as_str())
                    .unwrap_or("unknown")
                    .to_owned(),
                display_name: value
                    .get("displayName")
                    .and_then(|name| name.as_str())
                    .unwrap_or("Unknown device")
                    .to_owned(),
                model: value
                    .get("model")
                    .and_then(|model| model.as_str())
                    .unwrap_or("unknown")
                    .to_owned(),
                status: value
                    .get("support")
                    .and_then(|support| support.get("status"))
                    .and_then(|status| status.as_str())
                    .unwrap_or("research")
                    .to_owned(),
                notes: value
                    .get("support")
                    .and_then(|support| support.get("notes"))
                    .and_then(|notes| notes.as_str())
                    .unwrap_or("")
                    .to_owned(),
                platforms,
                capabilities,
            })
        })
        .collect()
}

const CONFIGURATION_BACKUP_FORMAT: &str = "shengjian-backup";
const CONFIGURATION_BACKUP_VERSION: u32 = 2;

#[tauri::command]
pub fn export_configuration(
    state: State<'_, AppState>,
    path: String,
    accent: Option<String>,
    last_remote_id: Option<String>,
) -> Result<(), CommandError> {
    let backup = ConfigurationBackup {
        format: CONFIGURATION_BACKUP_FORMAT.to_owned(),
        version: CONFIGURATION_BACKUP_VERSION,
        created_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        settings: state
            .engine
            .settings()
            .load_document()
            .map_err(CommandError::from)?,
        button_profiles: state.engine.button_profiles(),
        macros: macros::load(&state.macro_path).map_err(CommandError::from)?,
        transcripts: crate::transcript_history::load()
            .map_err(|message| CommandError::new("configuration-export", &message, true))?,
        preferences: BackupPreferences {
            accent,
            last_remote_id,
        },
    };
    let contents = serde_json::to_string_pretty(&backup)
        .map_err(|_| CommandError::new("configuration-export", "cannot encode backup", false))?;
    std::fs::write(&path, contents)
        .map_err(|_| CommandError::new("configuration-export", "cannot write backup", true))
}

#[tauri::command]
pub fn import_configuration(
    state: State<'_, AppState>,
    path: String,
) -> Result<ConfigurationRestoreResult, CommandError> {
    let contents = std::fs::read_to_string(&path)
        .map_err(|_| CommandError::new("configuration-import", "cannot read backup", true))?;
    let backup: ConfigurationBackup = serde_json::from_str(&contents)
        .map_err(|_| CommandError::new("configuration-import", "invalid backup", false))?;
    if backup.format != CONFIGURATION_BACKUP_FORMAT
        || !matches!(backup.version, 1 | CONFIGURATION_BACKUP_VERSION)
    {
        return Err(CommandError::new(
            "configuration-import",
            "unsupported backup format",
            false,
        ));
    }

    state
        .engine
        .settings()
        .save_document(&backup.settings)
        .map_err(CommandError::from)?;
    let profile_count = backup.button_profiles.profiles.len();
    state.engine.set_button_profiles(backup.button_profiles);
    let saved_macros =
        macros::save(&state.macro_path, &backup.macros).map_err(CommandError::from)?;
    state.host.set_macro_library(saved_macros.clone());
    let transcript_count = if backup.version >= 2 {
        let count = backup.transcripts.len();
        crate::transcript_history::save(&backup.transcripts)
            .map_err(|message| CommandError::new("configuration-import", &message, true))?;
        count
    } else {
        0
    };

    state
        .host
        .set_enhanced_capture_enabled(backup.settings.global.rc003_capture_enabled);
    state
        .host
        .set_transcript_capture_enabled(backup.settings.global.local_transcript_history_enabled);
    let platform_id = state.host.platform_id();
    let device_id = state
        .engine
        .connection()
        .snapshot()
        .remote
        .and_then(|remote| remote.device_id)
        .map(|device| device.as_str().to_owned());
    apply_effective_audio_settings(
        &state.engine,
        device_id.as_deref(),
        platform_id,
        state.host.as_ref(),
    );
    select_default_audio_endpoint(&state.engine);

    Ok(ConfigurationRestoreResult {
        profiles: profile_count,
        macros: saved_macros.definitions.len(),
        transcripts: transcript_count,
        accent: backup.preferences.accent,
        last_remote_id: backup.preferences.last_remote_id,
    })
}

fn apply_effective_audio_settings(
    engine: &shengjian_app::CoreEngine,
    device_id: Option<&str>,
    platform_id: &str,
    host: &dyn HostPlatform,
) {
    let Ok(document) = engine.settings().load_document() else {
        return;
    };
    let settings = document.resolve(device_id, platform_id);
    host.set_voice_input_tool(settings.voice_input_tool);
    host.set_voice_hotkey(settings.voice_hotkey.clone());
    let gain = settings.gain_db.round().clamp(0.0, 24.0) as i16;
    let _ = engine.audio().set_gain_db(gain);
    if let Some(endpoint_id) = settings.audio_endpoint_id.as_deref() {
        let _ = engine.audio().select_endpoint(endpoint_id);
    }
}

fn select_default_audio_endpoint(engine: &shengjian_app::CoreEngine) {
    if engine.audio().snapshot().endpoint.is_some() {
        return;
    }
    let Ok(endpoints) = engine.audio().list_endpoints() else {
        return;
    };
    let preferred = endpoints
        .iter()
        .find(|endpoint| {
            endpoint.recommended && endpoint.name.to_ascii_lowercase().contains("cable input")
        })
        .or_else(|| endpoints.iter().find(|endpoint| endpoint.recommended));
    if let Some(endpoint) = preferred {
        let _ = engine.audio().select_endpoint(&endpoint.id);
    }
}

pub fn apply_startup_audio_settings(engine: &shengjian_app::CoreEngine, platform_id: &str) {
    let Ok(document) = engine.settings().load_document() else {
        return;
    };
    let settings = document.resolve(None, platform_id);
    let gain = settings.gain_db.round().clamp(0.0, 24.0) as i16;
    let _ = engine.audio().set_gain_db(gain);
    if let Some(endpoint_id) = settings.audio_endpoint_id.as_deref() {
        let _ = engine.audio().select_endpoint(endpoint_id);
    }
}
