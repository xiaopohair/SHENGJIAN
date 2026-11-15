import { describe, expect, it } from "vitest";
import {
  effectiveAudioEndpointId,
  effectiveGainDb,
  effectiveVoiceInputTool,
} from "./effectiveAudio";
import type { SettingsDocument } from "./types";

function document(): SettingsDocument {
  return {
    schemaVersion: 1,
    global: {
      gain_db: 12,
      voice_input_tool: "chatterfly",
      audio_endpoint_id: "global-endpoint",
    },
    devices: {
      // 三项都覆盖：设备独立设置全部生效。
      "xiaomi-rc003": {
        windows: {
          gainDb: 10,
          voiceInputTool: "wechat",
          audioEndpointId: "device-endpoint",
        },
      },
      // 只写了昵称：音频相关的三项都应回落到全局。
      "xiaomi-rc001": { windows: { name: "客厅" } },
    },
  } as unknown as SettingsDocument;
}

describe("effective audio settings", () => {
  it("prefers the per-device override when the device has one", () => {
    expect(effectiveGainDb(document(), "xiaomi-rc003")).toEqual({ value: 10, source: "device" });
    expect(effectiveVoiceInputTool(document(), "xiaomi-rc003")).toEqual({
      value: "wechat",
      source: "device",
    });
    expect(effectiveAudioEndpointId(document(), "xiaomi-rc003")).toEqual({
      value: "device-endpoint",
      source: "device",
    });
  });

  it("falls back to the global value when the override omits the field", () => {
    expect(effectiveGainDb(document(), "xiaomi-rc001")).toEqual({ value: 12, source: "global" });
    expect(effectiveVoiceInputTool(document(), "xiaomi-rc001")).toEqual({
      value: "chatterfly",
      source: "global",
    });
    expect(effectiveAudioEndpointId(document(), "xiaomi-rc001")).toEqual({
      value: "global-endpoint",
      source: "global",
    });
  });

  it("falls back to the global value with no device, an unknown device, or an empty override", () => {
    for (const deviceId of [null, undefined, "", "xiaomi-unknown"]) {
      expect(effectiveGainDb(document(), deviceId)).toEqual({ value: 12, source: "global" });
    }
    const empty = {
      ...document(),
      devices: { "xiaomi-arn9": { windows: { gainDb: undefined, voiceInputTool: "" } } },
    } as unknown as SettingsDocument;
    expect(effectiveGainDb(empty, "xiaomi-arn9")).toEqual({ value: 12, source: "global" });
    expect(effectiveVoiceInputTool(empty, "xiaomi-arn9")).toEqual({
      value: "chatterfly",
      source: "global",
    });
  });

  it("rounds the resolved gain to whole dB like the range control", () => {
    const fractional = {
      ...document(),
      global: { ...document().global, gain_db: 11.6 },
      devices: {},
    } as unknown as SettingsDocument;
    expect(effectiveGainDb(fractional, "xiaomi-rc003")).toEqual({ value: 12, source: "global" });
  });
});
