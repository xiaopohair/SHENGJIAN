/**
 * 系统动作（媒体/音量/窗口键）目录：宏编辑器与按键方案共用同一套 key 与展示文案。
 *
 * `key` 是两处持久化的稳定 id：
 * - 宏：`parameters.chord` 存条目里的 `chord`（返回桌面/切换任务是组合键）；
 * - 按键方案：`LogicalAction.key` 存 id（后端把 `show_desktop` / `task_switch`
 *   展开成对应组合键，其余单键 id 原样注入）。
 */
import type { Component } from "vue";
import {
  Layers,
  Monitor,
  Play,
  SkipBack,
  SkipForward,
  Volume1,
  Volume2,
  VolumeX,
} from "lucide-vue-next";
import { trDynamic } from "./i18n";
import type { MacroChord } from "./types";

export interface SystemActionDefinition {
  /** 持久化 id（宏/按键方案共用；发布后不要改）。 */
  key: string;
  /** 中文源文案；展示时走 trDynamic 翻译。 */
  label: string;
  /** 卡片图标；没有图标时退回键帽文字。 */
  icon?: Component;
  /** 无图标时显示的键帽文字（如 Esc）。 */
  keycap?: string;
  /** 宏里实际保存/执行的组合键。 */
  chord: MacroChord;
}

const single = (key: string): MacroChord => ({ modifiers: [], key });

export const SYSTEM_KEYS: readonly SystemActionDefinition[] = [
  { key: "volume_up", label: "音量 +", icon: Volume2, chord: single("volume_up") },
  { key: "volume_down", label: "音量 −", icon: Volume1, chord: single("volume_down") },
  { key: "volume_mute", label: "静音", icon: VolumeX, chord: single("volume_mute") },
  { key: "media_play_pause", label: "播放 / 暂停", icon: Play, chord: single("media_play_pause") },
  { key: "media_prev", label: "上一曲", icon: SkipBack, chord: single("media_prev") },
  { key: "media_next", label: "下一曲", icon: SkipForward, chord: single("media_next") },
  {
    key: "show_desktop",
    label: "返回桌面",
    icon: Monitor,
    chord: { modifiers: ["win"], key: "d" },
  },
  {
    key: "task_switch",
    label: "切换任务",
    icon: Layers,
    chord: { modifiers: ["alt"], key: "tab" },
  },
  { key: "escape", label: "退出", keycap: "Esc", chord: single("escape") },
];

const SYSTEM_KEY_SET = new Set(SYSTEM_KEYS.map((item) => item.key));

/** 是否是运行时认识的系统动作 id（用于校验，不用于自由输入）。 */
export function isSystemActionKey(key: string): boolean {
  return SYSTEM_KEY_SET.has(key);
}

export function systemKeyLabel(key: string): string {
  const match = SYSTEM_KEYS.find((item) => item.key === key);
  return match ? trDynamic(match.label) : key;
}

function normalizeModifiers(modifiers: readonly string[] | undefined): string {
  return [...(modifiers ?? [])].map(String).sort().join("|");
}

/** 宏里的 chord 是否对应某个系统动作条目。 */
export function systemActionIdForChord(chord: MacroChord): string {
  const modifiers = normalizeModifiers(chord.modifiers);
  const match = SYSTEM_KEYS.find(
    (item) =>
      item.chord.key === chord.key && normalizeModifiers(item.chord.modifiers) === modifiers,
  );
  return match?.key ?? chord.key;
}

/** 宏摘要用：组合键（Win+D / Alt+Tab）也能显示成系统动作名称。 */
export function systemActionLabelForChord(chord: MacroChord): string | null {
  const key = systemActionIdForChord(chord);
  const match = SYSTEM_KEYS.find((item) => item.key === key);
  return match ? trDynamic(match.label) : null;
}

export function systemActionById(id: string): SystemActionDefinition | undefined {
  return SYSTEM_KEYS.find((item) => item.key === id);
}

/** 单键系统动作可按住连发；Win+D / Alt+Tab 这类组合键不连发。 */
export function systemActionAllowsRepeat(id: string): boolean {
  const definition = systemActionById(id);
  return definition !== undefined && definition.chord.modifiers.length === 0;
}
