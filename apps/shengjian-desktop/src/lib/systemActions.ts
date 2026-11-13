/**
 * 系统动作（媒体/音量键）目录：宏编辑器与按键方案共用同一套 key 与展示文案。
 *
 * `key` 是后端运行时识别的固定名字（宏里存 `parameters.chord.key`，
 * 按键方案里存 `LogicalAction.key`），因此两处必须完全一致。
 */
import type { Component } from "vue";
import { Play, SkipBack, SkipForward, Volume1, Volume2, VolumeX } from "lucide-vue-next";
import { trDynamic } from "./i18n";

export interface SystemActionDefinition {
  /** 后端运行时使用的固定 key。 */
  key: string;
  /** 中文源文案；展示时走 trDynamic 翻译。 */
  label: string;
  /** 卡片图标；没有图标时退回键帽文字。 */
  icon?: Component;
  /** 无图标时显示的键帽文字（如 Esc）。 */
  keycap?: string;
}

export const SYSTEM_KEYS: readonly SystemActionDefinition[] = [
  { key: "volume_up", label: "音量 +", icon: Volume2 },
  { key: "volume_down", label: "音量 −", icon: Volume1 },
  { key: "volume_mute", label: "静音", icon: VolumeX },
  { key: "media_play_pause", label: "播放 / 暂停", icon: Play },
  { key: "media_prev", label: "上一曲", icon: SkipBack },
  { key: "media_next", label: "下一曲", icon: SkipForward },
  { key: "escape", label: "Esc", keycap: "Esc" },
];

const SYSTEM_KEY_SET = new Set(SYSTEM_KEYS.map((item) => item.key));

/** 是否是运行时认识的系统动作 key（用于校验，不用于自由输入）。 */
export function isSystemActionKey(key: string): boolean {
  return SYSTEM_KEY_SET.has(key);
}

export function systemKeyLabel(key: string): string {
  const match = SYSTEM_KEYS.find((item) => item.key === key);
  return match ? trDynamic(match.label) : key;
}
