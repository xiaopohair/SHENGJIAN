/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";

import type { ButtonProfile, RecommendedButtonProfilesBundle } from "../lib/types";

vi.mock("../lib/api", () => ({
  getRecommendedButtonProfiles: vi.fn(),
  errorMessage: (cause: unknown) => String(cause),
}));

import * as api from "../lib/api";
import RecommendedProfilesDialog from "./RecommendedProfilesDialog.vue";

enableAutoUnmount(afterEach);

const getRecommendedButtonProfiles = vi.mocked(api.getRecommendedButtonProfiles);

function profile(id: string, name: string): ButtonProfile {
  return { id, name, appTargets: [], autoApply: false, bindings: {} };
}

function bundle(): RecommendedButtonProfilesBundle {
  return {
    bundleVersion: "2026.11.1",
    profiles: [
      {
        id: "preset.desktop.shortcuts",
        name: "桌面快捷键",
        deviceModel: null,
        compatibleModels: [],
        appTargets: [],
        autoApply: false,
        bindCount: 2,
        profile: profile("preset.desktop.shortcuts", "桌面快捷键"),
      },
      {
        id: "preset.netease.controls",
        name: "网易云音乐按键",
        deviceModel: null,
        compatibleModels: ["RC003"],
        appTargets: ["netease_music"],
        autoApply: true,
        bindCount: 3,
        profile: profile("preset.netease.controls", "网易云音乐按键"),
      },
    ],
  };
}

function mountDialog(model: string | null = "ARN9") {
  return mount(RecommendedProfilesDialog, {
    attachTo: document.body,
    props: { open: true, model },
  });
}

function query<T extends HTMLElement>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`missing element: ${selector}`);
  return element;
}

function queryAll<T extends HTMLElement>(selector: string): T[] {
  return Array.from(document.querySelectorAll<T>(selector));
}

beforeEach(() => {
  vi.clearAllMocks();
  setLanguagePreference("zh-Hans");
  window.localStorage.clear();
  getRecommendedButtonProfiles.mockResolvedValue(bundle());
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("RecommendedProfilesDialog", () => {
  it("lists the recommendations with model fit, counts and the bundle version", async () => {
    const wrapper = mountDialog();
    await flushPromises();

    expect(getRecommendedButtonProfiles).toHaveBeenCalledWith("ARN9");
    const text = query(".recommended-dialog").textContent ?? "";
    expect(text).toContain("桌面快捷键");
    expect(text).toContain("网易云音乐按键");
    expect(text).toContain("通用");
    expect(text).toContain("RC003");
    expect(text).toContain("0 个应用目标");
    expect(text).toContain("1 个应用目标");
    expect(text).toContain("2 个绑定");
    expect(text).toContain("3 个绑定");
    expect(text).toContain("自动应用");
    expect(text).toContain("2026.11.1");
    // 配置版本在页脚（不再放标题下方）。
    expect(query(".recommended-foot-version").textContent).toContain("2026.11.1");
    expect(wrapper.emitted("loaded")?.[0]).toEqual(["2026.11.1"]);
  });

  it("emits the picked profile when copy & use is clicked", async () => {
    const wrapper = mountDialog();
    await flushPromises();

    query<HTMLButtonElement>(".recommended-use").click();
    await flushPromises();

    const picked = wrapper.emitted("use")?.[0]?.[0] as { id: string } | undefined;
    expect(picked?.id).toBe("preset.desktop.shortcuts");
  });

  it("shows an empty state when the backend returns no profiles", async () => {
    getRecommendedButtonProfiles.mockResolvedValue({ bundleVersion: "2026.11.1", profiles: [] });
    mountDialog();
    await flushPromises();

    expect(document.body.textContent).toContain("当前型号暂无推荐方案");
  });

  it("shows a retry action after a load failure and recovers", async () => {
    getRecommendedButtonProfiles
      .mockRejectedValueOnce(new Error("bundle failed"))
      .mockResolvedValueOnce(bundle());
    mountDialog();
    await flushPromises();

    expect(document.body.textContent).toContain("无法加载推荐方案");
    const retry = queryAll<HTMLButtonElement>(".recommended-status button").find(
      (button) => button.textContent?.trim() === "重试",
    );
    expect(retry).toBeDefined();
    retry!.click();
    await flushPromises();

    expect(getRecommendedButtonProfiles).toHaveBeenCalledTimes(2);
    expect(document.body.textContent).toContain("桌面快捷键");
  });

  it("closes on Escape", async () => {
    const wrapper = mountDialog();
    await flushPromises();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await flushPromises();

    expect(wrapper.emitted("close")).toHaveLength(1);
  });
});
