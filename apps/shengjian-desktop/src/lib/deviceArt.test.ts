import { describe, expect, it } from "vitest";
import { deviceLineArt } from "./deviceArt";

describe("deviceLineArt", () => {
  it("matches the artwork to the remote model", () => {
    const rc003 = deviceLineArt("RC003");
    expect(deviceLineArt("ARN9")).not.toBe(rc003);
    expect(deviceLineArt("RC001")).not.toBe(rc003);
    // Chromecast 是白色长条遥控器，线稿与三款小米都不同（2026-11-18）。
    expect(deviceLineArt("CHROMECAST")).not.toBe(rc003);
    expect(deviceLineArt("chromecast")).toBe(deviceLineArt("CHROMECAST"));
    expect(deviceLineArt(" Chromecast ")).toBe(deviceLineArt("CHROMECAST"));
  });

  it("falls back to RC003 for generic or unknown models", () => {
    const rc003 = deviceLineArt("RC003");
    expect(deviceLineArt(null)).toBe(rc003);
    expect(deviceLineArt(undefined)).toBe(rc003);
    expect(deviceLineArt("rc003")).toBe(rc003);
    expect(deviceLineArt("UNKNOWN")).toBe(rc003);
  });
});
