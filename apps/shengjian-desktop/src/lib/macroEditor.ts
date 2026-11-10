import type { Component } from "vue";
import {
  AppWindow,
  CircleAlert,
  Focus,
  Globe2,
  Hourglass,
  Keyboard,
  LockKeyhole,
  Workflow,
} from "lucide-vue-next";
import { MACRO_ACTIONS, MACRO_MODIFIERS } from "./macros";
import { tr, trDynamic } from "./i18n";
import type { MacroChord, MacroStep } from "./types";

export const ACTIONS = MACRO_ACTIONS;
export const MODIFIERS = MACRO_MODIFIERS;

export const KEY_LABELS: Record<string, string> = {
  space: "Space",
  enter: "Enter",
  tab: "Tab",
  backspace: "Backspace",
  delete: "Delete",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
};

const ACTION_ICONS: Record<string, Component> = {
  send_shortcut: Keyboard,
  open_application: AppWindow,
  open_url: Globe2,
  focus_input: Focus,
  focus_frontmost: Focus,
  wait_for_application: Hourglass,
  system_action: LockKeyhole,
  run_macro: Workflow,
};

const ACTION_BY_ID = new Map<string, (typeof ACTIONS)[number]>(
  ACTIONS.map((action) => [action.id, action]),
);

/**
 * Categorical colour per action family so steps are easy to scan.
 * Hues stay within the app palette and never carry status meaning.
 */
const ACTION_TONES: Record<string, string> = {
  open_application: "green",
  wait_for_application: "amber",
  focus_input: "blue",
  focus_frontmost: "blue",
  open_url: "blue",
  send_shortcut: "violet",
  system_action: "violet",
  run_macro: "accent",
};

export function actionLabel(actionId: string): string {
  const label = ACTION_BY_ID.get(actionId)?.label;
  return label ? trDynamic(label) : actionId;
}

export function actionIcon(actionId: string): Component {
  return ACTION_ICONS[actionId] ?? CircleAlert;
}

export function actionTone(actionId: string): string {
  return ACTION_TONES[actionId] ?? "neutral";
}

/**
 * System actions are a fixed set of OS/media keys; the runtime only maps these
 * names, so the editor offers them directly instead of free chord capture.
 */
export const SYSTEM_KEYS = [
  { key: "volume_up", label: "音量 +" },
  { key: "volume_down", label: "音量 −" },
  { key: "volume_mute", label: "静音" },
  { key: "media_play_pause", label: "播放 / 暂停" },
  { key: "media_prev", label: "上一曲" },
  { key: "media_next", label: "下一曲" },
  { key: "escape", label: "Esc" },
] as const;

export function systemKeyLabel(key: string): string {
  const match = SYSTEM_KEYS.find((item) => item.key === key);
  return match ? trDynamic(match.label) : key;
}

export function chord(step: MacroStep): MacroChord {
  return step.parameters.chord ?? { modifiers: [], key: "" };
}

export function chordParts(value: MacroChord): string[] {
  const parts = value.modifiers
    .map((modifier) => MODIFIERS.find((item) => item.id === modifier)?.label ?? modifier)
    .filter(Boolean);
  const key =
    KEY_LABELS[value.key] ?? (value.key.length === 1 ? value.key.toUpperCase() : value.key);
  if (key) parts.push(key);
  return parts;
}

export function chordLabel(value: MacroChord): string {
  return chordParts(value).join(" + ");
}

export function stepSummary(step: MacroStep): string {
  switch (step.action) {
    case "send_shortcut":
      return chordLabel(chord(step)) || tr("未设置快捷键");
    case "system_action":
      return chord(step).key.trim() ? systemKeyLabel(chord(step).key) : tr("未选择系统动作");
    case "open_application":
    case "focus_input":
      return step.parameters.target || tr("未设置应用目标");
    case "focus_frontmost":
      return tr("当前前台输入框");
    case "wait_for_application":
      return `${step.parameters.target || tr("未设置应用目标")} · ${
        step.parameters.state === "visible_window" ? tr("窗口可见") : tr("进程运行")
      }`;
    case "open_url":
      return step.parameters.url || tr("未设置网址");
    case "run_macro":
      return step.parameters.nestedMacroName || tr("未选择目标宏");
    default:
      return tr("未设置参数");
  }
}

/**
 * Only actions whose runtime implementation waits on something use a step
 * timeout. Shortcut, focus-frontmost and open-url steps run immediately.
 */
const TIMEOUT_ACTIONS = new Set(["open_application", "wait_for_application", "run_macro"]);

export function actionUsesTimeout(actionId: string): boolean {
  return TIMEOUT_ACTIONS.has(actionId);
}

export function stepIssue(step: MacroStep): string {
  const maxTimeout = step.action === "run_macro" ? 60000 : 10000;
  if (
    actionUsesTimeout(step.action) &&
    step.timeoutMs !== null &&
    step.timeoutMs !== undefined &&
    (step.timeoutMs < 100 || step.timeoutMs > maxTimeout)
  ) {
    return tr("步骤超时需在 100–{max}ms 之间", { max: maxTimeout });
  }
  switch (step.action) {
    case "send_shortcut":
      return chord(step).key.trim() ? "" : tr("请填写快捷键按键");
    case "system_action":
      return chord(step).key.trim() ? "" : tr("请选择系统动作");
    case "open_application":
    case "focus_input":
    case "wait_for_application":
      return (step.parameters.target ?? "").trim() ? "" : tr("请填写应用目标");
    case "open_url":
      return /^https?:\/\/\S+/i.test(step.parameters.url ?? "")
        ? ""
        : tr("网址必须以 http:// 或 https:// 开头");
    case "run_macro":
      return (step.parameters.nestedMacroId ?? "").trim() ? "" : tr("请选择目标宏");
    default:
      return "";
  }
}

export function runStepTone(outcome: string): string {
  if (outcome === "done") return "done";
  if (outcome === "cancelled") return "neutral";
  if (outcome === "unknown" || outcome === "completed_with_unknown_result") return "warning";
  return "danger";
}
