import { describe, expect, it } from "vitest";
import { formatHotkey, normalizeHotkey, sameHotkey, splitHotkey } from "./hotkey";
import { setLanguagePreference } from "./i18n";

setLanguagePreference("zh-Hans");

describe("hotkey helpers", () => {
  it("formats a chord with friendly keycaps", () => {
    expect(formatHotkey(["ControlLeft", "MetaLeft"])).toBe("Ctrl + Win");
    expect(formatHotkey(["KeyA"])).toBe("A");
    expect(formatHotkey([])).toBe("不注入热键");
  });

  it("normalizes modifier order so any click order ends up the same", () => {
    expect(normalizeHotkey(["MetaLeft", "ControlLeft"])).toEqual(["ControlLeft", "MetaLeft"]);
    expect(normalizeHotkey(["KeyA", "ShiftLeft"])).toEqual(["ShiftLeft", "KeyA"]);
  });

  it("treats left and right variants of a modifier as the same chord", () => {
    expect(sameHotkey(["ControlLeft", "MetaLeft"], ["ControlRight", "MetaLeft"])).toBe(true);
    expect(sameHotkey(["ControlLeft"], ["ControlLeft", "KeyA"])).toBe(false);
  });

  it("splits a stored chord back into modifiers and the main key", () => {
    expect(splitHotkey(["ControlLeft", "MetaLeft"])).toEqual({
      modifiers: ["ControlLeft", "MetaLeft"],
      mainKey: "",
    });
    expect(splitHotkey(["AltRight"])).toEqual({ modifiers: ["AltLeft"], mainKey: "" });
    expect(splitHotkey(["ShiftLeft", "KeyA"])).toEqual({
      modifiers: ["ShiftLeft"],
      mainKey: "KeyA",
    });
  });
});
