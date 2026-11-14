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

/**
 * 修饰键（区分左右，顺序即最终组合里的顺序）。
 *
 * 后端解析支持 `leftctrl`/`rightctrl` 这类名字（`is_modifier_key` 同词表），
 * 注入时分别按左右 Ctrl/Shift/Alt/Win 按下。
 */
export const CHORD_MODIFIER_OPTIONS: ChordModifierOption[] = [
  { code: "leftctrl", label: "左 Ctrl" },
  { code: "rightctrl", label: "右 Ctrl" },
  { code: "leftshift", label: "左 Shift" },
  { code: "rightshift", label: "右 Shift" },
  { code: "leftalt", label: "左 Alt" },
  { code: "rightalt", label: "右 Alt" },
  { code: "leftwin", label: "左 Win" },
  { code: "rightwin", label: "右 Win" },
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
