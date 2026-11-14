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
  it("leaves the brand to the sidebar and keeps only window controls", () => {
    const wrapper = mount(TitleBar);

    expect(wrapper.find(".titlebar-brand").exists()).toBe(false);
    expect(wrapper.find(".titlebar-name").exists()).toBe(false);
    expect(wrapper.find(".titlebar-drag").attributes("data-tauri-drag-region")).toBeDefined();
  });
});
