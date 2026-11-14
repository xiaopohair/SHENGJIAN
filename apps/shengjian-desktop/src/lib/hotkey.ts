import { tr, trDynamic } from "./i18n";

/**
 * 语音热键的展示与规范化。
 *
 * 存储格式是前端 `KeyboardEvent.code` 列表（`ControlLeft`、`KeyA`…），
 * 与 `config/voice-tools.json` 的 `defaultHotkey` 同一套词表。
 */

/** 只修饰组合的键：选主键时不列它们。 */
export const MODIFIER_CODES = [
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
] as const;

const MODIFIER_ORDER: readonly string[] = MODIFIER_CODES;

/** 点选面板里的修饰键（左右合并成一个，选中用左侧码）。 */
export interface ModifierOption {
  code: string;
  label: string;
  /** 同义键：解析既有值时任一命中即视为已选。 */
  aliases: string[];
}

export const MODIFIER_OPTIONS: ModifierOption[] = [
  { code: "ControlLeft", label: "左 Ctrl", aliases: ["ControlLeft"] },
  { code: "ControlRight", label: "右 Ctrl", aliases: ["ControlRight"] },
  { code: "ShiftLeft", label: "左 Shift", aliases: ["ShiftLeft"] },
  { code: "ShiftRight", label: "右 Shift", aliases: ["ShiftRight"] },
  { code: "AltLeft", label: "左 Alt", aliases: ["AltLeft"] },
  { code: "AltRight", label: "右 Alt", aliases: ["AltRight"] },
  { code: "MetaLeft", label: "左 Win", aliases: ["MetaLeft"] },
  { code: "MetaRight", label: "右 Win", aliases: ["MetaRight"] },
];

/** 点选面板里的主键。 */
export const MAIN_KEY_GROUPS: { label: string; keys: string[] }[] = [
  {
    label: "常用",
    keys: [
      "Space",
      "Enter",
      "Tab",
      "Escape",
      "Backspace",
      "Delete",
      "Insert",
      "Home",
      "End",
      "PageUp",
      "PageDown",
      "ArrowUp",
      "ArrowDown",
      "ArrowLeft",
      "ArrowRight",
    ],
  },
  {
    label: "字母",
    keys: "ABCDEFGHIJKLMNOPQRSTUVWXYZ".split("").map((letter) => `Key${letter}`),
  },
  {
    label: "数字",
    keys: "0123456789".split("").map((digit) => `Digit${digit}`),
  },
  {
    label: "功能键",
    keys: Array.from({ length: 12 }, (_, index) => `F${index + 1}`),
  },
];

const LABELS: Record<string, string> = {
  ControlLeft: "Ctrl",
  ControlRight: "右 Ctrl",
  ShiftLeft: "Shift",
  ShiftRight: "右 Shift",
  AltLeft: "Alt",
  AltRight: "右 Alt",
  MetaLeft: "Win",
  MetaRight: "右 Win",
  Space: "空格",
  Enter: "回车",
  Tab: "Tab",
  Escape: "Esc",
  Backspace: "退格",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
};

/** 单个键 → 键帽文字（`KeyA` → `A`）。 */
export function hotkeyKeyLabel(code: string): string {
  if (LABELS[code]) return trDynamic(LABELS[code]);
  if (code.startsWith("Key")) return code.slice(3);
  if (code.startsWith("Digit")) return code.slice(5);
  return code;
}

/** 按键列表 → 「Ctrl + Shift + A」；空列表表示不注入热键。 */
export function formatHotkey(keys: readonly string[]): string {
  if (keys.length === 0) return tr("不注入热键");
  return keys.map(hotkeyKeyLabel).join(" + ");
}

/**
 * 规范顺序：修饰键按 Ctrl → Shift → Alt → Win，主键保持在最后。
 *
 * 同一组按键不论点选先后都得到相同结果，便于比较与判重。
 */
export function normalizeHotkey(keys: readonly string[]): string[] {
  const modifiers = MODIFIER_ORDER.filter((code) => keys.includes(code));
  const others = keys.filter((code) => !MODIFIER_ORDER.includes(code));
  return [...modifiers, ...others];
}

/**
 * 两份热键是否等效（忽略顺序）。
 *
 * 左右修饰键是**不同的键**（后端按 LeftControl/RightControl 分别注入），
 * 所以这里不做左右合并。
 */
export function sameHotkey(a: readonly string[], b: readonly string[]): boolean {
  return [...a].sort().join(",") === [...b].sort().join(",");
}

/** 把已存储的按键拆成「修饰键选择」+「主键」，供点选面板回填。 */
export function splitHotkey(keys: readonly string[]): {
  modifiers: string[];
  mainKey: string;
} {
  const modifiers = MODIFIER_OPTIONS.filter((option) =>
    keys.some((code) => option.aliases.includes(code)),
  ).map((option) => option.code);
  const mainKey = keys.find((code) => !MODIFIER_ORDER.includes(code)) ?? "";
  return { modifiers, mainKey };
}
