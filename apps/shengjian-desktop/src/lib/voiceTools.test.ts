import { describe, expect, it } from "vitest";
import {
  toolHotkey,
  withToolHotkey,
  defaultHotkeyFor,
  requiresReportLayerSynthesis,
  voiceToolLabel,
  voiceToolOptions,
} from "./voiceTools";
import type { VoiceToolInfo } from "./types";

const catalog: VoiceToolInfo[] = [
  { id: "wechat", label: "微信输入法", defaultHotkey: ["ControlLeft", "MetaLeft"] },
  { id: "doubao", label: "豆包输入法", defaultHotkey: ["AltRight"] },
  { id: "blank", label: "  ", defaultHotkey: [] },
];

describe("voice tool catalog helpers", () => {
  it("flags the IMEs that drop injected keys as needing report-layer synthesis", () => {
    expect(requiresReportLayerSynthesis("doubao")).toBe(true);
    expect(requiresReportLayerSynthesis(" DOUBAO ")).toBe(true);
    // Chatterfly 与豆包同集合（2026-11-16 实测：注入没反应、报告层能拉起）。
    expect(requiresReportLayerSynthesis("chatterfly")).toBe(true);
    expect(requiresReportLayerSynthesis(" Chatterfly ")).toBe(true);
    // 微信输入法吃注入键，不需要助手。
    expect(requiresReportLayerSynthesis("wechat")).toBe(false);
    expect(requiresReportLayerSynthesis("vokie")).toBe(false);
    expect(requiresReportLayerSynthesis("")).toBe(false);
    expect(requiresReportLayerSynthesis(null)).toBe(false);
  });

  it("remembers each tool's custom hotkey across tool switches", () => {
    // A 配好自定义 → 切到 B（B 没有记录，回落到默认 = null）→ 切回 A 仍是原来的值。
    let map = withToolHotkey({}, "doubao", ["RightAlt"]);
    expect(toolHotkey(map, "doubao")).toEqual(["RightAlt"]);
    expect(toolHotkey(map, "chatterfly")).toBeNull();

    map = withToolHotkey(map, "chatterfly", ["ControlLeft", "ShiftLeft"]);
    expect(toolHotkey(map, "doubao")).toEqual(["RightAlt"]);
    expect(toolHotkey(map, "chatterfly")).toEqual(["ControlLeft", "ShiftLeft"]);

    // 恢复默认 = 从表里删除该工具，不影响其它工具。
    map = withToolHotkey(map, "doubao", null);
    expect(toolHotkey(map, "doubao")).toBeNull();
    expect(toolHotkey(map, "chatterfly")).toEqual(["ControlLeft", "ShiftLeft"]);

    // 空工具 id / 空键列表都不落表。
    expect(withToolHotkey(map, "  ", ["ControlLeft"])).toEqual(map);
    expect(withToolHotkey(map, "vokie", [])).toEqual(map);
    expect(toolHotkey(null, "doubao")).toBeNull();
  });

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
