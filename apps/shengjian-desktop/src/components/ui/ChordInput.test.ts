/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import ChordInput from "./ChordInput.vue";
import { actionUsesTimeout, stepIssue } from "../../lib/macroEditor";
import { chordFromKeyboardEvent } from "../../lib/shortcutCapture";
import type { MacroChord, MacroStep } from "../../lib/types";

function keyEvent(init: KeyboardEventInit): KeyboardEvent {
  return new KeyboardEvent("keydown", { ...init, bubbles: true });
}

describe("ChordInput", () => {
  it("reads modifiers and the final key from a keydown event", () => {
    const chord = chordFromKeyboardEvent(
      keyEvent({ code: "KeyN", key: "n", ctrlKey: true, shiftKey: true }),
    );
    expect(chord).toEqual({ modifiers: ["control", "shift"], key: "n" });
  });

  it("maps non-printable keys by code", () => {
    expect(chordFromKeyboardEvent(keyEvent({ code: "F5", key: "F5" }))?.key).toBe("f5");
    expect(chordFromKeyboardEvent(keyEvent({ code: "ArrowUp", key: "ArrowUp" }))?.key).toBe("up");
  });

  it("opens the picker from the select button", async () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: [], key: "" } as MacroChord },
      attachTo: document.body,
    });
    await wrapper.find("button").trigger("click");
    expect(document.body.querySelector(".chord-dialog")).not.toBeNull();
    wrapper.unmount();
  });

  it("applies the picked chord", async () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: [], key: "" } as MacroChord },
      attachTo: document.body,
    });
    await wrapper.find("button").trigger("click");
    // 修饰键「左 Ctrl」+ 主键 K（修饰键区分左右）
    const chips = [...document.body.querySelectorAll<HTMLElement>(".chord-chip")];
    chips.find((chip) => chip.textContent?.trim() === "左 Ctrl")?.click();
    const keys = [...document.body.querySelectorAll<HTMLElement>(".chord-key")];
    keys.find((key) => key.textContent?.trim() === "K")?.click();
    await wrapper.vm.$nextTick();
    [...document.body.querySelectorAll<HTMLElement>(".chord-actions button")]
      .find((button) => button.textContent?.includes("确定"))
      ?.click();
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update:modelValue")?.[0]?.[0]).toEqual({
      modifiers: ["leftctrl"],
      key: "k",
    });
    wrapper.unmount();
  });

  it("clears a recorded shortcut", async () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: ["control"], key: "n" } as MacroChord },
    });
    const clearButton = wrapper.find('[aria-label="清除快捷键"]');
    expect(clearButton.exists()).toBe(true);
    await clearButton.trigger("click");
    expect(wrapper.emitted("update:modelValue")?.[0]?.[0]).toEqual({ modifiers: [], key: "" });
    wrapper.unmount();
  });

  it("offers no clear action when nothing is recorded", () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: [], key: "" } as MacroChord },
    });
    expect(wrapper.find('[aria-label="清除快捷键"]').exists()).toBe(false);
    wrapper.unmount();
  });
});

describe("step timeout scope", () => {
  const step = (action: string, timeoutMs: number | null): MacroStep => ({
    stepId: "step-1",
    action,
    timeoutMs,
    parameters: { target: "notepad.exe", chord: { modifiers: [], key: "n" } },
  });

  it("only marks actions that actually wait as timeout users", () => {
    expect(actionUsesTimeout("open_application")).toBe(true);
    expect(actionUsesTimeout("wait_for_application")).toBe(true);
    expect(actionUsesTimeout("run_macro")).toBe(true);
    expect(actionUsesTimeout("send_shortcut")).toBe(false);
    expect(actionUsesTimeout("focus_frontmost")).toBe(false);
    expect(actionUsesTimeout("open_url")).toBe(false);
  });

  it("ignores an out-of-range timeout on an action that does not use it", () => {
    expect(stepIssue(step("send_shortcut", 99))).toBe("");
    expect(stepIssue(step("open_application", 99))).not.toBe("");
  });
});
