import { describe, expect, it } from "vitest";
import { domCodeForKeyCode, macroKeyFromDomCode, modifierForDomCode } from "./captureBridge";

describe("captureBridge", () => {
  it("maps hook KeyCode names back to DOM codes", () => {
    expect(domCodeForKeyCode("left_control")).toBe("ControlLeft");
    expect(domCodeForKeyCode("right_windows")).toBe("MetaRight");
    expect(domCodeForKeyCode("a")).toBe("KeyA");
    expect(domCodeForKeyCode("digit7")).toBe("Digit7");
    expect(domCodeForKeyCode("f5")).toBe("F5");
  });

  it("returns null for keys it cannot map instead of guessing", () => {
    expect(domCodeForKeyCode("control")).toBeNull();
    expect(domCodeForKeyCode("media_play_pause")).toBeNull();
    expect(domCodeForKeyCode("")).toBeNull();
  });

  it("recognizes modifier sides as the same macro modifier", () => {
    expect(modifierForDomCode("ControlLeft")).toBe("control");
    expect(modifierForDomCode("ControlRight")).toBe("control");
    expect(modifierForDomCode("AltRight")).toBe("alt");
    expect(modifierForDomCode("MetaLeft")).toBe("win");
    expect(modifierForDomCode("KeyA")).toBeNull();
  });

  it("converts DOM codes to the macro runtime key names", () => {
    expect(macroKeyFromDomCode("KeyA")).toBe("a");
    expect(macroKeyFromDomCode("Digit1")).toBe("1");
    expect(macroKeyFromDomCode("F12")).toBe("f12");
    expect(macroKeyFromDomCode("NumpadMultiply")).toBe("numpadmultiply");
    expect(macroKeyFromDomCode("Space")).toBe("space");
    expect(macroKeyFromDomCode("ControlLeft")).toBeNull();
  });
});
