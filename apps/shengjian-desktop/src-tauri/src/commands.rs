use crate::host::{
    DeviceStatus, HostCapabilities, HostError, HostPlatform, MacroRunStatusView, TranscriptRecord,
};
use crate::macros;
use crate::state::AppState;
use serde::{Deserialize, Serialize};
use shengjian_app::{EngineSnapshot, SettingsDocument};
use shengjian_contract::{
    AudioEndpoint, AudioSnapshot, ButtonProfile, ButtonProfilesDocument, ConnectionSnapshot,
    EngineError, RemoteCandidate, RemoteId,
};
use shengjian_domain::{AppSettings, MacroLibraryDocument};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tauri_plugin_autostart::ManagerExt;

/// 可配置的语音输入工具目录（`config/voice-tools.json`）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VoiceToolInfo {
    pub id: String,
    pub label: String,
    pub default_hotkey: Vec<String>,
}

#[tauri::command]
pub fn voice_tool_catalog() -> Vec<VoiceToolInfo> {
    shengjian_domain::voice_tools::tools()
        .into_iter()
        .map(|tool| VoiceToolInfo {
            id: tool.id,
            label: tool.label,
            default_hotkey: tool.default_hotkey,
        })
        .collect()
}

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
    // 方案按设备型号隔离：连上就立刻切到该型号的方案，不等后台轮询。
    state.engine.sync_device_model();
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
    let result = tauri::async_runtime::spawn_blocking(move || engine.disconnect())
        .await
        .map_err(|_| CommandError::new("command-join-failed", "disconnect task failed", true))?
        .map_err(CommandError::from)?;
    // 断开后型号变回未知；让方案服务跟着回落，界面不会挂着上一台的方案。
    state.engine.sync_device_model();
    Ok(result)
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
    state
        .host
        .set_voice_input_tool(settings.voice_input_tool.as_deref());
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

/// 兼容保留：把 target 降级成可执行文件名（旧调用方用）。
///
/// 已弃用（deprecated）：新代码不要再用它把规范 target 降级成 exe 名——
/// 读取目标状态/问题用 [`describe_app_targets`]，派生方案匹配键用
/// [`shengjian_windows::app_launcher::app_match_key`]。
#[tauri::command]
pub fn resolve_app_target(path: String) -> Option<String> {
    shengjian_windows::app_launcher::target_executable_name(&path)
}

/// 已安装应用目录（预设 + 开始菜单/AppsFolder 已安装应用，后端 60s 缓存）。
#[tauri::command]
pub async fn list_launchable_apps(
    refresh: Option<bool>,
) -> Result<shengjian_windows::app_launcher::LaunchableAppsResult, CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        shengjian_windows::app_launcher::list_launchable_apps(refresh.unwrap_or(false))
    })
    .await
    .map_err(|_| CommandError::new("command-join-failed", "list launchable apps failed", true))
}

/// 批量描述应用目标：选择器加载旧值后一次问完（安装/可启动/可匹配/旧式/问题）。
#[tauri::command]
pub fn describe_app_targets(
    targets: Vec<String>,
) -> Vec<shengjian_windows::app_launcher::AppTargetInfo> {
    shengjian_windows::app_launcher::describe_app_targets(&targets)
}

/// 规范化单条应用目标（手动填写/旧值修复时校验用）。
#[tauri::command]
pub fn canonicalize_app_target(raw: String) -> shengjian_windows::app_launcher::AppTargetInfo {
    shengjian_windows::app_launcher::canonical_app_target(&raw)
}

/// 批量提取应用图标（`data:image/png;base64,` data URL；顺序与输入一致，失败为 null）。
#[tauri::command]
pub async fn app_icon_data_urls(targets: Vec<String>) -> Vec<Option<String>> {
    let count = targets.len();
    tauri::async_runtime::spawn_blocking(move || {
        shengjian_windows::app_icons::app_icon_data_urls(&targets)
    })
    .await
    .unwrap_or_else(|_| vec![None; count])
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

/// 「推荐方案」列表里的一条只读视图（`profile` 内含完整内容供预览/复制）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedButtonProfile {
    pub id: String,
    pub name: String,
    pub device_model: Option<String>,
    pub compatible_models: Vec<String>,
    pub app_targets: Vec<String>,
    pub auto_apply: bool,
    pub bind_count: usize,
    pub profile: ButtonProfile,
}

/// 内置推荐方案目录（只读；按当前型号过滤）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendedButtonProfilesView {
    pub bundle_version: String,
    pub profiles: Vec<RecommendedButtonProfile>,
}

/// 列出内置只读推荐方案（按 `compatibleModels` 过滤当前型号）。
///
/// 推荐方案不进入用户方案文档；要编辑请走 [`copy_recommended_button_profile`]。
#[tauri::command]
pub fn recommended_button_profiles(model: Option<String>) -> RecommendedButtonProfilesView {
    let bundle_version = shengjian_domain::button_presets::bundle_version();
    let profiles = shengjian_domain::button_presets::profiles_for_model(model.as_deref())
        .into_iter()
        .map(|preset| RecommendedButtonProfile {
            id: preset.profile.id.clone(),
            name: preset.profile.name.clone(),
            device_model: preset.profile.device_model.clone(),
            compatible_models: preset.compatible_models,
            app_targets: preset.profile.app_targets.clone(),
            auto_apply: preset.profile.auto_apply,
            bind_count: preset.profile.bound_button_count(),
            profile: preset.profile,
        })
        .collect();
    RecommendedButtonProfilesView {
        bundle_version,
        profiles,
    }
}

/// 把内置推荐方案复制成一份可编辑的普通方案（不落盘）。
///
/// 校验存在性与型号适配；生成与现有用户方案不冲突的 id、名称加「（我的）」、
/// 盖 `createdAt`；`deviceModel/appTargets/bindings` 原样保留。前端把返回值插入
/// 用户文档后走现有 `save_button_profiles` 流程。
#[tauri::command]
pub fn copy_recommended_button_profile(
    state: State<'_, AppState>,
    id: String,
    model: Option<String>,
) -> Result<ButtonProfile, CommandError> {
    let preset = shengjian_domain::button_presets::profile(&id).ok_or_else(|| {
        CommandError::new(
            "preset-not-found",
            "recommended button profile not found",
            false,
        )
    })?;
    if !preset.applies_to_model(model.as_deref()) {
        return Err(CommandError::new(
            "preset-model-mismatch",
            "recommended button profile is not compatible with the current device model",
            false,
        ));
    }
    let existing_ids: Vec<String> = state
        .engine
        .button_profiles()
        .profiles
        .into_iter()
        .map(|profile| profile.id)
        .collect();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    Ok(shengjian_domain::button_presets::copy_as_user_profile(
        &preset,
        &existing_ids,
        now.as_millis() as u64,
        now.as_secs(),
    ))
}

#[cfg(test)]
mod recommended_profiles_tests {
    use super::*;

    /// 前端契约：字段名/过滤/计数字段与 `recommended_button_profiles` 声明一致。
    #[test]
    fn recommended_profiles_view_serializes_frontend_contract() {
        let view = recommended_button_profiles(Some("rc003".to_owned()));
        let value = serde_json::to_value(&view).expect("serialize recommended profiles");
        assert_eq!(value["bundleVersion"], "2026.11.1");
        let profiles = value["profiles"].as_array().expect("profiles array");
        assert_eq!(profiles.len(), 2, "RC003 应看到通用 + 仅 RC003 两条");
        assert_eq!(profiles[0]["id"], "preset.desktop.shortcuts");
        assert_eq!(profiles[0]["bindCount"], 2);
        assert_eq!(profiles[0]["deviceModel"], serde_json::Value::Null);
        assert_eq!(profiles[1]["compatibleModels"][0], "RC003");
        assert_eq!(profiles[1]["appTargets"][0], "netease_music");
        assert_eq!(profiles[1]["autoApply"], true);
        assert_eq!(profiles[1]["profile"]["autoApply"], true);
        assert_eq!(profiles[1]["profile"]["isDefault"], false);

        // 仅 RC003 适配的方案不出现在 ARN9 下。
        let arn9 = recommended_button_profiles(Some("ARN9".to_owned()));
        assert_eq!(arn9.profiles.len(), 1);
        assert_eq!(arn9.profiles[0].id, "preset.desktop.shortcuts");
    }
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
pub fn test_macro_draft(
    state: State<'_, AppState>,
    definition: shengjian_domain::MacroDefinition,
) -> Result<(), CommandError> {
    state
        .host
        .test_macro_definition(definition)
        .map_err(CommandError::from)
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
    const PROFILES: [&str; 3] = [
        include_str!("../../../../profiles/xiaomi-rc003.json"),
        include_str!("../../../../profiles/xiaomi-rc001.json"),
        include_str!("../../../../profiles/xiaomi-arn9.json"),
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
    host.set_voice_input_tool(settings.voice_input_tool.as_deref());
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
