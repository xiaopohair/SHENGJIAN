import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 组合键录入的**钩子层**通道（`shortcut-capture` 事件）。
 *
 * 前端 DOM 的 `keydown` 拿不到完整组合：输入法会把自己的语音和弦「吞掉 +
 * 重放」，被吞掉的那半个组合只以**注入副本**的形式在系统键盘钩子层可见
 * （2026-09-27 真机：左 Ctrl + 左 Win 只录到左 Ctrl，而每会话恰好 4 条注入副本）。
 *
 * 所以录入期间打开平台侧 shortcut capture：钩子吞下物理边沿、把每一条（含注入副本）
 * 以本模块订阅的事件发上来，前端按这份 ground truth 组装组合。与
 * `shortcutCapture.ts`（DOM `KeyboardEvent` → 宏组合）配合使用：钩子通道不可用时退回 DOM。
 */

export interface ShortcutCaptureEdge {
  /** `KeyCode` 的 snake_case 序列化值，例如 `left_control`。 */
  key: string;
  isPressed: boolean;
  /** `real` = 物理边沿；`injected` = 输入法重放的副本。 */
  source: "real" | "injected";
}

/** `KeyCode`（snake_case）→ 浏览器 `KeyboardEvent.code`；映射不到返回 null。 */
export function domCodeForKeyCode(key: string): string | null {
  const table: Record<string, string> = {
    left_control: "ControlLeft",
    right_control: "ControlRight",
    left_shift: "ShiftLeft",
    right_shift: "ShiftRight",
    left_alt: "AltLeft",
    right_alt: "AltRight",
    left_windows: "MetaLeft",
    right_windows: "MetaRight",
    backspace: "Backspace",
    tab: "Tab",
    enter: "Enter",
    escape: "Escape",
    space: "Space",
    page_up: "PageUp",
    page_down: "PageDown",
    end: "End",
    home: "Home",
    left: "ArrowLeft",
    up: "ArrowUp",
    right: "ArrowRight",
    down: "ArrowDown",
    insert: "Insert",
    delete: "Delete",
    apps: "ContextMenu",
    oem_1: "Semicolon",
    oem_plus: "Equal",
    oem_comma: "Comma",
    oem_minus: "Minus",
    oem_period: "Period",
    oem_2: "Slash",
    oem_3: "Backquote",
    oem_4: "BracketLeft",
    oem_5: "Backslash",
    oem_6: "BracketRight",
    oem_7: "Quote",
    numpad_multiply: "NumpadMultiply",
    numpad_add: "NumpadAdd",
    numpad_subtract: "NumpadSubtract",
    numpad_decimal: "NumpadDecimal",
    numpad_divide: "NumpadDivide",
    num_lock: "NumLock",
  };
  const mapped = table[key];
  if (mapped) return mapped;
  if (/^[a-z]$/.test(key)) return `Key${key.toUpperCase()}`;
  if (/^digit[0-9]$/.test(key)) return `Digit${key.slice(5)}`;
  if (/^f([1-9]|1[0-2])$/.test(key)) return key.toUpperCase();
  if (/^numpad[0-9]$/.test(key)) return `Numpad${key.slice(6)}`;
  return null;
}

/** 该 DOM code 是不是修饰键；是则返回宏运行时用的修饰键名。 */
export function modifierForDomCode(code: string): string | null {
  if (code === "ControlLeft" || code === "ControlRight") return "control";
  if (code === "ShiftLeft" || code === "ShiftRight") return "shift";
  if (code === "AltLeft" || code === "AltRight") return "alt";
  if (code === "MetaLeft" || code === "MetaRight") return "win";
  return null;
}

const CODE_KEYS: Record<string, string> = {
  Space: "space",
  Enter: "enter",
  NumpadEnter: "enter",
  Tab: "tab",
  Backspace: "backspace",
  Delete: "delete",
  Insert: "insert",
  Home: "home",
  End: "end",
  PageUp: "pageup",
  PageDown: "pagedown",
  ArrowUp: "up",
  ArrowDown: "down",
  ArrowLeft: "left",
  ArrowRight: "right",
  Semicolon: ";",
  Equal: "=",
  Comma: ",",
  Minus: "-",
  Period: ".",
  Slash: "/",
  Backquote: "`",
  BracketLeft: "[",
  Backslash: "\\",
  BracketRight: "]",
  Quote: "'",
  NumpadMultiply: "numpadmultiply",
  NumpadAdd: "numpadadd",
  NumpadSubtract: "numpadsubtract",
  NumpadDecimal: "numpaddecimal",
  NumpadDivide: "numpaddivide",
};

/** DOM code → 宏运行时的键名（与 `normalizeShortcutKey` 同一套词表）。 */
export function macroKeyFromDomCode(code: string): string | null {
  if (/^Key[A-Z]$/.test(code)) return code.slice(3).toLowerCase();
  if (/^Digit[0-9]$/.test(code)) return code.slice(5).toLowerCase();
  if (/^F([1-9]|1[0-2])$/.test(code)) return code.toLowerCase();
  if (/^Numpad[0-9]$/.test(code)) return code.toLowerCase();
  return CODE_KEYS[code] ?? null;
}

/** 订阅钩子层录入边沿；返回取消订阅函数。 */
export async function listenShortcutCapture(
  handler: (code: string, pressed: boolean) => void,
): Promise<UnlistenFn> {
  return listen<ShortcutCaptureEdge>("shortcut-capture", ({ payload }) => {
    const code = domCodeForKeyCode(String(payload?.key ?? ""));
    if (!code) return;
    handler(code, Boolean(payload?.isPressed));
  });
}
