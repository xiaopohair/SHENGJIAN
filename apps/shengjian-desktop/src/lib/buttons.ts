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

/** 默认方案的固定显示名（后端强制，不随用户改名变化）。 */
export const DEFAULT_PROFILE_NAME = "默认";

/** 是否为不可删除的默认方案：每个设备型号各一份，外加一份通用兜底。 */
export function isDefaultProfile(profile: { isDefault?: boolean }): boolean {
  return Boolean(profile.isDefault);
}

/**
 * 某型号的按键布局。
 *
 * 真机确认 ARN9（普通款）**没有 TV 键**，其余位置与 RC003 一致；
 * 型号未知时给全量布局，保证所有位置都能编辑。
 */
export function profileButtonsForModel(
  model: string | null | undefined,
): Array<{ id: LogicalButton; label: string }> {
  const normalized = (model ?? "").trim().toUpperCase();
  if (normalized === "ARN9") {
    return PROFILE_BUTTONS.filter((button) => button.id !== "tv");
  }
  return PROFILE_BUTTONS;
}

/** 方案是否对给定型号可见（通用方案对所有型号可见）。 */
export function profileAppliesToModel(
  profile: { deviceModel?: string | null },
  model: string | null | undefined,
): boolean {
  const own = (profile.deviceModel ?? "").trim().toUpperCase();
  if (!own) return true;
  return own === (model ?? "").trim().toUpperCase();
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
  /** 归属设备型号（大写，如 "RC003"）；null = 通用方案。 */
  deviceModel?: string | null;
  /** 是否为该型号的默认方案。 */
  isDefault?: boolean;
  bindings: Record<string, Partial<Record<ProfileTrigger, LogicalAction>>>;
}
