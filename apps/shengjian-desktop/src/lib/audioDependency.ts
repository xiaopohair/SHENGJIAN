/**
 * VB-CABLE 依赖的纯函数：三态文案、是否阻断继续、推荐设备、设备列表合并。
 *
 * 向导与设置页共用这些规则，避免「什么状态算就绪」在两处漂移；组件只负责渲染。
 */
import type { TranslationKey } from "./i18n";
import type { AudioDependencyState, AudioDependencyStatus, AudioEndpoint } from "./types";

export type AudioDependencyTone = "success" | "warning" | "neutral";

/** 后端没给 installUrl 时的兜底下载页（也方便测试断言）。 */
export const VB_CABLE_INSTALL_URL = "https://vb-audio.com/Cable/";

/** 三态标题（loading/未知用 null 表示）。 */
export function audioDependencyTitleKey(
  state: AudioDependencyState | null | undefined,
): TranslationKey {
  switch (state) {
    case "ready":
      return "已检测到 VB-CABLE";
    case "missing":
      return "未检测到 VB-CABLE，远程语音将没有声音";
    case "disabled":
      return "已安装 VB-CABLE，但未找到可用的远程语音输出设备";
    default:
      return "正在检测 VB-CABLE…";
  }
}

/** 三态说明；loading/未知没有说明文案。 */
export function audioDependencyDetailKey(
  state: AudioDependencyState | null | undefined,
): TranslationKey | null {
  switch (state) {
    case "ready":
      return "已找到可用的远程语音输出设备。";
    case "missing":
      return "安装 VB-CABLE 后，遥控器语音会通过虚拟声卡进入转写。";
    case "disabled":
      return "请在 Windows 声音设置中启用 VB-CABLE 的播放/录制设备，或重启系统后重新检测。";
    default:
      return null;
  }
}

export function audioDependencyTone(
  state: AudioDependencyState | null | undefined,
): AudioDependencyTone {
  if (state === "ready") return "success";
  if (state === "missing" || state === "disabled") return "warning";
  return "neutral";
}

/** 只有 ready 才算依赖满足。 */
export function isAudioDependencyReady(status: AudioDependencyStatus | null | undefined): boolean {
  return status?.state === "ready";
}

/** 向导第 3 步在依赖未就绪时硬阻断「下一步」。 */
export function blocksOnboardingProgress(
  step: number,
  status: AudioDependencyStatus | null | undefined,
): boolean {
  return step === 3 && !isAudioDependencyReady(status);
}

/** ready 时后端给的默认输出设备；没有/非 ready 返回 null。 */
export function suggestedEndpointId(
  status: AudioDependencyStatus | null | undefined,
): string | null {
  if (!isAudioDependencyReady(status)) return null;
  const id = status?.suggestedEndpointId?.trim();
  return id ? id : null;
}

/**
 * 把 CABLE 设备合并进现有设备列表（按 id 去重，保留原列表顺序）。
 * 依赖命令返回的 cableEndpoints 可能比 list_audio_endpoints 更新，合并后下拉才能选中。
 */
export function mergeAudioEndpoints(
  base: readonly AudioEndpoint[],
  cable: readonly AudioEndpoint[],
): AudioEndpoint[] {
  const merged = [...base];
  const seen = new Set(base.map((endpoint) => endpoint.id));
  for (const endpoint of cable) {
    if (seen.has(endpoint.id)) continue;
    seen.add(endpoint.id);
    merged.push(endpoint);
  }
  return merged;
}

/** 设置页/健康检查展示用：ready 时优先列出 CABLE 设备名。 */
/** 建议端点（标准 CABLE Input 优先）的显示名；没有则退回第一个候选。 */
export function suggestedCableEndpointName(
  status: AudioDependencyStatus | null | undefined,
): string {
  const endpoints = status?.cableEndpoints ?? [];
  const suggested =
    endpoints.find((endpoint) => endpoint.id === status?.suggestedEndpointId) ?? endpoints[0];
  return suggested?.name?.trim() ?? "";
}

export function cableEndpointNames(status: AudioDependencyStatus | null | undefined): string {
  return (status?.cableEndpoints ?? [])
    .map((endpoint) => endpoint.name?.trim())
    .filter(Boolean)
    .join(" / ");
}
