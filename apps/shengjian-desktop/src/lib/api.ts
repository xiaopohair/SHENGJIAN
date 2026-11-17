import { invoke } from "@tauri-apps/api/core";
import { uiLocale } from "./i18n";
import { localizeError } from "./errors";
import type {
  AppTargetInfo,
  AudioDependencyStatus,
  AudioEndpoint,
  AudioSnapshot,
  ButtonProfile,
  ButtonProfilesDocument,
  ButtonProfileState,
  CommandError,
  CommonPhrasesDocument,
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
  PresetExportView,
  RawInputStatus,
  RecommendedButtonProfilesBundle,
  RemoteCandidate,
  SettingsDocument,
  TranscriptRecord,
  VoiceToolInfo,
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

export async function getCommonPhrasesDocument(): Promise<CommonPhrasesDocument> {
  return invoke<CommonPhrasesDocument>("common_phrases_document");
}

export async function saveCommonPhrases(document: CommonPhrasesDocument): Promise<void> {
  return invoke<void>("save_common_phrases", { document });
}

export async function openCommonPhrasesPanel(): Promise<boolean> {
  return invoke<boolean>("open_common_phrases_panel");
}

export async function getRawInputStatus(): Promise<RawInputStatus> {
  return invoke<RawInputStatus>("raw_input_status");
}

export async function setButtonListening(enabled: boolean): Promise<RawInputStatus> {
  return invoke<RawInputStatus>("set_button_listening", { enabled });
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

/** 批量取应用图标 data URL（顺序与输入一致；取不到为 null）。 */
export async function appIconDataUrls(targets: string[]): Promise<Array<string | null>> {
  return invoke<Array<string | null>>("app_icon_data_urls", { targets });
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

/**
 * 配置目录下发的语音输入工具目录（`config/voice-tool-presets.json`）。
 * label 直接来自配置，应用名不翻译；defaultHotkey 是 KeyboardEvent.code 序列。
 */
export async function getVoiceToolCatalog(): Promise<VoiceToolInfo[]> {
  return invoke<VoiceToolInfo[]>("voice_tool_catalog");
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

export async function getCurrentProcessId(): Promise<number> {
  return invoke<number>("current_process_id");
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

/** 卸载全按键支持服务：提权移除计划任务（弹一次 UAC）。 */
export async function uninstallRc003Helper(): Promise<void> {
  return invoke<void>("uninstall_rc003_helper");
}

/**
 * 重启全按键支持服务：`schtasks /End` + `/Run`，**不需要管理员**。
 *
 * 后台 agent 换代后，宿主若是共享进程，助手会拒绝自我刷新；重启任务让它重新
 * 扫描宿主并注入当前版本。
 */
export async function restartRc003Helper(): Promise<void> {
  return invoke<void>("restart_rc003_helper");
}

export async function listAudioEndpoints(): Promise<AudioEndpoint[]> {
  return invoke<AudioEndpoint[]>("list_audio_endpoints");
}

/**
 * 检测远程语音输出依赖（目前只认 VB-CABLE）。
 * ready = 已安装且可用；disabled = 已安装但没有可用设备；missing = 未安装。
 */
export async function getAudioDependencyStatus(): Promise<AudioDependencyStatus> {
  return invoke<AudioDependencyStatus>("audio_dependency_status");
}

export async function selectAudioEndpoint(endpointId: string): Promise<AudioSnapshot> {
  return invoke<AudioSnapshot>("select_audio_endpoint", { endpointId });
}

export async function forgetRemote(remoteId: string): Promise<void> {
  return invoke<void>("forget_remote", { remoteId });
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

/**
 * 只读地解析方案文件，供导入预检（冲突、型号、条数）使用。
 *
 * 与 [`importProfiles`] 的区别是它**不会**写进引擎状态，
 * 所以可以先看清楚再决定覆盖哪些；真正落地走 [`saveButtonProfiles`]。
 */
export async function previewImportProfiles(path: string): Promise<ButtonProfilesDocument> {
  return invoke<ButtonProfilesDocument>("preview_import_profiles", { path });
}

/**
 * 只读推荐方案（配置目录下发，按当前型号过滤；传入 null 表示型号未知）。
 * 后端已按兼容型号过滤，`profiles` 为空表示当前型号暂无推荐。
 */
export async function getRecommendedButtonProfiles(
  model: string | null,
): Promise<RecommendedButtonProfilesBundle> {
  return invoke<RecommendedButtonProfilesBundle>("recommended_button_profiles", { model });
}

/** 把推荐方案复制成用户方案：返回新 id、名称带「（我的）」的完整方案。 */
export async function copyRecommendedButtonProfile(
  id: string,
  model: string | null,
): Promise<ButtonProfile> {
  return invoke<ButtonProfile>("copy_recommended_button_profile", { id, model });
}

/**
 * 把当前方案直接导出成推荐方案文件（在应用内完成，不需要命令行）。
 *
 * 后端会按与运行时相同的规则校验并归一化（型号大写、目标小写等），
 * 写出的文件可直接放进仓库 `config/recommended-button-profiles/` 后跑 `pnpm config:build`。
 */
export async function exportProfileAsPreset(params: {
  path: string;
  profile: ButtonProfile;
  presetId: string;
  compatibleModels: string[];
}): Promise<PresetExportView> {
  return invoke<PresetExportView>("export_profile_as_preset", {
    path: params.path,
    profile: params.profile,
    presetId: params.presetId,
    compatibleModels: params.compatibleModels,
  });
}

export async function getMacroLibrary(): Promise<MacroLibraryDocument> {
  return invoke<MacroLibraryDocument>("macro_library");
}

/**
 * 配置目录下发的只读推荐宏（与用户宏库同构）。
 * 只用于按键方案动作里的「运行其他宏」下拉；宏页不展示，也不写入用户宏库。
 */
export async function getRecommendedMacros(): Promise<MacroDefinition[]> {
  return invoke<MacroDefinition[]>("recommended_macro_definitions");
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

export interface VoiceTestPadRect {
  x: number;
  y: number;
  width: number;
  height: number;
  /** 0xRRGGBB：卡片背景/边框，取当前主题的计算样式。 */
  background: number;
  border: number;
}

/**
 * 打开原生语音测试板（Win32 EDIT：输入法可以在里面落字，不用切应用）。
 * 坐标是主窗口客户区的**物理像素**，由调用方按 `getBoundingClientRect × devicePixelRatio` 算好。
 */
export async function openVoiceTestPad(rect: VoiceTestPadRect): Promise<void> {
  return invoke<void>("open_voice_test_pad", { ...rect });
}

/** 主窗口重新获得焦点时，把测试板重新顶到最前。 */
export async function raiseVoiceTestPad(): Promise<void> {
  return invoke<void>("raise_voice_test_pad");
}

/** 原生窗口当前是否前台。 */
export async function voiceTestPadForeground(): Promise<boolean> {
  return invoke<boolean>("voice_test_pad_foreground");
}

/** 收起成底部细带：露出识别文字，同时保留键盘焦点。 */
export async function collapseVoiceTestPad(): Promise<void> {
  return invoke<void>("collapse_voice_test_pad");
}

/** 还原成完整高度（下一次按下语音键时）。 */
export async function expandVoiceTestPad(): Promise<void> {
  return invoke<void>("expand_voice_test_pad");
}

export async function closeVoiceTestPad(): Promise<void> {
  return invoke<void>("close_voice_test_pad");
}

export async function voiceTestPadText(): Promise<string | null> {
  return invoke<string | null>("voice_test_pad_text");
}

/** 设备测试面板：打开期间临时开启捕获，结果只进内存（不写语音记录）。 */
export async function beginTranscriptProbe(): Promise<void> {
  return invoke<void>("begin_transcript_probe");
}

/** 关闭探针并恢复用户原来的捕获开关。 */
export async function endTranscriptProbe(): Promise<void> {
  return invoke<void>("end_transcript_probe");
}

/** 取走最近一次识别内容（取过即清空）。 */
export async function takeTranscriptProbe(): Promise<TranscriptRecord | null> {
  return invoke<TranscriptRecord | null>("take_transcript_probe");
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

/**
 * 只读预览宏库文件：解析但不落盘。
 *
 * 多文件导入要先把每个文件读进来合并，再统一保存；不能直接用
 * `importMacroLibrary`（它每个文件都会整库覆盖一次）。
 */
export async function previewImportMacroLibrary(path: string): Promise<MacroLibraryDocument> {
  return invoke<MacroLibraryDocument>("preview_import_macro_library", { path });
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
