/** @vitest-environment jsdom */
import { describe, expect, it } from "vitest";
import { mount } from "@vue/test-utils";
import TopbarSearch from "./TopbarSearch.vue";
import type { PaletteCommand } from "../lib/palette";

const commands: PaletteCommand[] = [
  { id: "nav:alpha", label: "Alpha", group: "导航", keywords: "alpha" },
  { id: "nav:beta", label: "Beta", group: "导航", keywords: "beta" },
  { id: "nav:gamma", label: "Gamma", group: "导航", keywords: "gamma" },
];

describe("TopbarSearch", () => {
  it("expands from the icon, filters, and runs the selected command", async () => {
    const wrapper = mount(TopbarSearch, {
      props: { commands },
      attachTo: document.body,
    });

    await wrapper.find(".topbar-search-toggle").trigger("click");
    const input = wrapper.find(".topbar-search-field input");
    expect(input.exists()).toBe(true);

    await input.setValue("gamma");
    expect(wrapper.findAll(".topbar-search-option")).toHaveLength(1);

    await input.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("run")?.at(-1)).toEqual([commands[2]]);
    expect(wrapper.find(".topbar-search-field").exists()).toBe(false);

    wrapper.unmount();
  });

  it("resets the active index when the query changes", async () => {
    const wrapper = mount(TopbarSearch, {
      props: { commands },
      attachTo: document.body,
    });

    await wrapper.find(".topbar-search-toggle").trigger("click");
    const input = wrapper.find(".topbar-search-field input");
    await input.trigger("keydown", { key: "ArrowDown" });
    await input.setValue("gamma");
    expect(wrapper.find(".topbar-search-option.active").text()).toBe("Gamma");

    await input.trigger("keydown", { key: "Enter" });
    expect(wrapper.emitted("run")?.at(-1)).toEqual([commands[2]]);
    wrapper.unmount();
  });
});
