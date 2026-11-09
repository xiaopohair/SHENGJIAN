/** @vitest-environment jsdom */
import { describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import TitleBar from "./TitleBar.vue";
import { setLanguagePreference } from "../lib/i18n";

setLanguagePreference("zh-Hans");

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({
    isMaximized: vi.fn().mockResolvedValue(false),
    onResized: vi.fn().mockResolvedValue(() => undefined),
    minimize: vi.fn(),
    toggleMaximize: vi.fn(),
    close: vi.fn(),
  }),
}));

describe("TitleBar", () => {
  it("shows the brand mark and product name on the left", () => {
    const wrapper = mount(TitleBar);

    expect(wrapper.find(".titlebar-brand .brand-mark").exists()).toBe(true);
    expect(wrapper.find(".titlebar-name").text()).toBe("声键");
    expect(wrapper.find(".titlebar-brand").attributes("data-tauri-drag-region")).toBeDefined();
  });
});
