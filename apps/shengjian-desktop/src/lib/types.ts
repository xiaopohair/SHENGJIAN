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

export interface SettingsDocument {
  schemaVersion: number;
  global: {
    gain_db: number;
    audio_endpoint_id?: string | null;
    close_to_tray?: boolean;
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
  voiceInputTool?: string | null;
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
