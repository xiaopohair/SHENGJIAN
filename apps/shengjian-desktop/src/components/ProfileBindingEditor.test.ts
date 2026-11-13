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
    wrapper.unmount();
  });

  it("shows the check-mark note once an app target is configured", async () => {
    const wrapper = mountEditor({ ok: { single: { type: "open-app", app: "wechat" } } });
    await flushPromises();

    expect(wrapper.find(".config-issue").exists()).toBe(false);
    expect(wrapper.find(".config-note").text()).toContain("点「选择应用」可更换");
    wrapper.unmount();
  });

  it("keeps a note-only config block for parameterless actions", async () => {
    const wrapper = mountEditor({ ok: { single: { type: "disabled" } } });
    await flushPromises();

    expect(wrapper.find(".action-config").exists()).toBe(true);
    expect(wrapper.find(".config-head").exists()).toBe(false);
    expect(wrapper.find(".config-note").text()).toContain("该手势不会触发任何动作");
    wrapper.unmount();
  });
});
