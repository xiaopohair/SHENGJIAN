import { describe, expect, it } from "vitest";
import { defaultHotkeyFor, voiceToolLabel, voiceToolOptions } from "./voiceTools";
import type { VoiceToolInfo } from "./types";

const catalog: VoiceToolInfo[] = [
  { id: "wechat", label: "微信输入法", defaultHotkey: ["ControlLeft", "MetaLeft"] },
  { id: "doubao", label: "豆包输入法", defaultHotkey: ["AltRight"] },
  { id: "blank", label: "  ", defaultHotkey: [] },
];

describe("voice tool catalog helpers", () => {
  it("maps catalog entries to dropdown options without translating labels", () => {
    const options = voiceToolOptions(catalog, "");
    expect(options).toEqual([
      { id: "wechat", label: "微信输入法", fallback: false },
      { id: "doubao", label: "豆包输入法", fallback: false },
      { id: "blank", label: "blank", fallback: false },
    ]);
  });

  it("appends the saved value as a fallback option when it is not in the catalog", () => {
    const options = voiceToolOptions(catalog, " legacy ");
    expect(options.at(-1)).toEqual({ id: "legacy", label: "legacy", fallback: true });
    expect(options.filter((option) => option.fallback)).toHaveLength(1);
  });

  it("does not duplicate a saved id that is already in the catalog", () => {
    const options = voiceToolOptions(catalog, "wechat");
    expect(options).toHaveLength(catalog.length);
    expect(options.some((option) => option.fallback)).toBe(false);
    expect(voiceToolOptions(catalog, null)).toHaveLength(catalog.length);
  });

  it("does not mutate the catalog array", () => {
    const before = catalog.map((tool) => tool.id);
    voiceToolOptions(catalog, "legacy");
    expect(catalog.map((tool) => tool.id)).toEqual(before);
  });

  it("resolves display labels and keeps unknown ids visible", () => {
    expect(voiceToolLabel(catalog, "wechat")).toBe("微信输入法");
    expect(voiceToolLabel(catalog, "legacy")).toBe("legacy");
    expect(voiceToolLabel(catalog, "")).toBe("");
    expect(voiceToolLabel([], "wechat")).toBe("wechat");
  });

  it("returns the configured default hotkey sequence", () => {
    expect(defaultHotkeyFor(catalog, "wechat")).toEqual(["ControlLeft", "MetaLeft"]);
    expect(defaultHotkeyFor(catalog, "doubao")).toEqual(["AltRight"]);
    expect(defaultHotkeyFor(catalog, "legacy")).toEqual([]);
    expect(defaultHotkeyFor(catalog, "")).toEqual([]);
  });

  it("hands out a copy so callers cannot mutate the catalog entry", () => {
    const hotkey = defaultHotkeyFor(catalog, "wechat");
    hotkey.push("KeyA");
    expect(catalog[0].defaultHotkey).toEqual(["ControlLeft", "MetaLeft"]);
  });
});
