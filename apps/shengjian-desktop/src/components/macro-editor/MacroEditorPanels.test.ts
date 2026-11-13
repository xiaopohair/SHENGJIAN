/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { setLanguagePreference } from "../../lib/i18n";
import { mount } from "@vue/test-utils";
import MacroActionPanel from "./MacroActionPanel.vue";
import MacroEditorHeader from "./MacroEditorHeader.vue";
import MacroSettingsDrawer from "./MacroSettingsDrawer.vue";
import MacroStepList from "./MacroStepList.vue";
import type { MacroStep } from "../../lib/types";

function step(overrides: Partial<MacroStep> = {}): MacroStep {
  return {
    stepId: "step-1",
    action: "open_application",
    parameters: { target: "notepad.exe" },
    ...overrides,
  };
}

const rows = [
  { step: step(), index: 0, issue: "", summary: "notepad.exe" },
  {
    step: step({
      stepId: "step-2",
      action: "send_shortcut",
      parameters: { chord: { modifiers: ["control"], key: "n" } },
    }),
    index: 1,
    issue: "",
    summary: "Ctrl + N",
  },
];

setLanguagePreference("zh-Hans");

describe("Macro editor panels", () => {
  it("exposes the selected step and action via aria attributes", async () => {
    const wrapper = mount(MacroActionPanel, {
      props: {
        step: step(),
        issue: "",
        index: 0,
        total: 1,
        otherMacros: [],
      },
    });

    const actions = wrapper.findAll(".action-option");
    expect(actions[1].attributes("aria-pressed")).toBe("true");
    await actions[2].trigger("click");
    expect(wrapper.emitted("update-step")?.[0]?.[0]).toMatchObject({
      action: "open_url",
    });
    wrapper.unmount();
  });

  it("selects a step from the keyboard and reorders via the list", async () => {
    const wrapper = mount(MacroStepList, {
      props: {
        rows,
        selectedStepId: rows[0].step.stepId,
        runSteps: [],
      },
    });

    const stepRows = wrapper.findAll(".step-row");
    expect(stepRows[0].attributes("role")).toBe("button");
    expect(stepRows[0].attributes("aria-pressed")).toBe("true");
    await stepRows[1].trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("select")?.[0]).toEqual([1]);
    await wrapper.findAll(".step-insert button")[0].trigger("click");
    expect(wrapper.emitted("insert")?.[0]).toEqual([1]);
    wrapper.unmount();
  });

  it("shows the shortcut as plain text, not keycaps", () => {
    const wrapper = mount(MacroStepList, {
      props: { rows, selectedStepId: rows[0].step.stepId, runSteps: [] },
    });

    expect(wrapper.findAll(".step-row")[1].text()).toContain("Ctrl + N");
    expect(wrapper.findAll(".step-copy kbd")).toHaveLength(0);
    wrapper.unmount();
  });

  it("reorders steps by dragging the handle", async () => {
    // jsdom reports zero-size rects, so give each row a vertical band.
    const originalRect = HTMLElement.prototype.getBoundingClientRect;
    HTMLElement.prototype.getBoundingClientRect = function (this: HTMLElement) {
      const index = Number(this.dataset.index);
      const top = Number.isNaN(index) ? 0 : index * 60;
      return {
        top,
        bottom: top + 48,
        height: 48,
        left: 0,
        right: 200,
        width: 200,
        x: 0,
        y: top,
      } as DOMRect;
    };

    const wrapper = mount(MacroStepList, {
      props: { rows, selectedStepId: rows[0].step.stepId, runSteps: [] },
      attachTo: document.body,
    });

    try {
      const stepRows = wrapper.findAll(".step-row");
      stepRows[0]
        .find(".drag-handle")
        .element.dispatchEvent(new PointerEvent("pointerdown", { button: 0, bubbles: true }));
      window.dispatchEvent(new PointerEvent("pointermove", { clientY: 84, button: 0 }));
      window.dispatchEvent(new PointerEvent("pointerup", { button: 0 }));
      await wrapper.vm.$nextTick();
      expect(wrapper.emitted("reorder")?.[0]).toEqual([0, 1]);
    } finally {
      HTMLElement.prototype.getBoundingClientRect = originalRect;
      wrapper.unmount();
    }
  });

  it("keeps move and add controls in the step header toolbar", async () => {
    const wrapper = mount(MacroStepList, {
      props: { rows, selectedStepId: rows[0].step.stepId, runSteps: [] },
    });

    const toolbar = wrapper.find(".step-toolbar");
    expect(toolbar.exists()).toBe(true);
    const buttons = toolbar.findAll("button");
    expect(buttons).toHaveLength(3);

    // First row: up is disabled, down is available.
    expect(buttons[0].attributes("disabled")).toBeDefined();
    await buttons[1].trigger("click");
    expect(wrapper.emitted("move")?.[0]).toEqual([1]);
    await buttons[2].trigger("click");
    expect(wrapper.emitted("add")).toHaveLength(1);
    wrapper.unmount();
  });

  it("colours the action icon by action family", () => {
    const wrapper = mount(MacroStepList, {
      props: { rows, selectedStepId: null, runSteps: [] },
    });

    const tiles = wrapper.findAll(".action-tile");
    expect(tiles[0].classes()).toContain("tone-green");
    expect(tiles[1].classes()).toContain("tone-violet");
    wrapper.unmount();
  });

  it("offers a system-action card grid instead of chord capture", async () => {
    const wrapper = mount(MacroActionPanel, {
      props: {
        step: step({
          action: "system_action",
          parameters: { chord: { modifiers: [], key: "volume_mute" } },
        }),
        issue: "",
        index: 0,
        total: 1,
        otherMacros: [],
      },
    });

    expect(wrapper.find(".chord-input").exists()).toBe(false);
    const grid = wrapper.find(".system-action-grid");
    expect(grid.exists()).toBe(true);
    expect(grid.attributes("role")).toBe("radiogroup");

    const radios = grid.findAll('[role="radio"]');
    expect(radios).toHaveLength(9);
    expect(radios.map((radio) => radio.text())).toContain("音量 +");
    // The saved key keeps its radio selected.
    expect(radios[2].attributes("aria-checked")).toBe("true");

    await radios[0].trigger("click");
    expect(wrapper.emitted("update-step")?.[0]?.[0]).toMatchObject({
      parameters: { chord: { modifiers: [], key: "volume_up" } },
    });
    // 返回桌面是组合键：宏里存真实 chord，而不是伪 key。
    const desktop = radios.find((radio) => radio.text().includes("返回桌面"));
    expect(desktop).toBeDefined();
    await desktop!.trigger("click");
    expect(wrapper.emitted("update-step")?.[1]?.[0]).toMatchObject({
      parameters: { chord: { modifiers: ["win"], key: "d" } },
    });
    wrapper.unmount();
  });

  it("shows each step's run outcome and duration inside the step row", () => {
    const wrapper = mount(MacroStepList, {
      props: {
        rows,
        selectedStepId: rows[0].step.stepId,
        runSteps: [
          {
            index: 0,
            stepId: "step-1",
            action: "open_application",
            outcome: "done",
            elapsedMs: 126,
          },
          {
            index: 1,
            stepId: "step-2",
            action: "send_shortcut",
            outcome: "timed_out",
            elapsedMs: 2000,
          },
        ],
      },
    });

    const results = wrapper.findAll(".run-result");
    expect(results).toHaveLength(2);
    expect(results[0].text()).toContain("126ms");
    expect(results[1].text()).toContain("2000ms");
    expect(results[1].classes()).toContain("danger");
    wrapper.unmount();
  });
  it("labels the title with 编辑宏 and hides the inline description", () => {
    const wrapper = mount(MacroEditorHeader, {
      props: {
        name: "测试宏",
        summary: "说明文字",
        editingNew: false,
        busy: false,
        dirty: false,
        canTest: true,
      },
    });

    expect(wrapper.find(".eyebrow").text()).toBe("编辑宏：");
    // The summary now lives in the properties drawer, not inline.
    expect(wrapper.find(".desc-input").exists()).toBe(false);
    expect(wrapper.text()).not.toContain("说明文字");
    // Editing an existing macro: the title is display-only (rename via 属性).
    expect(wrapper.find(".name-input").exists()).toBe(false);
    expect(wrapper.find(".name-display").text()).toBe("测试宏");
    wrapper.unmount();
  });

  it("opens the properties drawer and edits name and summary", async () => {
    const name = "旧名称";
    const summary = "旧说明";
    const wrapper = mount(MacroSettingsDrawer, {
      attachTo: document.body,
      props: {
        open: true,
        name,
        summary,
        "onUpdate:name": (value: string) => wrapper.setProps({ name: value }),
        "onUpdate:summary": (value: string) => wrapper.setProps({ summary: value }),
      },
    });

    expect(document.querySelector(".macro-settings-head h2")?.textContent?.trim()).toBe("宏属性");
    const labels = [...document.querySelectorAll(".macro-field-label")].map((node) =>
      node.textContent?.trim(),
    );
    expect(labels).toContain("名称");
    expect(labels).toContain("说明");
    // Stacked layout: each field has its label first, then the input.
    for (const field of document.querySelectorAll<HTMLElement>(".macro-field")) {
      const label = field.querySelector(".macro-field-label");
      const input = field.querySelector("input");
      expect(label).not.toBeNull();
      expect(input).not.toBeNull();
      expect(
        label!.compareDocumentPosition(input!) & Node.DOCUMENT_POSITION_FOLLOWING,
      ).toBeTruthy();
    }

    const inputs = [...document.querySelectorAll<HTMLInputElement>(".macro-settings-body input")];
    expect(inputs).toHaveLength(2);
    inputs[1].value = "新说明";
    inputs[1].dispatchEvent(new Event("input"));
    await wrapper.vm.$nextTick();
    expect(wrapper.emitted("update:summary")?.[0]).toEqual(["新说明"]);
    wrapper.unmount();
  });
});
