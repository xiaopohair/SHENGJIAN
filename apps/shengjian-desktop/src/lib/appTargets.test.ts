import { describe, expect, it } from "vitest";
import {
  appTargetStatus,
  displayNameOf,
  executableNameFromPath,
  isOpenAppTarget,
  issueBadgeKey,
  issueReasonKey,
  mergeAppTarget,
  mergeAppTargetsText,
  splitAppTargets,
  subtitleOf,
} from "./appTargets";
import { hasTranslation } from "./i18n";
import type { AppTargetInfo } from "./types";

function info(overrides: Partial<AppTargetInfo> = {}): AppTargetInfo {
  return {
    input: "C:\\Apps\\x.exe",
    target: "C:\\Apps\\x.exe",
    kind: "path",
    displayName: "X 应用",
    executableName: "x.exe",
    installed: true,
    launchable: true,
    matchable: true,
    portable: false,
    legacy: false,
    issue: null,
    ...overrides,
  };
}

describe("app target text helpers", () => {
  it("splits comma separated targets and trims entries", () => {
    expect(splitAppTargets("chrome.exe, WeChat.exe ,, notepad.exe")).toEqual([
      "chrome.exe",
      "WeChat.exe",
      "notepad.exe",
    ]);
    expect(splitAppTargets(null)).toEqual([]);
    expect(splitAppTargets("   ")).toEqual([]);
  });

  it("appends trimmed targets and ignores duplicates case-insensitively", () => {
    expect(mergeAppTarget(["chrome.exe"], " WeChat.exe ")).toEqual(["chrome.exe", "WeChat.exe"]);
    expect(mergeAppTarget(["chrome.exe"], "CHROME.EXE")).toEqual(["chrome.exe"]);
    expect(mergeAppTarget(["chrome.exe"], "   ")).toEqual(["chrome.exe"]);
    expect(mergeAppTarget([" chrome.exe ", ""], "notepad.exe")).toEqual([
      "chrome.exe",
      "notepad.exe",
    ]);
  });

  it("does not mutate the input array", () => {
    const targets = ["chrome.exe"];
    mergeAppTarget(targets, "notepad.exe");
    expect(targets).toEqual(["chrome.exe"]);
  });

  it("merges the text field form used by the settings drawer", () => {
    expect(mergeAppTargetsText("chrome.exe, wechat.exe", "WeChat.exe")).toBe(
      "chrome.exe, wechat.exe",
    );
    expect(mergeAppTargetsText("", "chrome.exe")).toBe("chrome.exe");
    expect(mergeAppTargetsText("chrome.exe", "notepad.exe")).toBe("chrome.exe, notepad.exe");
  });

  it("recognises only non-empty target strings", () => {
    expect(isOpenAppTarget("chrome.exe")).toBe(true);
    expect(isOpenAppTarget("  ")).toBe(false);
    expect(isOpenAppTarget(null)).toBe(false);
    expect(isOpenAppTarget(42)).toBe(false);
  });

  it("extracts executable names from windows and unix paths", () => {
    expect(executableNameFromPath("C:\\Program Files\\App\\app.exe")).toBe("app.exe");
    expect(executableNameFromPath("/usr/bin/foo")).toBe("foo");
    expect(executableNameFromPath("  ")).toBe("");
  });
});

describe("app target display helpers", () => {
  it("prefers the display name, then the executable name, then the raw target", () => {
    expect(displayNameOf(info())).toBe("X 应用");
    expect(displayNameOf(info({ displayName: null }))).toBe("x.exe");
    expect(displayNameOf(info({ displayName: null, executableName: null }))).toBe(
      "C:\\Apps\\x.exe",
    );
    expect(displayNameOf(null)).toBe("");
  });

  it("only shows a subtitle when it differs from the display name", () => {
    expect(subtitleOf(info())).toBe("x.exe");
    expect(subtitleOf(info({ executableName: null }))).toBe("C:\\Apps\\x.exe");
    expect(
      subtitleOf(
        info({
          displayName: "C:\\Apps\\x.exe",
          executableName: "x.exe",
          target: "C:\\Apps\\x.exe",
        }),
      ),
    ).toBe("x.exe");
  });
});

describe("app target status", () => {
  it("reports a healthy launch target without a badge", () => {
    expect(appTargetStatus(info(), "launch").labelKey).toBeNull();
  });

  it("marks legacy targets as a repairable warning in launch mode", () => {
    const status = appTargetStatus(info({ kind: "legacyName", legacy: true }), "launch");
    expect(status.labelKey).toBe("旧式");
    expect(status.repair).toBe(true);
    expect(status.detailKey).toBe("旧式目标，建议重新选择");
  });

  it("marks unavailable launch targets as errors with a reason", () => {
    const status = appTargetStatus(info({ launchable: false, issue: "path_missing" }), "launch");
    expect(status.labelKey).toBe("路径不存在");
    expect(status.tone).toBe("danger");
    expect(status.repair).toBe(true);
    expect(status.reasonKey).toBe("路径不存在");
  });

  it("explains launch-only targets without offering repair", () => {
    const status = appTargetStatus(
      info({ matchable: false, issue: "non_executable_target" }),
      "launch",
    );
    expect(status.labelKey).toBe("仅启动");
    expect(status.repair).toBe(false);
    expect(status.detailKey).toBe("仅支持启动，不会自动切换方案");
  });

  it("does not treat a bare exe name as legacy noise in match mode", () => {
    const status = appTargetStatus(
      info({ kind: "legacyName", legacy: true, matchable: true }),
      "match",
    );
    expect(status.labelKey).toBeNull();
    expect(status.repair).toBe(false);
  });

  it("warns and offers repair for unresolvable match targets", () => {
    const status = appTargetStatus(
      info({ kind: "legacyName", legacy: true, matchable: false, issue: "bare_name_unresolved" }),
      "match",
    );
    expect(status.labelKey).toBe("无法解析");
    expect(status.tone).toBe("danger");
    expect(status.repair).toBe(true);
    expect(status.reasonKey).toBe("找不到该可执行文件");
  });

  it("explains targets that cannot be matched automatically in match mode", () => {
    const status = appTargetStatus(
      info({ kind: "aumid", executableName: null, matchable: false, issue: "no_executable_name" }),
      "match",
    );
    expect(status.labelKey).toBe("不支持自动匹配");
    expect(status.repair).toBe(false);
  });

  it("maps every issue token to dictionary entries", () => {
    const issues = [
      "empty",
      "unknown_target",
      "preset_not_installed",
      "path_missing",
      "aumid_not_installed",
      "bare_name_unresolved",
      "lnk_unresolved",
      "no_executable_name",
      "non_executable_target",
      "invalid",
      "something_new",
    ];
    for (const issue of issues) {
      expect(hasTranslation(issueReasonKey(issue))).toBe(true);
    }
    for (const issue of [...issues, null]) {
      expect(hasTranslation(issueBadgeKey(issue))).toBe(true);
    }
  });
});
