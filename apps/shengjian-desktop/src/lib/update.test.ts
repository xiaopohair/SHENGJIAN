import { afterEach, describe, expect, it, vi } from "vitest";
import {
  checkForUpdates,
  compareVersions,
  selectLatestRelease,
  type GitHubRelease,
} from "./update";

const releases: GitHubRelease[] = [
  {
    tag_name: "v0.3.0",
    name: "声键 0.3.0",
    html_url: "https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.3.0",
    draft: false,
    prerelease: false,
    published_at: "2026-12-01T00:00:00Z",
  },
  {
    tag_name: "v0.4.0-beta.1",
    name: "声键 0.4.0 beta",
    html_url: "https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.4.0-beta.1",
    draft: false,
    prerelease: true,
    published_at: "2026-12-02T00:00:00Z",
  },
  {
    tag_name: "v0.2.0",
    name: "声键 0.2.0",
    html_url: "https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.2.0",
    draft: false,
    prerelease: false,
    published_at: "2026-11-09T00:00:00Z",
  },
];

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("update checks", () => {
  it("compares stable and prerelease versions", () => {
    expect(compareVersions("0.3.0", "0.2.9")).toBe(1);
    expect(compareVersions("v0.2.0", "0.2.0")).toBe(0);
    expect(compareVersions("0.3.0-beta.1", "0.3.0")).toBe(-1);
    expect(compareVersions("0.3.0-beta.2", "0.3.0-beta.1")).toBe(1);
  });

  it("selects the latest stable release by default", () => {
    expect(selectLatestRelease(releases, false)?.tagName).toBe("v0.3.0");
  });

  it("includes prereleases when requested", () => {
    expect(selectLatestRelease(releases, true)?.tagName).toBe("v0.4.0-beta.1");
  });

  it("reports an available update from GitHub Releases", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        json: async () => releases,
      })),
    );

    const result = await checkForUpdates("0.2.0", false);
    expect(result.updateAvailable).toBe(true);
    expect(result.latest?.tagName).toBe("v0.3.0");
  });

  it("reports no update when the current version is latest", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => ({
        ok: true,
        json: async () => releases,
      })),
    );

    const result = await checkForUpdates("0.3.0", false);
    expect(result.updateAvailable).toBe(false);
  });
});
