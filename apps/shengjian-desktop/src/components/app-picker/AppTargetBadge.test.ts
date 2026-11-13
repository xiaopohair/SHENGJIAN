/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import { flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../../lib/i18n";
import { resetAppPickerState } from "../../composables/useAppPicker";
import type { AppTargetInfo } from "../../lib/types";

vi.mock("../../lib/api", () => ({
  describeAppTargets: vi.fn(),
  listLaunchableApps: vi.fn(async () => ({ apps: [], scanned: true, scanError: null })),
  canonicalizeAppTarget: vi.fn(),
  errorMessage: (cause: unknown) => String(cause),
}));

import * as api from "../../lib/api";
import AppTargetBadge from "./AppTargetBadge.vue";

const describeAppTargets = vi.mocked(api.describeAppTargets);

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

function mountBadge(props: { target: string; mode?: "launch" | "match" }) {
  return mount(AppTargetBadge, { props });
}

beforeEach(() => {
  vi.clearAllMocks();
  resetAppPickerState();
  setLanguagePreference("zh-Hans");
  describeAppTargets.mockImplementation(async (targets: string[]) =>
    targets.map((input) => info({ input })),
  );
});

describe("AppTargetBadge", () => {
  it("shows the friendly name, executable subtitle and full target tooltip", async () => {
    describeAppTargets.mockResolvedValue([info()]);
    const wrapper = mountBadge({ target: "C:\\Apps\\x.exe" });
    await flushPromises();

    expect(wrapper.text()).toContain("X 应用");
    expect(wrapper.text()).toContain("x.exe");
    expect(wrapper.attributes("title")).toBe("C:\\Apps\\x.exe");
    expect(wrapper.find(".app-target-repair").exists()).toBe(false);
  });

  it("falls back to the raw target while the description is loading", () => {
    describeAppTargets.mockImplementation(() => new Promise(() => {}));
    const wrapper = mountBadge({ target: "C:\\Apps\\x.exe" });

    expect(wrapper.text()).toContain("C:\\Apps\\x.exe");
    expect(wrapper.find(".app-target-flag").exists()).toBe(false);
  });

  it("flags legacy launch targets and emits repair", async () => {
    describeAppTargets.mockResolvedValue([info({ kind: "legacyName", legacy: true })]);
    const wrapper = mountBadge({ target: "x.exe" });
    await flushPromises();

    expect(wrapper.text()).toContain("旧式");
    const repair = wrapper.find(".app-target-repair");
    expect(repair.exists()).toBe(true);
    await repair.trigger("click");
    expect(wrapper.emitted("repair")).toHaveLength(1);
  });

  it("flags unavailable launch targets with the concrete reason", async () => {
    describeAppTargets.mockResolvedValue([
      info({ launchable: false, installed: false, issue: "path_missing" }),
    ]);
    const wrapper = mountBadge({ target: "D:\\Gone\\app.exe" });
    await flushPromises();

    expect(wrapper.text()).toContain("路径不存在");
    expect(wrapper.find(".app-target-repair").exists()).toBe(true);
    expect(wrapper.find(".app-target-flag").attributes("title")).toContain("路径不存在");
  });

  it("shows launch-only as information without a repair entry", async () => {
    describeAppTargets.mockResolvedValue([
      info({ matchable: false, issue: "non_executable_target" }),
    ]);
    const wrapper = mountBadge({ target: "C:\\Tools\\run.bat" });
    await flushPromises();

    expect(wrapper.text()).toContain("仅启动");
    expect(wrapper.find(".app-target-repair").exists()).toBe(false);
  });

  it("does not show legacy noise for match-mode targets", async () => {
    describeAppTargets.mockResolvedValue([
      info({ kind: "legacyName", legacy: true, matchable: true }),
    ]);
    const wrapper = mountBadge({ target: "x.exe", mode: "match" });
    await flushPromises();

    expect(wrapper.text()).not.toContain("旧式");
    expect(wrapper.find(".app-target-flag").exists()).toBe(false);
  });

  it("warns when a match target cannot be resolved", async () => {
    describeAppTargets.mockResolvedValue([
      info({
        input: "missing.exe",
        target: "missing.exe",
        kind: "legacyName",
        legacy: true,
        matchable: false,
        issue: "bare_name_unresolved",
      }),
    ]);
    const wrapper = mountBadge({ target: "missing.exe", mode: "match" });
    await flushPromises();

    expect(wrapper.text()).toContain("无法解析");
    expect(wrapper.find(".app-target-flag").attributes("title")).toContain("找不到该可执行文件");
    const repair = wrapper.find(".app-target-repair");
    expect(repair.exists()).toBe(true);
    await repair.trigger("click");
    expect(wrapper.emitted("repair")).toHaveLength(1);
  });

  it("keeps the raw target visible when describing fails", async () => {
    describeAppTargets.mockRejectedValue(new Error("boom"));
    const wrapper = mountBadge({ target: "C:\\Apps\\x.exe" });
    await flushPromises();

    expect(wrapper.text()).toContain("C:\\Apps\\x.exe");
    expect(wrapper.find(".app-target-flag").exists()).toBe(false);
  });
});
