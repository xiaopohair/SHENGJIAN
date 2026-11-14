import { computed, onUnmounted, shallowRef, type Ref } from "vue";
import { tr, trDynamic } from "../lib/i18n";

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
  let pressed = new Set<string>();
  let captured: string[] = [];

  const effective = computed(() => options.hotkey.value ?? options.fallback());
  const hasCustom = computed(() => (options.hotkey.value?.length ?? 0) > 0);
  const label = computed(() => (recording.value ? tr("请按下组合键…") : format(effective.value)));

  function format(keys: string[]): string {
    if (keys.length === 0) return tr("不注入热键");
    return keys.map(hotkeyKeyLabel).join(" + ");
  }

  /** 用已捕获的按键结算并结束录制；`commit=false` 表示丢弃。 */
  function settle(commit: boolean) {
    if (commit && captured.length > 0) {
      options.hotkey.value = captured.slice(0, MAX_HOTKEY_KEYS);
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

  function onKeydown(event: KeyboardEvent) {
    if (!recording.value) return;
    // 输入法组字期间不采集：这些键是给输入法的，抢过来只会打架。
    if (event.isComposing) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.code === "Escape") {
      settle(false);
      return;
    }
    if (event.repeat || !isSupportedHotkeyCode(event.code)) return;
    pressed.add(event.code);
    if (!captured.includes(event.code)) captured.push(event.code);
    if (!MODIFIER_CODES.has(event.code)) finish();
  }

  function onKeyup(event: KeyboardEvent) {
    if (!recording.value) return;
    event.preventDefault();
    event.stopPropagation();
    pressed.delete(event.code);
    // A chord of modifiers only commits once every modifier is released.
    if (pressed.size === 0 && captured.length > 0 && captured.every((c) => MODIFIER_CODES.has(c))) {
      finish();
    }
  }

  function start() {
    recording.value = true;
    pressed = new Set();
    captured = [];
    window.addEventListener("keydown", onKeydown, true);
    window.addEventListener("keyup", onKeyup, true);
    window.addEventListener("blur", onWindowBlur);
  }

  function stop() {
    recording.value = false;
    pressed = new Set();
    captured = [];
    window.removeEventListener("keydown", onKeydown, true);
    window.removeEventListener("keyup", onKeyup, true);
    window.removeEventListener("blur", onWindowBlur);
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
