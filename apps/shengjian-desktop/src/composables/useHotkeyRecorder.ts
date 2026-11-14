import { computed, onUnmounted, shallowRef, type Ref } from "vue";
import { tr, trDynamic } from "../lib/i18n";
import { beginShortcutCapture, endShortcutCapture } from "../lib/api";
import { listenShortcutCapture } from "../lib/captureBridge";

/** Keys that only modify a chord; recording keeps waiting for a real key. */
const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "AltLeft",
  "AltRight",
  "ShiftLeft",
  "ShiftRight",
  "MetaLeft",
  "MetaRight",
]);

/** Codes shown with a friendly label instead of the raw code. */
const HOTKEY_LABELS: Record<string, string> = {
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
  NumpadEnter: "小键盘回车",
  Tab: "Tab",
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
  Escape: "Esc",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
};

/** Codes allowed as the main key of a hotkey (modifiers are not main keys). */
const MAIN_HOTKEY_CODES = new Set(
  Object.keys(HOTKEY_LABELS).filter((code) => !MODIFIER_CODES.has(code)),
);

const MAX_HOTKEY_KEYS = 8;

/**
 * 纯修饰键组合的结算宽限（毫秒）。
 *
 * 现场：先按 ControlLeft、松手，再按 ControlRight —— 松开第一个修饰键的瞬间
 * "没有按键按着了"，旧实现立刻结算成 `Ctrl`，第二个 Ctrl 永远进不来。
 * 留一段宽限：期间再按修饰键就并进同一个组合；没人接着按才真正结算。
 */
const MODIFIER_COMMIT_DELAY_MS = 320;

/** 修饰键规范顺序：同一组按键不论先后都得到相同结果。 */
const MODIFIER_ORDER = [
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
];

/** 把捕获到的按键排成稳定顺序：修饰键按规范次序，其余保持按下顺序。 */
function normalizeHotkey(keys: string[]): string[] {
  const modifiers = MODIFIER_ORDER.filter((code) => keys.includes(code));
  const others = keys.filter((code) => !MODIFIER_ORDER.includes(code));
  return [...modifiers, ...others];
}

function isSupportedHotkeyCode(code: string): boolean {
  return (
    MODIFIER_CODES.has(code) ||
    MAIN_HOTKEY_CODES.has(code) ||
    /^Key[A-Z]$/.test(code) ||
    /^Digit[0-9]$/.test(code) ||
    /^F([1-9]|1[0-2])$/.test(code) ||
    /^Numpad[0-9]$/.test(code)
  );
}

function hotkeyKeyLabel(key: string): string {
  if (HOTKEY_LABELS[key]) return trDynamic(HOTKEY_LABELS[key]);
  if (key.startsWith("Key")) return key.slice(3);
  if (key.startsWith("Digit")) return key.slice(5);
  if (/^Numpad[0-9]$/.test(key)) return `Num ${key.slice(6)}`;
  return key;
}

export interface HotkeyRecorderOptions {
  /** Stored custom hotkey codes; `null` means "use the tool default". */
  hotkey: Ref<string[] | null>;
  /** Codes used when no custom hotkey is stored. */
  fallback: () => string[];
}

/**
 * Records a hotkey chord from the keyboard while active.
 *
 * Owns the window listeners and their cleanup, so the page only renders the
 * keycaps and toggles `recording`.
 */
export function useHotkeyRecorder(options: HotkeyRecorderOptions) {
  const recording = shallowRef(false);
  /** 录入期是否已经把输入法让位出去（用来保证成对恢复）。 */
  let imeYielded = false;
  /** 平台钩子层录入通道是否生效（生效时以它为准，DOM 事件不再参与）。 */
  let osCapture = false;
  let unlistenHook: (() => void) | undefined;
  let pressed = new Set<string>();
  let captured: string[] = [];

  const effective = computed(() => options.hotkey.value ?? options.fallback());
  const hasCustom = computed(() => (options.hotkey.value?.length ?? 0) > 0);
  const label = computed(() => (recording.value ? tr("请按下组合键…") : format(effective.value)));

  function format(keys: string[]): string {
    if (keys.length === 0) return tr("不注入热键");
    return keys.map(hotkeyKeyLabel).join(" + ");
  }

  let commitTimer: number | undefined;

  function clearCommitTimer() {
    if (commitTimer === undefined) return;
    window.clearTimeout(commitTimer);
    commitTimer = undefined;
  }

  /** 用已捕获的按键结算并结束录制；`commit=false` 表示丢弃。 */
  function settle(commit: boolean) {
    clearCommitTimer();
    if (commit && captured.length > 0) {
      options.hotkey.value = normalizeHotkey(captured).slice(0, MAX_HOTKEY_KEYS);
    }
    stop();
  }

  function finish() {
    settle(true);
  }

  /**
   * 窗口失焦时结算。
   *
   * 采集组合键时按下的键会被系统或别的程序抢走（Win 开开始菜单、输入法弹面板等），
   * 焦点一旦离开本窗口，`keyup` 就收不到了——纯修饰键的组合（Ctrl+Win）永远等不到
   * 「全部松开」那一刻，录制会一直挂在「请按下组合键…」。这里用已捕获的按键结算，
   * 保证采集能结束且不丢已按下的键。
   */
  function onWindowBlur() {
    if (!recording.value) return;
    settle(true);
  }

  /** 一条按键边沿（DOM 与钩子层共用）。 */
  function handleKey(code: string, isDown: boolean) {
    if (!recording.value) return;
    if (isDown) {
      if (code === "Escape") {
        settle(false);
        return;
      }
      if (!isSupportedHotkeyCode(code)) return;
      // 又来新键了：取消上一次「纯修饰键待结算」，并进同一个组合。
      clearCommitTimer();
      pressed.add(code);
      if (!captured.includes(code)) captured.push(code);
      if (!MODIFIER_CODES.has(code)) finish();
      return;
    }
    pressed.delete(code);
    // 纯修饰键组合：等一小段宽限再结算，期间按下别的修饰键会并进同一组合。
    if (pressed.size === 0 && captured.length > 0 && captured.every((c) => MODIFIER_CODES.has(c))) {
      clearCommitTimer();
      commitTimer = window.setTimeout(() => {
        commitTimer = undefined;
        finish();
      }, MODIFIER_COMMIT_DELAY_MS);
    }
  }

  function onKeydown(event: KeyboardEvent) {
    if (!recording.value) return;
    // 钩子层通道生效时以它为准：输入法吞掉的那半个组合只在那里可见。
    if (osCapture) return;
    // 输入法组字期间不采集：这些键是给输入法的，抢过来只会打架。
    if (event.isComposing) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.repeat) return;
    handleKey(event.code, true);
  }

  function onKeyup(event: KeyboardEvent) {
    if (!recording.value) return;
    if (osCapture) return;
    event.preventDefault();
    event.stopPropagation();
    handleKey(event.code, false);
  }

  /**
   * 录入期让输入法让位。
   *
   * 输入法的键盘钩子先于本应用安装，只在自身是活动输入法时拦截语音和弦——不让位的
   * 话用户按的修饰键会被它直接吃掉，前端连 keydown 都收不到（现场：Ctrl+右Ctrl
   * 只录到右 Ctrl）。平台侧会把录入窗口线程的输入区域切到非 IME 布局，恢复时切回。
   */
  async function yieldInputMethod() {
    if (imeYielded) return;
    imeYielded = true;
    try {
      // 先订阅边沿，再打开吞键：反过来的话最早几条边沿会丢。
      unlistenHook = await listenShortcutCapture((code, isDown) => handleKey(code, isDown));
      osCapture = await beginShortcutCapture();
    } catch {
      unlistenHook?.();
      unlistenHook = undefined;
      osCapture = false;
      imeYielded = false;
    }
  }

  function restoreInputMethod() {
    unlistenHook?.();
    unlistenHook = undefined;
    osCapture = false;
    if (!imeYielded) return;
    imeYielded = false;
    void endShortcutCapture().catch(() => undefined);
  }

  function start() {
    recording.value = true;
    pressed = new Set();
    captured = [];
    void yieldInputMethod();
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("keyup", onKeyup, true);
    window.addEventListener("blur", onWindowBlur);
  }

  function stop() {
    recording.value = false;
    clearCommitTimer();
    pressed = new Set();
    captured = [];
    window.removeEventListener("keydown", onKeydown, true);
    window.removeEventListener("keyup", onKeyup, true);
    window.removeEventListener("blur", onWindowBlur);
    restoreInputMethod();
  }

  function toggle() {
    if (recording.value) stop();
    else start();
  }

  function reset() {
    stop();
    options.hotkey.value = null;
  }

  onUnmounted(stop);

  return { recording, label, hasCustom, toggle, reset };
}
