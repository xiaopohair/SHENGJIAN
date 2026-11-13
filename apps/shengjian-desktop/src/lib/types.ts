export type ConnectionPhase =
  "idle" | "scanning" | "connecting" | "ready" | "disconnecting" | "disconnected";

export interface RemoteCandidate {
  remoteId: string;
  name: string;
  model: string | null;
  deviceId: string | null;
  isSupported: boolean;
  token: string | null;
}

export interface ConnectionSnapshot {
  phase: ConnectionPhase;
  remote: RemoteCandidate | null;
  generation: number;
  lastError: EngineErrorPayload | null;
}

export interface VoiceSnapshot {
  state: string;
  generation: number;
  lastError: EngineErrorPayload | null;
}

export interface AudioEndpoint {
  id: string;
  name: string;
  recommended: boolean;
}

export interface AudioSnapshot {
  state: string;
  endpoint: AudioEndpoint | null;
  format: unknown;
  gainDb: number;
  generation: number;
  lastError: EngineErrorPayload | null;
}

export interface MacroChord {
  modifiers: string[];
  key: string;
}

export interface MacroParameters {
  target?: string | null;
  state?: string | null;
  chord?: MacroChord | null;
  url?: string | null;
  nestedMacroId?: string | null;
  nestedMacroName?: string | null;
}

export interface MacroStep {
  stepId: string;
  action: string;
  timeoutMs?: number | null;
  parameters: MacroParameters;
}

export interface MacroDefinition {
  schemaVersion: string;
  macroId: string;
  version: string;
  name: string;
  summary?: string | null;
  steps: MacroStep[];
}

export interface MacroLibraryDocument {
  formatVersion: number;
  definitions: MacroDefinition[];
}

export interface MacroStepReport {
  index: number;
  stepId: string;
  action: string;
  outcome: string;
  elapsedMs: number;
}

export interface MacroRunReport {
  macroId: string;
  result: string;
  elapsedMs: number;
  steps: MacroStepReport[];
}

export interface MacroRunStatus {
  running: boolean;
  report: MacroRunReport | null;
}

export interface MacroHistoryEntry {
  macroId: string;
  name: string;
  result: string;
  elapsedMs: number;
  atUnix: number;
  steps: number;
}

export interface DeviceSettingsOverride {
  gainDb?: number;
  audioEndpointId?: string | null;
  audioEndpointName?: string | null;
  voiceInputTool?: string | null;
}

/**
 * 配置目录（`config/voice-tools.json`）下发的语音输入工具。
 * `label` 是应用自己的名字，不做界面翻译；`defaultHotkey` 是
 * `KeyboardEvent.code` 序列，作为用户未自定义热键时的默认值。
 */
export interface VoiceToolInfo {
  id: string;
  label: string;
  defaultHotkey: string[];
}

export interface SettingsDocument {
  schemaVersion: number;
  global: {
    gain_db: number;
    audio_endpoint_id?: string | null;
    close_to_tray?: boolean;
    open_window_at_launch?: boolean;
    check_prerelease_updates?: boolean;
    theme_preference?: string;
    rc003_capture_enabled?: boolean;
    voice_input_tool?: string | null;
    voice_hotkey?: string[] | null;
    local_transcript_history_enabled?: boolean;
    application_language?: string;
    [key: string]: unknown;
  };
  devices: Record<string, Record<string, DeviceSettingsOverride>>;
}

export type LogicalButton =
  | "back"
  | "ok"
  | "tv"
  | "home"
  | "menu"
  | "left"
  | "right"
  | "down"
  | "up"
  | "power"
  | "volume-up"
  | "volume-down";

export type LogicalAction =
  | { type: "disabled" }
  | { type: "focus-input" }
  | { type: "shortcut"; label: string; keys: string[] }
  | { type: "open-app"; app: string; focus?: string }
  | { type: "common-phrase" }
  | { type: "mouse"; kind: string; direction?: string | null; amount?: number }
  | { type: "run-macro"; macro_id: string }
  | { type: string; [key: string]: unknown };

export interface ButtonProfile {
  id: string;
  name: string;
  appTargets: string[];
  autoApply: boolean;
  holdRepeat?: "single" | "long" | null;
  holdRepeatByButton?: Record<string, "single" | "long" | null>;
  voiceInputTool?: string | null;
  /** 归属设备型号（大写，如 "RC003"）；null = 通用方案。 */
  deviceModel?: string | null;
  /** 是否为该型号的默认方案（不可删除、不可改名）。 */
  isDefault?: boolean;
  bindings: Record<string, Record<string, LogicalAction>>;
}

export interface ButtonProfilesDocument {
  schemaVersion: number;
  profiles: ButtonProfile[];
  activeManualProfileId: string | null;
}

export interface ButtonProfileState {
  document: ButtonProfilesDocument;
  activeProfileId: string | null;
}

export interface ButtonEdge {
  button: string;
  pressed: boolean;
  atMs: number;
}

export interface ButtonGesture {
  button: string;
  trigger: string;
  atMs: number;
}

export interface HostCapabilities {
  platform: string;
  enhancedCapture: boolean;
  macroRuntime: boolean;
  autostart: boolean;
  tray: boolean;
  commonPhrases: boolean;
}

export interface DeviceStatus {
  phase: string;
  generation: number;
  reconnectAttempt: number;
  powerNotificationsAvailable: boolean;
  batteryLevel: number | null;
  firmwareVersion: string | null;
}

export interface DevicePlatformSupport {
  platform: string;
  status: string;
}

export interface DeviceSupport {
  id: string;
  displayName: string;
  model: string;
  status: string;
  notes: string;
  platforms: DevicePlatformSupport[];
  capabilities: string[];
}

export interface TranscriptRecord {
  sessionId: string;
  startedAtMs: number;
  endedAtMs: number;
  localDateKey: string;
  applicationKey: string;
  applicationName: string;
  source: string;
  text: string;
}

export interface EngineSnapshot {
  connection: ConnectionSnapshot;
  voice: VoiceSnapshot;
  audio: AudioSnapshot;
  capabilities: unknown;
}

export interface EngineErrorPayload {
  domain: string;
  code: string;
  retryable: boolean;
}

export interface EventEnvelope {
  seq: number;
  generation: number;
  event: {
    type: string;
    snapshot?: unknown;
    edge?: unknown;
    gesture?: unknown;
    error?: EngineErrorPayload;
    [key: string]: unknown;
  };
}

export interface CommandError {
  code: string;
  message: string;
  retryable: boolean;
}

/** 应用目标形态：预设 id / 绝对路径 / shell:AppsFolder AUMID / 旧式裸名 / 无效。 */
export type AppTargetKind = "preset" | "path" | "aumid" | "legacyName" | "invalid";

/** 可列出的应用条目形态（旧式裸名不会由列表命令产生）。 */
export type LaunchableAppKind = "preset" | "path" | "aumid";
export type LaunchableAppSource = "preset" | "startMenu" | "appsFolder";

/**
 * 后端 `describe_app_targets` / `canonicalize_app_target` 的解析结果。
 * `input` 是调用方传入的原串，其余字段是规范化后的身份描述。
 */
export interface AppTargetInfo {
  input: string;
  target: string;
  kind: AppTargetKind;
  displayName: string | null;
  executableName: string | null;
  installed: boolean;
  launchable: boolean;
  matchable: boolean;
  portable: boolean;
  legacy: boolean;
  /** issue token：empty/preset_not_installed/path_missing/...，正常时为 null。 */
  issue: string | null;
}

/** 可启动应用列表中的一项。 */
export interface LaunchableApp {
  appId: string;
  displayName: string;
  target: string;
  executableName: string | null;
  kind: LaunchableAppKind;
  source: LaunchableAppSource;
  installed: boolean;
  portable: boolean;
  matchable: boolean;
}

/** `list_launchable_apps` 的返回：应用列表 + 扫描是否完整。 */
export interface LaunchableAppsResult {
  apps: LaunchableApp[];
  scanned: boolean;
  /** scanError token：app_scan_failed/app_scan_busy/app_scan_timeout，正常时为 null。 */
  scanError: string | null;
}
