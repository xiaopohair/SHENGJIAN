import type { MacroDefinition, MacroParameters, MacroStep } from "./types";

export interface MacroDraft {
  macroId: string;
  name: string;
  summary: string;
  version: string;
  steps: MacroStep[];
}

export const MACRO_ACTIONS = [
  { id: "send_shortcut", label: "发送快捷键" },
  { id: "open_application", label: "打开应用" },
  { id: "open_url", label: "打开网址" },
  { id: "focus_input", label: "聚焦应用输入框" },
  { id: "focus_frontmost", label: "聚焦前台输入框" },
  { id: "wait_for_application", label: "等待应用运行" },
  { id: "system_action", label: "系统动作" },
  { id: "run_macro", label: "运行其他宏" },
] as const;

export const MACRO_MODIFIERS = [
  { id: "control", label: "Ctrl" },
  { id: "alt", label: "Alt" },
  { id: "shift", label: "Shift" },
  { id: "win", label: "Win" },
] as const;

export function defaultMacroParameters(action: string): MacroParameters {
  switch (action) {
    case "open_application":
    case "focus_input":
      return { target: "" };
    case "wait_for_application":
      return { target: "", state: "running" };
    case "send_shortcut":
    case "system_action":
      // No prefilled chord: the user records the one they want.
      return { chord: { modifiers: [], key: "" } };
    case "open_url":
      return { url: "https://" };
    case "run_macro":
      return { nestedMacroId: "", nestedMacroName: "" };
    default:
      return {};
  }
}

export function createMacroStep(action = "send_shortcut"): MacroStep {
  return {
    stepId: `step-${crypto.randomUUID()}`,
    action,
    parameters: defaultMacroParameters(action),
  };
}

export function compareMacroVersions(left: string, right: string): number {
  const parse = (value: string) => value.split(".").map((part) => Number(part) || 0);
  const a = parse(left);
  const b = parse(right);
  for (let index = 0; index < 3; index += 1) {
    const difference = (a[index] ?? 0) - (b[index] ?? 0);
    if (difference !== 0) return difference;
  }
  return 0;
}

/** One definition per macro ID, highest semver first. */
export function latestMacroDefinitions(definitions: MacroDefinition[]): MacroDefinition[] {
  const seen = new Set<string>();
  return definitions
    .slice()
    .sort((left, right) => compareMacroVersions(right.version, left.version))
    .filter((definition) => {
      if (seen.has(definition.macroId)) return false;
      seen.add(definition.macroId);
      return true;
    });
}

export function nextMacroVersion(definitions: MacroDefinition[], macroId: string): string {
  const latest = latestMacroDefinitions(definitions).find(
    (definition) => definition.macroId === macroId,
  );
  if (!latest) return "1.0.0";
  return bumpMacroVersion(latest.version);
}

/** Increment the patch part of a semver-like version. */
export function bumpMacroVersion(version: string): string {
  const parts = version.split(".").map((part) => Number(part) || 0);
  while (parts.length < 3) parts.push(0);
  parts[2] += 1;
  return parts.slice(0, 3).join(".");
}
