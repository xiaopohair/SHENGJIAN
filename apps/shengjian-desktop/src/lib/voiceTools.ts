/**
 * 语音输入工具目录的纯函数工具。
 *
 * 工具列表来自配置目录（`config/voice-tools.json`）：`label` 是应用自己的名字，
 * 不做界面翻译；已保存的设置里仍然只存字符串 id。
 */
import type { VoiceToolInfo } from "./types";

export interface VoiceToolOption {
  id: string;
  label: string;
  /**
   * true = 已保存、但不在当前目录里的旧 id 的临时兜底项：
   * 仍然显示并保持选中，避免用户配置被静默丢掉。
   */
  fallback: boolean;
}

/** 目录项 + 当前值的兜底项（下拉选项）。 */
export function voiceToolOptions(
  catalog: readonly VoiceToolInfo[],
  currentId: string | null | undefined,
): VoiceToolOption[] {
  const options = catalog.map((tool) => ({
    id: tool.id,
    label: tool.label?.trim() || tool.id,
    fallback: false,
  }));
  const current = (currentId ?? "").trim();
  if (current && !catalog.some((tool) => tool.id === current)) {
    options.push({ id: current, label: current, fallback: true });
  }
  return options;
}

/** 展示用的工具名：目录里的 label，目录不认识的旧 id 原样回退。 */
export function voiceToolLabel(
  catalog: readonly VoiceToolInfo[],
  id: string | null | undefined,
): string {
  const key = (id ?? "").trim();
  if (!key) return "";
  const tool = catalog.find((entry) => entry.id === key);
  if (!tool) return key;
  return tool.label?.trim() || key;
}

/** 该工具的默认热键；目录里没有这个 id 时返回空（不注入热键）。 */
export function defaultHotkeyFor(
  catalog: readonly VoiceToolInfo[],
  id: string | null | undefined,
): string[] {
  const key = (id ?? "").trim();
  if (!key) return [];
  const tool = catalog.find((entry) => entry.id === key);
  return tool?.defaultHotkey ? [...tool.defaultHotkey] : [];
}
