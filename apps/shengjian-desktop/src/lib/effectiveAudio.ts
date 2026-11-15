import type { SettingsDocument } from "./types";

/**
 * 一个设置项解析后的取值与来源。
 *
 * 设备的「独立音频设置」打开时，`devices[deviceId].windows` 里的字段优先于全局值
 * （见 `crates/shengjian-app/src/settings_namespace.rs` 的
 * `resolves_global_then_device_override`）。界面必须用同一口径展示「实际生效」的值，
 * 否则全局页会显示一个并不生效的数字。
 */
export interface EffectiveSetting<T> {
  value: T;
  /** device = 该设备有自己的独立设置；global = 回落到全局设置。 */
  source: "device" | "global";
}

function deviceWindowsOverride(settings: SettingsDocument, deviceId?: string | null) {
  if (!deviceId) return undefined;
  return settings.devices?.[deviceId]?.windows;
}

export function effectiveGainDb(
  settings: SettingsDocument,
  deviceId?: string | null,
): EffectiveSetting<number> {
  const gain = deviceWindowsOverride(settings, deviceId)?.gainDb;
  if (typeof gain === "number" && Number.isFinite(gain)) {
    return { value: Math.round(gain), source: "device" };
  }
  return { value: Math.round(settings.global.gain_db ?? 0), source: "global" };
}

export function effectiveVoiceInputTool(
  settings: SettingsDocument,
  deviceId?: string | null,
): EffectiveSetting<string> {
  const tool = deviceWindowsOverride(settings, deviceId)?.voiceInputTool;
  if (typeof tool === "string" && tool.length > 0) {
    return { value: tool, source: "device" };
  }
  return { value: String(settings.global.voice_input_tool ?? ""), source: "global" };
}

export function effectiveAudioEndpointId(
  settings: SettingsDocument,
  deviceId?: string | null,
): EffectiveSetting<string> {
  const endpoint = deviceWindowsOverride(settings, deviceId)?.audioEndpointId;
  if (typeof endpoint === "string" && endpoint.length > 0) {
    return { value: endpoint, source: "device" };
  }
  return { value: String(settings.global.audio_endpoint_id ?? ""), source: "global" };
}
