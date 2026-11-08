/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import Tabs from "./Tabs.vue";

const items = [
  { id: "appearance", label: "外观" },
  { id: "audio", label: "音频" },
  { id: "advanced", label: "高级" },
];

describe("Tabs", () => {
  it("renders a labelled tablist and only links the active panel", () => {
    const wrapper = mount(Tabs, {
      props: { items, modelValue: "appearance" },
    });
    const tabs = wrapper.findAll('[role="tab"]');

    expect(wrapper.attributes("role")).toBe("tablist");
    expect(tabs).toHaveLength(3);
    expect(tabs[0].attributes("aria-controls")).toBe("panel-appearance");
    expect(tabs[1].attributes("aria-controls")).toBeUndefined();
    expect(tabs[0].attributes("aria-selected")).toBe("true");
  });

  it("emits the next tab on click and keyboard navigation", async () => {
    const wrapper = mount(Tabs, {
      props: { items, modelValue: "appearance" },
    });
    const tabs = wrapper.findAll('[role="tab"]');

    await tabs[1].trigger("click");
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["audio"]);

    await wrapper.setProps({ modelValue: "appearance" });
    await tabs[0].trigger("keydown", { key: "ArrowRight" });
    expect(wrapper.emitted("update:modelValue")?.at(-1)).toEqual(["audio"]);
  });
});
