import { listen, type UnlistenFn } from "@tauri-apps/api/event";

/**
 * 组合键录入的**钩子层**通道（`shortcut-capture` 事件）。
 *
 * 前端 DOM 的 `keydown`/`keyup` 拿不到完整组合：输入法会把自己的语音和弦「吞掉 +
 * 重放」，被吞掉的那半个组合只以**注入副本**的形式在系统键盘钩子层可见
 * （2026-09-27 真机：左 Ctrl + 左 Win 只录到左 Ctrl，每会话恰好 4 条注入副本）。
 *
 * 所以录入期间平台侧打开 shortcut capture：钩子吞下物理边沿、把每一条（含注入副本）
 * 以本模块订阅的事件发上来，前端按这份 ground truth 组装组合。与
 * `shortcutCapture.ts`（DOM `KeyboardEvent` → 宏组合）是两件事，互不替代。
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
    volume_mute: "AudioVolumeMute",
    volume_down: "AudioVolumeDown",
    volume_up: "AudioVolumeUp",
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
