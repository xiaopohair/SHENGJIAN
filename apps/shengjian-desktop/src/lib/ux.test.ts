import { describe, expect, it } from "vitest";
import { localizeError } from "./errors";
import { voiceLevelPercent } from "./voice";

describe("localizeError", () => {
  it("maps known backend messages for Chinese UI", () => {
    expect(localizeError("invalid backup", "configuration-import", false)).toBe("备份文件无效");
  });

  it("maps known Chinese backend messages for English UI", () => {
    expect(localizeError("无法读取备份文件", undefined, true)).toBe("Cannot read backup file");
  });

  it("maps known engine codes", () => {
    expect(localizeError("voice-not-idle", "voice-not-idle", false)).toBe("语音当前不处于空闲状态");
    expect(localizeError("voice-not-idle", "voice-not-idle", true)).toBe("Voice is not idle");
  });

  it("maps dynamic messages by prefix", () => {
    expect(localizeError("无法启动 BLE 工作线程：test", undefined, true)).toBe(
      "Cannot start BLE worker: test",
    );
  });

  it("maps timeouts and falls back for unknown codes", () => {
    expect(localizeError("operation-timeout", undefined, false)).toBe("操作超时，请重试");
    expect(localizeError("operation-timeout", undefined, true)).toBe("Operation timed out; retry");
    expect(localizeError("some-unknown-code", undefined, false)).toBe("操作失败，请重试");
    expect(localizeError("some-unknown-code", undefined, true)).toBe("Operation failed; retry");
  });

  it("falls back to the original message", () => {
    expect(localizeError("custom failure", "custom", false)).toBe("custom failure");
  });
});

describe("voiceLevelPercent", () => {
  it("maps -60 dBFS to 0% and 0 dBFS to 100%", () => {
    expect(voiceLevelPercent(-60)).toBe(0);
    expect(voiceLevelPercent(0)).toBe(100);
  });

  it("clamps below and above the range", () => {
    expect(voiceLevelPercent(-120)).toBe(0);
    expect(voiceLevelPercent(12)).toBe(100);
  });
});
