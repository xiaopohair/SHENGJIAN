import { describe, expect, it } from "vitest";
import { formatRelativeTime } from "./relativeTime";
import { setLanguagePreference } from "./i18n";

setLanguagePreference("zh-Hans");

const NOW = 1_700_000_000_000;

describe("relative time labels", () => {
  it("shows just-now for anything under a minute (including future timestamps)", () => {
    expect(formatRelativeTime(NOW, NOW)).toBe("刚刚");
    expect(formatRelativeTime(NOW - 30_000, NOW)).toBe("刚刚");
    expect(formatRelativeTime(NOW + 60_000, NOW)).toBe("刚刚");
  });

  it("shows minutes, hours and days with a single unit", () => {
    expect(formatRelativeTime(NOW - 60_000, NOW)).toBe("1 分钟前");
    expect(formatRelativeTime(NOW - 59 * 60_000, NOW)).toBe("59 分钟前");
    expect(formatRelativeTime(NOW - 60 * 60_000, NOW)).toBe("1 小时前");
    expect(formatRelativeTime(NOW - 3 * 24 * 60 * 60_000, NOW)).toBe("3 天前");
  });

  it("switches to months and years for older entries", () => {
    expect(formatRelativeTime(NOW - 45 * 24 * 60 * 60_000, NOW)).toBe("1 个月前");
    expect(formatRelativeTime(NOW - 400 * 24 * 60 * 60_000, NOW)).toBe("1 年前");
  });
});
