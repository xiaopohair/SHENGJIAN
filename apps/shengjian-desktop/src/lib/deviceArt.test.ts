import { describe, expect, it } from "vitest";
import { deviceLineArt } from "./deviceArt";

describe("deviceLineArt", () => {
  it("matches the artwork to the remote model", () => {
    const rc003 = deviceLineArt("RC003");
    expect(deviceLineArt("ARN9")).not.toBe(rc003);
    expect(deviceLineArt("RC001")).not.toBe(rc003);
  });

  it("falls back to RC003 for generic or unknown models", () => {
    const rc003 = deviceLineArt("RC003");
    expect(deviceLineArt(null)).toBe(rc003);
    expect(deviceLineArt(undefined)).toBe(rc003);
    expect(deviceLineArt("rc003")).toBe(rc003);
    expect(deviceLineArt("UNKNOWN")).toBe(rc003);
  });
});
