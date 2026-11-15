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
import { useEngineStore } from "../stores/engine";
import { PROFILE_BUTTONS } from "../lib/buttons";
import type { EngineSnapshot } from "../lib/types";
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

function setConnected(connected: boolean) {
  const store = useEngineStore();
  if (!connected) {
    store.snapshot = null;
    return;
  }
  const snapshot = {
    connection: {
      phase: "ready",
      remote: {
        remoteId: "remote-1",
        name: "遥控器",
        model: "RC003",
        deviceId: "xiaomi-rc003",
        token: "remote-1",
        isSupported: true,
      },
      generation: 1,
      lastError: null,
    },
    voice: { state: "idle" },
    audio: {
      state: "unconfigured",
      endpoint: null,
      format: null,
      gainDb: 0,
      generation: 0,
      lastError: null,
    },
    capabilities: {},
  } as unknown as EngineSnapshot;
  store.snapshot = snapshot;
}

function setVoiceTestPassed() {
  const store = useEngineStore();
  store.voiceHistory = [{ seq: 1, state: "streaming" }];
}

async function advanceToStep3(wrapper: ReturnType<typeof mountWizard>) {
  await primaryButton(wrapper).trigger("click"); // 开始配置
  await flushPromises();
  setConnected(true);
  await flushPromises();
  await primaryButton(wrapper).trigger("click"); // 下一步（第 2 步要求已连接）
  await flushPromises();
}

describe("OnboardingWizard step test gates", () => {
  it("requires the voice test and all button checks before advancing", async () => {
    getAudioDependencyStatus.mockResolvedValue(readyStatus());
    const wrapper = mountWizard();
    await flushPromises();

    // step 1 -> step 2 (connected) -> step 3
    setConnected(true);
    await flushPromises();
    await primaryButton(wrapper).trigger("click");
    await flushPromises();
    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    expect(wrapper.find(".wizard-block-reason").text()).toContain("请先完成语音测试");

    setVoiceTestPassed();
    await flushPromises();
    expect(primaryButton(wrapper).attributes("disabled")).toBeUndefined();
    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    // step 4: 必须检测到全部按键；文案使用“全按键支持服务”
    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    expect(wrapper.text()).toContain("启用全按键支持服务");
    expect(wrapper.text()).toContain(
      "返回键和音量键需要安装全按键支持服务并开启才能使用；首次安装会请求管理员授权。",
    );
    expect(wrapper.find(".wizard-block-reason").text()).toContain("请先检测全部按键");
    const store = useEngineStore();
    store.buttonHistory = PROFILE_BUTTONS.map((button, index) => ({
      seq: index + 1,
      type: "gesture" as const,
      button: button.id,
    }));
    await flushPromises();
    expect(primaryButton(wrapper).attributes("disabled")).toBeUndefined();

    // 「重新测试」清空已检测状态，下一步重新被阻断。
    const reset = wrapper
      .findAll(".wizard-footer button")
      .find((button) => button.text() === "重新测试");
    expect(reset).toBeDefined();
    await reset!.trigger("click");
    await flushPromises();
    expect(wrapper.text()).toContain("已检测 0/");
    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    expect(wrapper.find(".wizard-block-reason").text()).toContain("请先检测全部按键");
  });
});

describe("OnboardingWizard remote connection gate", () => {
  it("blocks step 2 until a remote is connected", async () => {
    const wrapper = mountWizard();
    await flushPromises();
    await primaryButton(wrapper).trigger("click"); // 开始配置
    await flushPromises();

    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    expect(wrapper.find(".wizard-block-reason").text()).toContain("请先连接遥控器");

    setConnected(true);
    await flushPromises();
    expect(primaryButton(wrapper).attributes("disabled")).toBeUndefined();
    expect(wrapper.find(".wizard-block-reason").exists()).toBe(false);
  });
});

describe("OnboardingWizard settings persistence", () => {
  function configuredDocument(): SettingsDocument {
    return {
      schemaVersion: 1,
      global: {
        gain_db: 12,
        voice_input_tool: "wechat",
        audio_endpoint_id: "cable-in",
        rc003_capture_enabled: true,
      },
      devices: {},
    } as unknown as SettingsDocument;
  }

  async function advanceToStep3(wrapper: ReturnType<typeof mountWizard>) {
    await primaryButton(wrapper).trigger("click"); // 开始配置 → 第 2 步
    await flushPromises();
    setConnected(true);
    await flushPromises();
    await primaryButton(wrapper).trigger("click"); // 第 2 步 → 第 3 步
    await flushPromises();
  }

  async function advanceToFinish(wrapper: ReturnType<typeof mountWizard>) {
    setVoiceTestPassed();
    await flushPromises();
    await primaryButton(wrapper).trigger("click"); // 第 3 步 → 第 4 步
    await flushPromises();
    const store = useEngineStore();
    store.buttonHistory = PROFILE_BUTTONS.map((button, index) => ({
      seq: index + 1,
      type: "gesture" as const,
      button: button.id,
    }));
    await flushPromises();
    await primaryButton(wrapper).trigger("click"); // 第 4 步 → 第 5 步
    await flushPromises();
    expect(primaryButton(wrapper).text()).toBe("进入应用");
  }

  it("does not write settings while the user is only browsing or changing wizard fields", async () => {
    getAudioDependencyStatus.mockResolvedValue(readyStatus());
    getSettings.mockResolvedValue(configuredDocument());
    const wrapper = mountWizard();
    await flushPromises();
    await advanceToStep3(wrapper);

    // 以前这里改下拉会立刻 saveSettings，重跑一次向导就会覆盖系统配置。
    await wrapper.findAll(".voice-options select")[1].setValue("");
    await flushPromises();
    expect(saveSettings).not.toHaveBeenCalled();
  });

  it("writes nothing when finishing without changes", async () => {
    getAudioDependencyStatus.mockResolvedValue(readyStatus());
    getSettings.mockResolvedValue(configuredDocument());
    const wrapper = mountWizard();
    await flushPromises();
    await advanceToStep3(wrapper);
    await advanceToFinish(wrapper);

    await primaryButton(wrapper).trigger("click"); // 进入应用
    await flushPromises();
    expect(saveSettings).not.toHaveBeenCalled();
  });

  it("writes only the field the wizard actually changed", async () => {
    getAudioDependencyStatus.mockResolvedValue(readyStatus());
    getSettings.mockResolvedValue(configuredDocument());
    const wrapper = mountWizard();
    await flushPromises();
    await advanceToStep3(wrapper);
    await wrapper.findAll(".voice-options select")[1].setValue("");
    await flushPromises();
    await advanceToFinish(wrapper);

    await primaryButton(wrapper).trigger("click"); // 进入应用
    await flushPromises();
    expect(saveSettings).toHaveBeenCalledTimes(1);
    const saved = saveSettings.mock.calls[0][0] as SettingsDocument;
    expect(saved.global.voice_input_tool).toBeNull();
    // 其余字段原样保留。
    expect(saved.global.audio_endpoint_id).toBe("cable-in");
    expect(saved.global.rc003_capture_enabled).toBe(true);
    expect(saved.global.gain_db).toBe(12);
  });
});

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
      .findAll(".audio-endpoint-actions button")
      .find((button) => button.text() === "下载 VB-CABLE");
    expect(download).toBeDefined();
    await download!.trigger("click");
    await flushPromises();
    expect(openExternalUrl).toHaveBeenCalledWith(INSTALL_URL);

    // Re-check picks up the now-ready state and auto-selects the suggested endpoint.
    getAudioDependencyStatus.mockResolvedValue(readyStatus());
    const recheck = wrapper
      .findAll(".audio-endpoint-actions button")
      .find((button) => button.text() === "重新检测");
    await recheck!.trigger("click");
    await flushPromises();

    // Detected: no success prompt at all (the card is hidden).
    expect(wrapper.find(".audio-dependency").exists()).toBe(false);
    // 依赖通过后还必须完成语音测试，才允许进入下一步。
    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    setVoiceTestPassed();
    await flushPromises();
    expect(primaryButton(wrapper).attributes("disabled")).toBeUndefined();
    expect(wrapper.find(".wizard-block-reason").exists()).toBe(false);
    // 「语音输出设备」现在是这一组里的第一个下拉（2026-11-15 调换顺序）。
    const endpointSelect = wrapper.findAll(".voice-options select")[0];
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
        .findAll(".audio-endpoint-actions button")
        .some((button) => button.text() === "重新检测"),
    ).toBe(true);
    // 已安装但设备不可用时也给出下载/重装入口。
    expect(
      wrapper
        .findAll(".audio-endpoint-actions button")
        .some((button) => button.text() === "下载 VB-CABLE"),
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
      .findAll(".audio-endpoint-actions button")
      .find((button) => button.text() === "重新检测");
    await recheck!.trigger("click");
    await flushPromises();
    expect(wrapper.find(".audio-dependency").text()).toContain("未检测到 VB-CABLE");
  });
});
