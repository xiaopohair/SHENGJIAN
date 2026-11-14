import { describe, expect, it } from "vitest";
import { domCodeForKeyCode } from "./captureBridge";

describe("domCodeForKeyCode", () => {
  it("maps the modifier keys the recorder must distinguish by side", () => {
    expect(domCodeForKeyCode("left_control")).toBe("ControlLeft");
    expect(domCodeForKeyCode("right_control")).toBe("ControlRight");
    expect(domCodeForKeyCode("left_windows")).toBe("MetaLeft");
    expect(domCodeForKeyCode("right_alt")).toBe("AltRight");
  });

  it("maps letters, digits and function keys", () => {
    expect(domCodeForKeyCode("a")).toBe("KeyA");
    expect(domCodeForKeyCode("digit7")).toBe("Digit7");
    expect(domCodeForKeyCode("f5")).toBe("F5");
    expect(domCodeForKeyCode("numpad3")).toBe("Numpad3");
  });

  it("maps punctuation and editing keys", () => {
    expect(domCodeForKeyCode("oem_3")).toBe("Backquote");
    expect(domCodeForKeyCode("backspace")).toBe("Backspace");
    expect(domCodeForKeyCode("left")).toBe("ArrowLeft");
  });

  it("returns null for unknown keys instead of guessing", () => {
    expect(domCodeForKeyCode("control")).toBeNull();
    expect(domCodeForKeyCode("media_play_pause")).toBeNull();
    expect(domCodeForKeyCode("")).toBeNull();
  });
});
