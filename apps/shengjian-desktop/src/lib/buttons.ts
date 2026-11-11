import type { LogicalAction, LogicalButton } from "./types";

export const PROFILE_BUTTONS: Array<{ id: LogicalButton; label: string }> = [
  { id: "back", label: "返回" },
  { id: "ok", label: "OK" },
  { id: "home", label: "主页" },
  { id: "tv", label: "TV" },
  { id: "menu", label: "菜单" },
  { id: "left", label: "左" },
  { id: "right", label: "右" },
  { id: "down", label: "下" },
  { id: "up", label: "上" },
  { id: "power", label: "电源" },
  { id: "volume-up", label: "音量+" },
  { id: "volume-down", label: "音量-" },
];

/** 全局方案的固定 id：恒存在、排首位、不可删除、不关联应用。 */
export const GLOBAL_PROFILE_ID = "global";

/** 全局方案的显示名（后端固定，不随用户改名变化）。 */
export const GLOBAL_PROFILE_NAME = "全局";

/** 是否为不可删除的全局方案。 */
export function isGlobalProfile(profile: { id: string }): boolean {
  return profile.id === GLOBAL_PROFILE_ID;
}

export const PROFILE_TRIGGERS = ["single", "double", "long"] as const;
export type ProfileTrigger = (typeof PROFILE_TRIGGERS)[number];
export type ProfileActionType =
  | "inherit"
  | "disabled"
  | "focus-input"
  | "shortcut"
  | "open-app"
  | "common-phrase"
  | "mouse"
  | "run-macro"
  | "other";

export interface ProfileDraft {
  id: string;
  name: string;
  appTargets: string;
  autoApply: boolean;
  holdRepeat: "" | "single" | "long";
  holdRepeatByButton?: Partial<Record<LogicalButton, "off" | "single" | "long">>;
  voiceInputTool: string;
  bindings: Record<string, Partial<Record<ProfileTrigger, LogicalAction>>>;
}
