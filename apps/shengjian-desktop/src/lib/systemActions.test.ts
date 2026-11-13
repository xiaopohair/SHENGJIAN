import { describe, expect, it } from "vitest";
import { hasTranslation } from "./i18n";
import {
  isSystemActionKey,
  SYSTEM_KEYS,
  systemActionAllowsRepeat,
  systemKeyLabel,
} from "./systemActions";

describe("system action catalog", () => {
  it("matches the runtime contract key set exactly", () => {
    expect(SYSTEM_KEYS.map((item) => item.key)).toEqual([
      "volume_up",
      "volume_down",
      "volume_mute",
      "media_play_pause",
      "media_prev",
      "media_next",
      "show_desktop",
      "task_switch",
      "escape",
    ]);
  });

  it("recognises only catalogued keys", () => {
    expect(isSystemActionKey("volume_mute")).toBe(true);
    expect(isSystemActionKey("media_play_pause")).toBe(true);
    expect(isSystemActionKey("show_desktop")).toBe(true);
    expect(isSystemActionKey("task_switch")).toBe(true);
    expect(isSystemActionKey("escape")).toBe(true);
    expect(isSystemActionKey("volume_")).toBe(false);
    expect(isSystemActionKey("")).toBe(false);
  });

  it("has translations for every label", () => {
    for (const item of SYSTEM_KEYS) {
      expect(hasTranslation(item.label)).toBe(true);
    }
    expect(systemKeyLabel("volume_up")).toBe("音量 +");
    expect(systemKeyLabel("media_next")).toBe("下一曲");
    expect(systemKeyLabel("show_desktop")).toBe("显示桌面");
    expect(systemKeyLabel("task_switch")).toBe("切换任务");
    expect(systemKeyLabel("escape")).toBe("退出");
  });

  it("falls back to the raw key for unknown values", () => {
    expect(systemKeyLabel("unknown_key")).toBe("unknown_key");
  });

  it("uses a keycap instead of an icon for Escape", () => {
    const escape = SYSTEM_KEYS.find((item) => item.key === "escape");
    expect(escape?.icon).toBeUndefined();
    expect(escape?.keycap).toBe("Esc");
    expect(escape?.label).toBe("退出");
    expect(SYSTEM_KEYS.filter((item) => !item.icon)).toHaveLength(1);
  });

  it("stores real chords for show desktop and task switch", () => {
    const desktop = SYSTEM_KEYS.find((item) => item.key === "show_desktop");
    expect(desktop?.chord).toEqual({ modifiers: ["win"], key: "d" });
    const tasks = SYSTEM_KEYS.find((item) => item.key === "task_switch");
    expect(tasks?.chord).toEqual({ modifiers: ["alt"], key: "tab" });
    expect(systemActionAllowsRepeat("volume_mute")).toBe(true);
    expect(systemActionAllowsRepeat("show_desktop")).toBe(false);
    expect(systemActionAllowsRepeat("task_switch")).toBe(false);
  });
});
