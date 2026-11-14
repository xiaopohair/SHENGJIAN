/**
 * 组合键点选面板的候选项（宏/动作快捷键用的那套词表）。
 *
 * 键名与宏运行时一致（`a`、`1`、`f5`、`space`、`;`…），与
 * `lib/shortcutCapture.ts` 的 `normalizeShortcutKey` 同一套词表。
 */

export interface ChordModifierOption {
  /** 宏运行时的修饰键名。 */
  code: string;
  label: string;
}

/** 修饰键（顺序即最终组合里的顺序）。 */
export const CHORD_MODIFIER_OPTIONS: ChordModifierOption[] = [
  { code: "control", label: "Ctrl" },
  { code: "shift", label: "Shift" },
  { code: "alt", label: "Alt" },
  { code: "win", label: "Win" },
];

export interface ChordKeyOption {
  /** 宏运行时的键名。 */
  key: string;
  label: string;
}

export interface ChordKeyGroup {
  label: string;
  keys: ChordKeyOption[];
}

const letters = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
  .split("")
  .map((letter): ChordKeyOption => ({ key: letter.toLowerCase(), label: letter }));

const digits = "0123456789"
  .split("")
  .map((digit): ChordKeyOption => ({ key: digit, label: digit }));

const functionKeys = Array.from({ length: 12 }, (_, index): ChordKeyOption => ({
  key: `f${index + 1}`,
  label: `F${index + 1}`,
}));

const common: ChordKeyOption[] = [
  { key: "space", label: "空格" },
  { key: "enter", label: "回车" },
  { key: "tab", label: "Tab" },
  { key: "escape", label: "Esc" },
  { key: "backspace", label: "退格" },
  { key: "delete", label: "Delete" },
  { key: "insert", label: "Insert" },
  { key: "home", label: "Home" },
  { key: "end", label: "End" },
  { key: "pageup", label: "PageUp" },
  { key: "pagedown", label: "PageDown" },
  { key: "up", label: "↑" },
  { key: "down", label: "↓" },
  { key: "left", label: "←" },
  { key: "right", label: "→" },
  { key: ";", label: ";" },
  { key: "=", label: "=" },
  { key: ",", label: "," },
  { key: "-", label: "-" },
  { key: ".", label: "." },
  { key: "/", label: "/" },
  { key: "`", label: "`" },
  { key: "[", label: "[" },
  { key: "\\", label: "\\" },
  { key: "]", label: "]" },
  { key: "'", label: "'" },
];

export const CHORD_KEY_GROUPS: ChordKeyGroup[] = [
  { label: "常用", keys: common },
  { label: "字母", keys: letters },
  { label: "数字", keys: digits },
  { label: "功能键", keys: functionKeys },
];
