/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { setLanguagePreference } from "../lib/i18n";
import type { AudioDependencyStatus, SettingsDocument } from "../lib/types";

vi.mock("../lib/api", () => ({
  errorMessage: (cause: unknown) => String(cause),
  getAudioDependencyStatus: vi.fn(),
  getRc003HelperInstalled: vi.fn(async () => false),
  getSettings: vi.fn(),
  installRc003Helper: vi.fn(async () => undefined),
  listAudioEndpoints: vi.fn(async () => []),
  openExternalUrl: vi.fn(async () => undefined),
  saveSettings: vi.fn(async () => undefined),
  getVoiceToolCatalog: vi.fn(async () => []),
  // The engine store imports these; onboarding does not call them.
  getEngineSnapshot: vi.fn(),
  getEngineEventCursor: vi.fn(),
  scanRemotes: vi.fn(),
  connectRemote: vi.fn(),
  disconnectRemote: vi.fn(),
}));

import * as api from "../lib/api";
import OnboardingWizard from "./OnboardingWizard.vue";

enableAutoUnmount(afterEach);

const getAudioDependencyStatus = vi.mocked(api.getAudioDependencyStatus);
const getSettings = vi.mocked(api.getSettings);
const openExternalUrl = vi.mocked(api.openExternalUrl);
const saveSettings = vi.mocked(api.saveSettings);

const INSTALL_URL = "https://vb-audio.com/Cable/";

function settingsDocument(): SettingsDocument {
  return { schemaVersion: 1, global: { gain_db: 0 }, devices: {} };
}

function missingStatus(): AudioDependencyStatus {
  return {
    state: "missing",
    cableEndpoints: [],
    suggestedEndpointId: null,
    installUrl: INSTALL_URL,
  };
}

function readyStatus(): AudioDependencyStatus {
  return {
    state: "ready",
    cableEndpoints: [{ id: "cable-in", name: "CABLE Input", recommended: true }],
    suggestedEndpointId: "cable-in",
    installUrl: INSTALL_URL,
  };
}

function mountWizard() {
  return mount(OnboardingWizard, {
    attachTo: document.body,
    global: { plugins: [createPinia()] },
  });
}

function primaryButton(wrapper: ReturnType<typeof mountWizard>) {
  return wrapper.findAll(".wizard-footer button").at(-1)!;
}

async function advanceToStep3(wrapper: ReturnType<typeof mountWizard>) {
  await primaryButton(wrapper).trigger("click"); // 开始配置
  await flushPromises();
  await primaryButton(wrapper).trigger("click"); // 稍后连接
  await flushPromises();
}

beforeEach(() => {
  vi.clearAllMocks();
  setLanguagePreference("zh-Hans");
  getSettings.mockResolvedValue(settingsDocument());
  saveSettings.mockResolvedValue(undefined);
  getAudioDependencyStatus.mockResolvedValue(missingStatus());
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("OnboardingWizard VB-CABLE dependency", () => {
  it("blocks next on step 3 until VB-CABLE is ready, then unlocks and preselects the device", async () => {
    const wrapper = mountWizard();
    await flushPromises();
    await advanceToStep3(wrapper);

    const card = wrapper.find(".audio-dependency");
    expect(card.exists()).toBe(true);
    expect(card.text()).toContain("未检测到 VB-CABLE");
    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    expect(wrapper.find(".wizard-block-reason").text()).toContain("请先安装并检测通过 VB-CABLE");

    // Download opens the official page through the whitelisted external URL command.
    const download = wrapper
      .findAll(".audio-dependency-actions button")
      .find((button) => button.text() === "下载 VB-CABLE");
    expect(download).toBeDefined();
    await download!.trigger("click");
    await flushPromises();
    expect(openExternalUrl).toHaveBeenCalledWith(INSTALL_URL);

    // Re-check picks up the now-ready state and auto-selects the suggested endpoint.
    getAudioDependencyStatus.mockResolvedValue(readyStatus());
    const recheck = wrapper
      .findAll(".audio-dependency-actions button")
      .find((button) => button.text() === "重新检测");
    await recheck!.trigger("click");
    await flushPromises();

    expect(card.text()).toContain("已检测到 VB-CABLE");
    expect(primaryButton(wrapper).attributes("disabled")).toBeUndefined();
    expect(wrapper.find(".wizard-block-reason").exists()).toBe(false);
    const endpointSelect = wrapper.findAll(".voice-options select")[1];
    expect((endpointSelect.element as HTMLSelectElement).value).toBe("cable-in");
  });

  it("shows the disabled state with a re-check action and keeps the block", async () => {
    getAudioDependencyStatus.mockResolvedValue({
      state: "disabled",
      cableEndpoints: [],
      suggestedEndpointId: null,
      installUrl: INSTALL_URL,
    });
    const wrapper = mountWizard();
    await flushPromises();
    await advanceToStep3(wrapper);

    const card = wrapper.find(".audio-dependency");
    expect(card.text()).toContain("已安装 VB-CABLE，但未找到可用的语音输出设备");
    expect(card.text()).toContain("Windows 声音设置");
    expect(
      wrapper
        .findAll(".audio-dependency-actions button")
        .some((button) => button.text() === "重新检测"),
    ).toBe(true);
    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
  });

  it("shows a retry card when the dependency check fails", async () => {
    getAudioDependencyStatus.mockRejectedValueOnce(new Error("command missing"));
    const wrapper = mountWizard();
    await flushPromises();
    await advanceToStep3(wrapper);

    expect(wrapper.find(".audio-dependency").text()).toContain("无法检测 VB-CABLE");
    expect(wrapper.find(".wizard-block-reason").text()).toContain("无法确认 VB-CABLE 状态");
    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();

    getAudioDependencyStatus.mockResolvedValue(missingStatus());
    const recheck = wrapper
      .findAll(".audio-dependency-actions button")
      .find((button) => button.text() === "重新检测");
    await recheck!.trigger("click");
    await flushPromises();
    expect(wrapper.find(".audio-dependency").text()).toContain("未检测到 VB-CABLE");
  });
});
