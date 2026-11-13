/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";
import type { ProfileDraft } from "../lib/buttons";
import { resetVoiceToolCatalog } from "../composables/useVoiceToolCatalog";
import type { VoiceToolInfo } from "../lib/types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
}));

vi.mock("../lib/api", () => ({
  getVoiceToolCatalog: vi.fn(),
  listLaunchableApps: vi.fn(async () => ({ apps: [], scanned: true, scanError: null })),
  describeAppTargets: vi.fn(async () => []),
  canonicalizeAppTarget: vi.fn(),
  appIconDataUrls: vi.fn(async (targets: string[]) => targets.map(() => null)),
  errorMessage: (cause: unknown) => String(cause),
}));

import * as api from "../lib/api";
import ProfileSettingsDrawer from "./ProfileSettingsDrawer.vue";

enableAutoUnmount(afterEach);

const getVoiceToolCatalog = vi.mocked(api.getVoiceToolCatalog);

const CATALOG: VoiceToolInfo[] = [
  { id: "wechat", label: "微信输入法", defaultHotkey: ["ControlLeft", "MetaLeft"] },
  { id: "doubao", label: "豆包输入法", defaultHotkey: ["AltRight"] },
  { id: "vokie", label: "Vokie 桌面版", defaultHotkey: [] },
];

function draftWith(voiceInputTool: string): ProfileDraft {
  return {
    id: "profile-test",
    name: "测试方案",
    appTargets: "",
    autoApply: false,
    holdRepeat: "",
    voiceInputTool,
    bindings: {},
  };
}

function mountDrawer(voiceInputTool = "") {
  return mount(ProfileSettingsDrawer, {
    attachTo: document.body,
    props: {
      open: true,
      busy: false,
      foregroundApp: "",
      draft: draftWith(voiceInputTool),
    },
  });
}

function voiceSelect(): HTMLSelectElement {
  const select = document.querySelector<HTMLSelectElement>(".settings-body select");
  if (!select) throw new Error("voice tool select not found");
  return select;
}

function options(): Array<{ value: string; label: string }> {
  return Array.from(voiceSelect().options).map((option) => ({
    value: option.value,
    label: option.textContent?.trim() ?? "",
  }));
}

beforeEach(() => {
  vi.clearAllMocks();
  resetVoiceToolCatalog();
  setLanguagePreference("zh-Hans");
  getVoiceToolCatalog.mockResolvedValue(CATALOG);
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("ProfileSettingsDrawer voice tool catalog", () => {
  it("renders the configured tool labels after the inherit empty option", async () => {
    mountDrawer();
    await flushPromises();

    expect(getVoiceToolCatalog).toHaveBeenCalledTimes(1);
    expect(options()).toEqual([
      { value: "", label: "使用设备覆盖设置或全局设置" },
      { value: "wechat", label: "微信输入法" },
      { value: "doubao", label: "豆包输入法" },
      { value: "vokie", label: "Vokie 桌面版" },
    ]);
  });

  it("does not translate configured app labels in the English UI", async () => {
    setLanguagePreference("en");
    mountDrawer();
    await flushPromises();

    const labels = options().map((option) => option.label);
    expect(labels).toContain("微信输入法");
    expect(labels).not.toContain("WeChat IME");
  });

  it("keeps a saved id that is missing from the catalog as a temporary option", async () => {
    mountDrawer("legacy-tool");
    await flushPromises();

    const legacy = options().find((option) => option.value === "legacy-tool");
    expect(legacy).toEqual({ value: "legacy-tool", label: "legacy-tool" });
    expect(voiceSelect().value).toBe("legacy-tool");
  });

  it("keeps the saved value selectable when the catalog command fails", async () => {
    getVoiceToolCatalog.mockRejectedValue(new Error("command missing"));
    mountDrawer("wechat");
    await flushPromises();

    expect(options()).toEqual([
      { value: "", label: "使用设备覆盖设置或全局设置" },
      { value: "wechat", label: "wechat" },
    ]);
    expect(voiceSelect().value).toBe("wechat");
  });
});
