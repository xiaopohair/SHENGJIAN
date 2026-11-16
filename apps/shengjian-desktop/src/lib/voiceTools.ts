/**
 * 语音输入工具目录的纯函数工具。
 *
 * 工具列表来自配置目录（`config/voice-tool-presets.json`）：`label` 是应用自己的名字，
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

/**
 * 换输入工具后仍然生效的自定义热键。
 *
 * 热键是**工具专属**的：豆包默认右 Alt、Chatterfly 默认修饰键组合，同一个组合
 * 换个工具通常就按不出来。若把旧值继续发给新工具，表现是「设置里保存成功、
 * 遥控器按了没反应」——所以换工具一律回到新工具的默认预设；同工具重选不清。
 */
export function customHotkeyAfterToolChange(
  previousTool: string | null | undefined,
  nextTool: string | null | undefined,
  custom: readonly string[] | null | undefined,
): string[] | null {
  const sameTool = (previousTool ?? "").trim() === (nextTool ?? "").trim();
  if (sameTool) return custom && custom.length > 0 ? [...custom] : null;
  return null;
}

/**
 * 该输入法是否**丢弃程序注入的按键**（`LLKHF_INJECTED`）。
 *
 * 豆包按设计丢弃注入键；Chatterfly 同样只认报告层的真实按键（2026-11-16 实测：
 * 注入的和弦稳定没反应，报告层替换同一和弦可拉起）。这类工具必须走"报告层合成"
 * （全按键支持服务）；没有它，声键用 SendInput 注入的和弦会被直接忽略，语音键
 * 看起来完全没反应。
 *
 * 与后端 `ble::ignores_injected_keys` 必须同集合：两侧漂移会让新手引导漏提示。
 */
export function requiresReportLayerSynthesis(toolId: string | null | undefined): boolean {
  const key = (toolId ?? "").trim().toLowerCase();
  return key === "doubao" || key === "chatterfly";
}
