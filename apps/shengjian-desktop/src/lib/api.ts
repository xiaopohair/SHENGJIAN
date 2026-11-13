import { invoke } from "@tauri-apps/api/core";
import { uiLocale } from "./i18n";
import { localizeError } from "./errors";
import type {
  AppTargetInfo,
  AudioEndpoint,
  AudioSnapshot,
  ButtonProfilesDocument,
  ButtonProfileState,
  CommandError,
  ConnectionSnapshot,
  DeviceStatus,
  DeviceSupport,
  EngineSnapshot,
  HostCapabilities,
  LaunchableAppsResult,
  MacroDefinition,
  MacroLibraryDocument,
  MacroHistoryEntry,
  MacroRunStatus,
  RemoteCandidate,
  SettingsDocument,
  TranscriptRecord,
} from "./types";

export async function getEngineSnapshot(): Promise<EngineSnapshot> {
  return invoke<EngineSnapshot>("engine_snapshot");
}

export async function getHostCapabilities(): Promise<HostCapabilities> {
  return invoke<HostCapabilities>("host_capabilities");
}

export async function getRemoteBattery(): Promise<number | null> {
  return invoke<number | null>("remote_battery");
}

export async function getVoiceLevel(): Promise<number> {
  return invoke<number>("voice_level");
}

export async function getDeviceStatus(): Promise<DeviceStatus | null> {
  return invoke<DeviceStatus | null>("device_status");
}

export async function getDeviceSupport(): Promise<DeviceSupport[]> {
  return invoke<DeviceSupport[]>("device_support");
}

export async function getForegroundExecutable(): Promise<string | null> {
  return invoke<string | null>("foreground_executable");
}

/**
 * @deprecated 旧的选择器用它把任意路径降级成 exe 名；新的「打开应用」链路改用
 * `canonicalizeAppTarget`（写入规范 target）与 `describeAppTargets`（展示友好名）。
 * 保留它只是为了还没迁移的旧调用。
 */
export async function resolveAppTarget(path: string): Promise<string | null> {
  return invoke<string | null>("resolve_app_target", { path });
}

/**
 * 列出可启动的应用（预设 + 开始菜单/AppsFolder 扫描 + 缓存）。
 * `refresh=true` 时强制重新扫描；扫描失败时预设部分仍会返回，`scanError` 说明原因。
 */
export async function listLaunchableApps(refresh = false): Promise<LaunchableAppsResult> {
  return invoke<LaunchableAppsResult>("list_launchable_apps", { refresh });
}

/** 批量描述已有 target：返回友好名、可启动性、自动匹配键与问题 token。 */
export async function describeAppTargets(targets: string[]): Promise<AppTargetInfo[]> {
  return invoke<AppTargetInfo[]>("describe_app_targets", { targets });
}

/** 把手填 target 规范化成可写入配置的身份（预设 id / 绝对路径 / AUMID）。 */
export async function canonicalizeAppTarget(target: string): Promise<AppTargetInfo> {
  return invoke<AppTargetInfo>("canonicalize_app_target", { raw: target });
}

export async function readDiagnosticLog(maxBytes?: number): Promise<string> {
  return invoke<string>("diagnostic_log", { maxBytes });
}

export async function exportDiagnosticBundle(path: string): Promise<void> {
  return invoke<void>("export_diagnostic_bundle", { path });
}

export async function getEngineEventCursor(): Promise<number> {
  return invoke<number>("engine_event_cursor");
}

export async function scanRemotes(): Promise<RemoteCandidate[]> {
  return invoke<RemoteCandidate[]>("scan_remotes");
}

export async function connectRemote(remoteId: string): Promise<ConnectionSnapshot> {
  return invoke<ConnectionSnapshot>("connect_remote", { remoteId });
}

export async function disconnectRemote(): Promise<ConnectionSnapshot> {
  return invoke<ConnectionSnapshot>("disconnect_remote");
}

export async function getSettings(): Promise<SettingsDocument> {
  return invoke<SettingsDocument>("get_settings");
}

export async function saveSettings(document: SettingsDocument): Promise<void> {
  return invoke<void>("save_settings", { document });
}

export async function resetSettings(): Promise<void> {
  return invoke<void>("reset_settings");
}

export async function openDiagnosticLogFolder(): Promise<void> {
  return invoke<void>("open_diagnostic_log_folder");
}

export async function openExternalUrl(url: string): Promise<void> {
  return invoke<void>("open_external_url", { url });
}

export async function getAutostartEnabled(): Promise<boolean> {
  return invoke<boolean>("get_autostart_enabled");
}

export async function setAutostartEnabled(enabled: boolean): Promise<void> {
  return invoke<void>("set_autostart_enabled", { enabled });
}

export async function getRc003HelperInstalled(): Promise<boolean> {
  return invoke<boolean>("rc003_helper_installed");
}

export async function installRc003Helper(): Promise<void> {
  return invoke<void>("install_rc003_helper");
}

export async function listAudioEndpoints(): Promise<AudioEndpoint[]> {
  return invoke<AudioEndpoint[]>("list_audio_endpoints");
}

export async function selectAudioEndpoint(endpointId: string): Promise<AudioSnapshot> {
  return invoke<AudioSnapshot>("select_audio_endpoint", { endpointId });
}

export async function setGainDb(gainDb: number): Promise<AudioSnapshot> {
  return invoke<AudioSnapshot>("set_gain_db", { gainDb });
}

export async function getButtonProfileState(): Promise<ButtonProfileState> {
  return invoke<ButtonProfileState>("button_profile_state");
}

export async function activateButtonProfile(profileId: string): Promise<boolean> {
  return invoke<boolean>("activate_button_profile", { profileId });
}

export async function saveButtonProfiles(document: ButtonProfilesDocument): Promise<void> {
  return invoke<void>("save_button_profiles", { document });
}

export async function exportProfiles(
  path: string,
  document: ButtonProfilesDocument,
): Promise<void> {
  return invoke<void>("export_profiles", { path, document });
}

export async function importProfiles(path: string): Promise<ButtonProfilesDocument> {
  return invoke<ButtonProfilesDocument>("import_profiles", { path });
}

export async function getMacroLibrary(): Promise<MacroLibraryDocument> {
  return invoke<MacroLibraryDocument>("macro_library");
}

export async function saveMacroLibrary(
  document: MacroLibraryDocument,
): Promise<MacroLibraryDocument> {
  return invoke<MacroLibraryDocument>("save_macro_library", { document });
}

export async function testMacro(macroId: string): Promise<void> {
  return invoke<void>("test_macro", { macroId });
}

export async function testMacroDraft(definition: MacroDefinition): Promise<void> {
  return invoke<void>("test_macro_draft", { definition });
}

export async function cancelMacro(macroId: string): Promise<boolean> {
  return invoke<boolean>("cancel_macro", { macroId });
}

export async function getMacroRunStatus(macroId: string): Promise<MacroRunStatus> {
  return invoke<MacroRunStatus>("macro_run_status", { macroId });
}

export async function getMacroRunHistory(): Promise<MacroHistoryEntry[]> {
  return invoke<MacroHistoryEntry[]>("macro_run_history");
}

export async function clearMacroRunHistory(): Promise<void> {
  return invoke<void>("clear_macro_run_history");
}

export async function getTranscriptHistory(): Promise<TranscriptRecord[]> {
  return invoke<TranscriptRecord[]>("transcript_history");
}

export async function clearTranscriptHistory(): Promise<void> {
  return invoke<void>("clear_transcript_history");
}

export async function setTrayLanguage(language: string): Promise<void> {
  return invoke<void>("set_tray_language", { language });
}

export async function setTrayUpdate(version: string | null): Promise<void> {
  return invoke<void>("set_tray_update", { version });
}

export async function exportMacroLibrary(
  path: string,
  document: MacroLibraryDocument,
): Promise<void> {
  return invoke<void>("export_macro_library", { path, document });
}

export async function importMacroLibrary(path: string): Promise<MacroLibraryDocument> {
  return invoke<MacroLibraryDocument>("import_macro_library", { path });
}

export async function clearButtonProfileActivation(): Promise<void> {
  return invoke<void>("clear_button_profile_activation");
}

export interface ConfigurationRestoreResult {
  profiles: number;
  macros: number;
  transcripts: number;
  accent?: string | null;
  lastRemoteId?: string | null;
}

export async function exportConfiguration(
  path: string,
  accent: string,
  lastRemoteId: string | null,
): Promise<void> {
  return invoke<void>("export_configuration", { path, accent, lastRemoteId });
}

export async function importConfiguration(path: string): Promise<ConfigurationRestoreResult> {
  return invoke<ConfigurationRestoreResult>("import_configuration", { path });
}

export function errorMessage(error: unknown): string {
  if (typeof error === "string") {
    return localizeError(error, undefined, uiLocale.value === "en");
  }
  if (error && typeof error === "object" && "message" in error) {
    const commandError = error as CommandError;
    return localizeError(commandError.message, commandError.code, uiLocale.value === "en");
  }
  return String(error);
}
