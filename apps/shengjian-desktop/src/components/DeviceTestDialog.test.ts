/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import DeviceTestDialog from "./DeviceTestDialog.vue";
import * as api from "../lib/api";
import { PROFILE_BUTTONS } from "../lib/buttons";
import { setLanguagePreference } from "../lib/i18n";
import { useEngineStore } from "../stores/engine";

vi.mock("@tauri-apps/api/window", () => ({
  getCurrentWindow: () => ({ minimize: vi.fn(async () => undefined) }),
}));

vi.mock("../lib/api", () => ({
  errorMessage: (cause: unknown) => String(cause),
  getEngineSnapshot: vi.fn(),
  getEngineEventCursor: vi.fn(),
  scanRemotes: vi.fn(),
  connectRemote: vi.fn(),
  disconnectRemote: vi.fn(),
  getDeviceStatus: vi.fn(),
  openVoiceTestPad: vi.fn(async () => undefined),
  raiseVoiceTestPad: vi.fn(async () => undefined),
  voiceTestPadForeground: vi.fn(async () => true),
  collapseVoiceTestPad: vi.fn(async () => undefined),
  expandVoiceTestPad: vi.fn(async () => undefined),
  closeVoiceTestPad: vi.fn(async () => undefined),
  voiceTestPadText: vi.fn(async () => null),
  beginTranscriptProbe: vi.fn(async () => undefined),
  endTranscriptProbe: vi.fn(async () => undefined),
  takeTranscriptProbe: vi.fn(async () => null),
}));

enableAutoUnmount(afterEach);

function mountDialog(tab: "voice" | "buttons" = "voice") {
  const pinia = createPinia();
  setActivePinia(pinia);
  return mount(DeviceTestDialog, {
    props: { initialTab: tab },
    global: { plugins: [pinia], stubs: { Teleport: true } },
    attachTo: document.body,
  });
}

beforeEach(() => {
  setLanguagePreference("zh-Hans");
  vi.mocked(api.takeTranscriptProbe).mockResolvedValue(null);
});

describe("DeviceTestDialog", () => {
  it("marks the voice path usable once a new streaming event arrives", async () => {
    const wrapper = mountDialog("voice");
    const store = useEngineStore();
    expect(wrapper.text()).toContain("等待语音输入");
    expect(wrapper.text()).not.toContain("语音链路正常");

    store.voiceHistory = [{ seq: 1, state: "streaming" }] as never;
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("语音链路正常");
  });

  it("clears the voice result when re-testing", async () => {
    const wrapper = mountDialog("voice");
    const store = useEngineStore();
    store.voiceHistory = [{ seq: 1, state: "streaming" }] as never;
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("语音链路正常");

    await wrapper.findAll("footer button")[0].trigger("click");
    expect(wrapper.text()).not.toContain("语音链路正常");
    expect(wrapper.text()).toContain("等待语音输入");
  });

  it("counts detected buttons and resets the count", async () => {
    const wrapper = mountDialog("buttons");
    const store = useEngineStore();
    expect(wrapper.text()).toContain(`已检测 0/${PROFILE_BUTTONS.length}`);

    store.buttonHistory = PROFILE_BUTTONS.map((button, index) => ({
      seq: index + 1,
      type: "gesture",
      button: button.id,
    })) as never;
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain(`已检测 ${PROFILE_BUTTONS.length}/${PROFILE_BUTTONS.length}`);
    expect(wrapper.text()).toContain("全部按键可用");

    await wrapper.findAll("footer button")[0].trigger("click");
    expect(wrapper.text()).toContain(`已检测 0/${PROFILE_BUTTONS.length}`);
    expect(wrapper.text()).not.toContain("全部按键可用");
  });

  it("switches tabs and closes on Escape", async () => {
    const wrapper = mountDialog("voice");
    await wrapper.findAll(".test-tab")[1].trigger("click");
    expect(wrapper.text()).toContain("请依次按下遥控器上的各个键");

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape" }));
    expect(wrapper.emitted("close")).toBeTruthy();
  });

  it("shows the transcript area only while the native test switch is on", async () => {
    vi.mocked(api.voiceTestPadText).mockResolvedValue("你好，这是识别内容");
    const wrapper = mountDialog("voice");
    await flushPromises();
    expect(wrapper.find("textarea").exists()).toBe(false);

    const startButton = wrapper.findAll("button").find((b) => b.text().includes("原生测试"));
    expect(startButton).toBeTruthy();
    await startButton!.trigger("click");
    await flushPromises();

    expect(vi.mocked(api.openVoiceTestPad)).toHaveBeenCalled();
    expect(wrapper.find("textarea").exists()).toBe(true);
    const field = wrapper.find("textarea").element as HTMLTextAreaElement;
    expect(field.value).toBe("你好，这是识别内容");
  });

  it("only lists the buttons the connected model has (ARN9 has no TV key)", async () => {
    const wrapper = mountDialog("buttons");
    const store = useEngineStore();
    store.snapshot = {
      connection: {
        phase: "ready",
        remote: {
          remoteId: "r1",
          name: "遥控器",
          model: "ARN9",
          deviceId: "xiaomi-arn9",
          token: "r1",
          isSupported: true,
        },
        generation: 1,
        lastError: null,
      },
      voice: { state: "idle" },
      audio: { state: "unconfigured", endpoint: null },
      capabilities: {},
    } as never;
    await wrapper.vm.$nextTick();
    expect(wrapper.text()).toContain("已检测 0/11");
    expect(wrapper.text()).not.toContain("TV");
  });
});
