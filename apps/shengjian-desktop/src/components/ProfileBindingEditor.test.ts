/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";
import type { ProfileDraft } from "../lib/buttons";
import ProfileBindingEditor from "./ProfileBindingEditor.vue";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
}));

vi.mock("../lib/api", () => ({
  listLaunchableApps: vi.fn(async () => ({ apps: [], scanned: true, scanError: null })),
  describeAppTargets: vi.fn(async () => []),
  canonicalizeAppTarget: vi.fn(),
  appIconDataUrls: vi.fn(async () => []),
  resolveAppTarget: vi.fn(async () => null),
  errorMessage: (cause: unknown) => String(cause),
}));

enableAutoUnmount(afterEach);
setLanguagePreference("zh-Hans");

function draftWith(bindings: ProfileDraft["bindings"]): ProfileDraft {
  return {
    id: "profile-test",
    name: "测试",
    appTargets: "",
    autoApply: false,
    holdRepeat: "",
    voiceInputTool: "",
    bindings,
  };
}

function mountEditor(bindings: ProfileDraft["bindings"]) {
  return mount(ProfileBindingEditor, {
    props: {
      buttonId: "ok",
      trigger: "single",
      busy: false,
      commonPhrasesAvailable: false,
      latestMacros: [],
      draft: draftWith(bindings),
    },
  });
}

describe("ProfileBindingEditor action parameters", () => {
  it("renders the macro-style divider, heading and issue row for open app", async () => {
    const wrapper = mountEditor({ ok: { single: { type: "open-app", app: "" } } });
    await flushPromises();

    const config = wrapper.find(".action-config");
    expect(config.exists()).toBe(true);
    expect(config.find(".config-head").text()).toBe("动作参数");
    expect(config.find(".config-field label").text()).toBe("应用");
    expect(config.find(".config-issue").text()).toContain("还没有选择应用");
    expect(config.find(".config-note").exists()).toBe(false);
    expect(config.find(".app-target-field-input").attributes("readonly")).toBeDefined();
    wrapper.unmount();
  });

  it("shows the check-mark note once an app target is configured", async () => {
    const wrapper = mountEditor({ ok: { single: { type: "open-app", app: "wechat" } } });
    await flushPromises();

    expect(wrapper.find(".config-issue").exists()).toBe(false);
    expect(wrapper.find(".config-note").text()).toContain("点「选择应用」更换应用");
    wrapper.unmount();
  });

  it("keeps a note-only config block for parameterless actions", async () => {
    const wrapper = mountEditor({ ok: { single: { type: "disabled" } } });
    await flushPromises();

    expect(wrapper.find(".action-config").exists()).toBe(true);
    expect(wrapper.find(".config-head").text()).toBe("动作参数");
    expect(wrapper.find(".config-note").text()).toContain("该手势不会触发任何动作");
    wrapper.unmount();
  });

  it("adds a system action card whose grid saves the picked key", async () => {
    const draft = draftWith({});
    const wrapper = mount(ProfileBindingEditor, {
      props: {
        buttonId: "ok",
        trigger: "single",
        busy: false,
        commonPhrasesAvailable: false,
        latestMacros: [],
        draft,
      },
    });
    await flushPromises();

    const card = wrapper
      .findAll(".binding-option")
      .find((button) => button.text().includes("系统动作"));
    expect(card).toBeDefined();
    await card!.trigger("click");

    // Switching to the action starts without a key and shows the missing-key issue.
    expect(draft.bindings.ok?.single).toEqual({ type: "system-action", key: "" });
    expect(wrapper.find(".system-action-grid").exists()).toBe(true);
    expect(wrapper.find(".config-issue").text()).toContain("请选择系统动作");
    expect(wrapper.find(".hold-repeat-section").exists()).toBe(false);

    const radios = wrapper.findAll('.system-action-grid [role="radio"]');
    expect(radios).toHaveLength(9);
    await radios[2].trigger("click");

    expect(draft.bindings.ok?.single).toEqual({ type: "system-action", key: "volume_mute" });
    expect(wrapper.find(".config-issue").exists()).toBe(false);
    expect(wrapper.find(".config-note").text()).toContain("系统媒体键");
    // A single system key repeats like a shortcut, matching the backend rule.
    expect(wrapper.find(".hold-repeat-section").exists()).toBe(true);

    // 返回桌面是组合类系统动作：存 id，且不出现按住连发。
    const desktop = radios.find((radio) => radio.text().includes("返回桌面"));
    expect(desktop).toBeDefined();
    await desktop!.trigger("click");
    expect(draft.bindings.ok?.single).toEqual({ type: "system-action", key: "show_desktop" });
    expect(wrapper.find(".hold-repeat-section").exists()).toBe(false);
    wrapper.unmount();
  });

  it("restores the selected radio for an existing system action binding", async () => {
    const wrapper = mountEditor({
      ok: { single: { type: "system-action", key: "media_next" } },
    });
    await flushPromises();

    const checked = wrapper
      .findAll('.system-action-grid [role="radio"]')
      .filter((radio) => radio.attributes("aria-checked") === "true");
    expect(checked).toHaveLength(1);
    expect(checked[0].text()).toContain("下一曲");
    expect(wrapper.find(".config-issue").exists()).toBe(false);
    wrapper.unmount();
  });
});
