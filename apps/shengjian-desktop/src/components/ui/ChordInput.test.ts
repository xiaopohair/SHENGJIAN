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

  it("ignores modifier-only presses so multi-key chords keep working", () => {
    expect(chordFromKeyboardEvent(keyEvent({ code: "ControlLeft", key: "Control" }))).toBeNull();
  });

  it("keeps the profile editor's shortcut string format", () => {
    // The key set produces the same "control+shift+n" string the 按键 page stores.
    const chord = chordFromKeyboardEvent(
      keyEvent({ code: "KeyN", key: "n", ctrlKey: true, shiftKey: true }),
    );
    expect([...(chord?.modifiers ?? []), chord?.key].join("+")).toBe("control+shift+n");
  });

  it("captures a chord through the input component", async () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: [], key: "" } as MacroChord },
    });
    await wrapper.find("button").trigger("click");
    window.dispatchEvent(keyEvent({ code: "KeyK", key: "k", ctrlKey: true }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update:modelValue")?.[0]?.[0]).toEqual({
      modifiers: ["control"],
      key: "k",
    });
    wrapper.unmount();
  });

  it("cancels capture on Escape without emitting", async () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: ["control"], key: "n" } as MacroChord },
    });
    await wrapper.find("button").trigger("click");
    window.dispatchEvent(keyEvent({ code: "Escape", key: "Escape" }));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update:modelValue")).toBeUndefined();
    expect(wrapper.find(".chord-hint").exists()).toBe(false);
    wrapper.unmount();
  });

  it("clears a recorded shortcut", async () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: ["control"], key: "n" } as MacroChord },
    });
    const clearButton = wrapper.find('[aria-label="Clear shortcut"]');
    expect(clearButton.exists()).toBe(true);
    await clearButton.trigger("click");
    expect(wrapper.emitted("update:modelValue")?.[0]?.[0]).toEqual({ modifiers: [], key: "" });
    wrapper.unmount();
  });

  it("offers no clear action when nothing is recorded", () => {
    const wrapper = mount(ChordInput, {
      props: { modelValue: { modifiers: [], key: "" } as MacroChord },
    });
    expect(wrapper.find('[aria-label="Clear shortcut"]').exists()).toBe(false);
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
