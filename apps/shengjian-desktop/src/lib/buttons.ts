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
