/** @vitest-environment jsdom */
import { describe, expect, it, vi } from "vitest";
import { defineComponent, h, ref } from "vue";
import { mount } from "@vue/test-utils";
import { useHotkeyRecorder } from "./useHotkeyRecorder";

const beginShortcutCapture = vi.fn(async () => undefined);
const endShortcutCapture = vi.fn(async () => undefined);
vi.mock("../lib/api", () => ({
  beginShortcutCapture: () => beginShortcutCapture(),
  endShortcutCapture: () => endShortcutCapture(),
}));
import { setLanguagePreference } from "../lib/i18n";

setLanguagePreference("zh-Hans");

function mountRecorder(initial: string[] | null = null) {
  const hotkey = ref<string[] | null>(initial);
  const api = {} as ReturnType<typeof useHotkeyRecorder>;
  const Host = defineComponent({
    setup() {
      Object.assign(api, useHotkeyRecorder({ hotkey, fallback: () => [] }));
      return () => h("div");
    },
  });
  const wrapper = mount(Host, { attachTo: document.body });
  return { wrapper, hotkey, api };
}

function keyEvent(type: "keydown" | "keyup", code: string, extra: KeyboardEventInit = {}) {
  const event = new KeyboardEvent(type, {
    code,
    key: code,
    bubbles: true,
    cancelable: true,
    ...extra,
  });
  window.dispatchEvent(event);
  return event;
}

describe("useHotkeyRecorder", () => {
  it("commits a chord on the first non-modifier key", () => {
    const { wrapper, hotkey, api } = mountRecorder();
    api.toggle();
    keyEvent("keydown", "ControlLeft");
    keyEvent("keydown", "KeyM");
    expect(hotkey.value).toEqual(["ControlLeft", "KeyM"]);
    expect(api.recording.value).toBe(false);
    wrapper.unmount();
  });

  it("commits a modifiers-only chord after the grace window", () => {
    vi.useFakeTimers();
    const { wrapper, hotkey, api } = mountRecorder();
    api.toggle();
    keyEvent("keydown", "ControlLeft");
    keyEvent("keydown", "MetaLeft");
    expect(hotkey.value).toBeNull();
    keyEvent("keyup", "ControlLeft");
    keyEvent("keyup", "MetaLeft");
    // 松手后不立刻结算：留宽限给「再补一个修饰键」。
    expect(hotkey.value).toBeNull();
    vi.advanceTimersByTime(400);
    expect(hotkey.value).toEqual(["ControlLeft", "MetaLeft"]);
    vi.useRealTimers();
    wrapper.unmount();
  });

  it("joins a second modifier pressed after the first was released", () => {
    // 现场：先按 ControlLeft 松手，再按 ControlRight，旧实现只录到 Ctrl。
    vi.useFakeTimers();
    const { wrapper, hotkey, api } = mountRecorder();
    api.toggle();
    keyEvent("keydown", "ControlLeft");
    keyEvent("keyup", "ControlLeft");
    vi.advanceTimersByTime(100);
    keyEvent("keydown", "ControlRight");
    keyEvent("keyup", "ControlRight");
    vi.advanceTimersByTime(400);
    expect(hotkey.value).toEqual(["ControlLeft", "ControlRight"]);
    vi.useRealTimers();
    wrapper.unmount();
  });

  it("orders modifiers canonically regardless of press order", () => {
    vi.useFakeTimers();
    const { wrapper, hotkey, api } = mountRecorder();
    api.toggle();
    keyEvent("keydown", "ControlRight");
    keyEvent("keydown", "ControlLeft");
    keyEvent("keyup", "ControlRight");
    keyEvent("keyup", "ControlLeft");
    vi.advanceTimersByTime(400);
    expect(hotkey.value).toEqual(["ControlLeft", "ControlRight"]);
    vi.useRealTimers();
    wrapper.unmount();
  });

  it("settles with captured keys when the window loses focus", () => {
    // 按 Win 会弹出开始菜单、输入法组合键会弹面板：焦点被抢走就再也收不到
    // keyup，旧实现会永远停在「请按下组合键…」。
    const { wrapper, hotkey, api } = mountRecorder();
    api.toggle();
    keyEvent("keydown", "ControlLeft");
    keyEvent("keydown", "MetaLeft");
    window.dispatchEvent(new Event("blur"));
    expect(api.recording.value).toBe(false);
    expect(hotkey.value).toEqual(["ControlLeft", "MetaLeft"]);
    wrapper.unmount();
  });

  it("keeps the old hotkey when losing focus without capturing anything", () => {
    const { wrapper, hotkey, api } = mountRecorder(["AltRight"]);
    api.toggle();
    window.dispatchEvent(new Event("blur"));
    expect(api.recording.value).toBe(false);
    expect(hotkey.value).toEqual(["AltRight"]);
    wrapper.unmount();
  });

  it("cancels on Escape without overwriting the stored hotkey", () => {
    const { wrapper, hotkey, api } = mountRecorder(["AltRight"]);
    api.toggle();
    keyEvent("keydown", "ControlLeft");
    keyEvent("keydown", "Escape");
    expect(api.recording.value).toBe(false);
    expect(hotkey.value).toEqual(["AltRight"]);
    wrapper.unmount();
  });

  it("yields the input method while recording and restores it after", () => {
    const { wrapper, api } = mountRecorder();
    beginShortcutCapture.mockClear();
    endShortcutCapture.mockClear();
    api.toggle();
    expect(beginShortcutCapture).toHaveBeenCalledTimes(1);
    api.toggle();
    expect(endShortcutCapture).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });

  it("restores the input method when recording settles", () => {
    const { wrapper, api } = mountRecorder();
    endShortcutCapture.mockClear();
    api.toggle();
    keyEvent("keydown", "ControlLeft");
    keyEvent("keydown", "KeyM");
    expect(api.recording.value).toBe(false);
    expect(endShortcutCapture).toHaveBeenCalledTimes(1);
    wrapper.unmount();
  });

  it("ignores keys that belong to an IME composition", () => {
    const { wrapper, hotkey, api } = mountRecorder();
    api.toggle();
    keyEvent("keydown", "KeyM", { isComposing: true });
    expect(hotkey.value).toBeNull();
    expect(api.recording.value).toBe(true);
    api.toggle();
    wrapper.unmount();
  });
});
