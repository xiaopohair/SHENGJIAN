/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { setLanguagePreference } from "../../lib/i18n";
import { resetAppPickerState } from "../../composables/useAppPicker";
import type { AppTargetInfo, LaunchableApp, LaunchableAppsResult } from "../../lib/types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
}));

vi.mock("../../lib/api", () => ({
  listLaunchableApps: vi.fn(),
  describeAppTargets: vi.fn(),
  canonicalizeAppTarget: vi.fn(),
  errorMessage: (cause: unknown) => String(cause),
}));

import * as api from "../../lib/api";
import AppPickerDialog from "./AppPickerDialog.vue";

enableAutoUnmount(afterEach);

const listLaunchableApps = vi.mocked(api.listLaunchableApps);
const describeAppTargets = vi.mocked(api.describeAppTargets);
const canonicalizeAppTarget = vi.mocked(api.canonicalizeAppTarget);
const openDialog = vi.mocked(openFileDialog);

const APPS: LaunchableApp[] = [
  {
    appId: "preset:wechat",
    displayName: "微信",
    target: "wechat",
    executableName: "WeChat.exe",
    kind: "preset",
    source: "preset",
    installed: true,
    portable: true,
    matchable: true,
  },
  {
    appId: "preset:chrome",
    displayName: "Chrome",
    target: "chrome",
    executableName: "chrome.exe",
    kind: "preset",
    source: "preset",
    installed: false,
    portable: true,
    matchable: true,
  },
  {
    appId: "startMenu:notepad",
    displayName: "记事本",
    target: "C:\\Windows\\notepad.exe",
    executableName: "notepad.exe",
    kind: "path",
    source: "startMenu",
    installed: true,
    portable: false,
    matchable: true,
  },
  {
    appId: "appsFolder:calc",
    displayName: "计算器",
    target: "shell:AppsFolder\\Microsoft.WindowsCalculator",
    executableName: null,
    kind: "aumid",
    source: "appsFolder",
    installed: true,
    portable: false,
    matchable: false,
  },
];

function appsResult(): LaunchableAppsResult {
  return { apps: APPS, scanned: true, scanError: null };
}

function info(overrides: Partial<AppTargetInfo> = {}): AppTargetInfo {
  return {
    input: "wechat",
    target: "wechat",
    kind: "preset",
    displayName: "微信",
    executableName: "WeChat.exe",
    installed: true,
    launchable: true,
    matchable: true,
    portable: true,
    legacy: false,
    issue: null,
    ...overrides,
  };
}

interface MountOptions {
  open?: boolean;
  mode?: "launch" | "match";
  currentTarget?: string;
  foregroundApp?: string;
  recentTargets?: string[];
  title?: string;
}

function mountDialog(options: MountOptions = {}) {
  return mount(AppPickerDialog, {
    attachTo: document.body,
    props: { open: true, ...options },
  });
}

function query<T extends HTMLElement>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) throw new Error(`missing element: ${selector}`);
  return element;
}

function queryAll<T extends HTMLElement>(selector: string): T[] {
  return Array.from(document.querySelectorAll<T>(selector));
}

function optionByText(text: string): HTMLElement {
  const option = queryAll<HTMLElement>(".app-picker-option").find((element) =>
    element.textContent?.includes(text),
  );
  if (!option) throw new Error(`missing app option: ${text}`);
  return option;
}

function buttonByText(text: string): HTMLElement {
  const button = queryAll<HTMLElement>(".app-picker button").find(
    (element) => element.textContent?.trim() === text,
  );
  if (!button) throw new Error(`missing button: ${text}`);
  return button;
}

async function typeSearch(value: string) {
  const input = query<HTMLInputElement>(".app-picker-search input");
  input.value = value;
  input.dispatchEvent(new Event("input"));
  await flushPromises();
}

beforeEach(() => {
  vi.clearAllMocks();
  resetAppPickerState();
  setLanguagePreference("zh-Hans");
  listLaunchableApps.mockResolvedValue(appsResult());
  describeAppTargets.mockImplementation(async (targets: string[]) =>
    targets.map((input) => info({ input, target: input })),
  );
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("AppPickerDialog", () => {
  it("loads the app list once and groups it into suggested and installed apps", async () => {
    mountDialog();
    await flushPromises();

    expect(listLaunchableApps).toHaveBeenCalledWith(false);
    const groups = queryAll(".app-picker-group").map((element) => element.textContent?.trim());
    expect(groups).toContain("推荐");
    expect(groups).toContain("已安装应用");
    expect(optionByText("微信")).toBeTruthy();
    expect(optionByText("未安装").textContent).toContain("Chrome");
  });

  it("switches between all, suggested and installed categories", async () => {
    mountDialog();
    await flushPromises();

    const tabs = () => queryAll<HTMLElement>(".app-picker-filters button");
    expect(tabs().map((tab) => tab.textContent?.trim())).toEqual(["全部", "推荐", "已安装应用"]);

    tabs()[1].click();
    await flushPromises();
    expect(queryAll(".app-picker-group").map((el) => el.textContent?.trim())).toEqual(["推荐"]);
    expect(optionByText("微信")).toBeTruthy();

    tabs()[2].click();
    await flushPromises();
    expect(queryAll(".app-picker-group").map((el) => el.textContent?.trim())).toEqual([
      "已安装应用",
    ]);
    expect(optionByText("记事本")).toBeTruthy();
  });

  it("filters the list from the search box", async () => {
    mountDialog();
    await flushPromises();
    await typeSearch("记事");

    const options = queryAll(".app-picker-option");
    expect(options).toHaveLength(1);
    expect(options[0].textContent).toContain("记事本");
  });

  it("selects the active item with Enter and reports the canonical launch target", async () => {
    const wrapper = mountDialog();
    await flushPromises();

    const input = query<HTMLInputElement>(".app-picker-search input");
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await flushPromises();

    expect(wrapper.emitted("select")?.[0]).toEqual(["wechat"]);
    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("skips disabled entries while moving with the arrow keys", async () => {
    const wrapper = mountDialog();
    await flushPromises();

    const input = query<HTMLInputElement>(".app-picker-search input");
    // The second suggested item (Chrome) is not installed, so Down lands on 记事本.
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowDown", bubbles: true }));
    input.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    await flushPromises();

    expect(wrapper.emitted("select")?.[0]).toEqual(["C:\\Windows\\notepad.exe"]);
  });

  it("emits the executable name in match mode and disables unmatched apps", async () => {
    const wrapper = mountDialog({ mode: "match" });
    await flushPromises();

    optionByText("微信").click();
    await flushPromises();
    expect(wrapper.emitted("select")?.[0]).toEqual(["WeChat.exe"]);

    const calculator = optionByText("计算器");
    expect(calculator.getAttribute("aria-disabled")).toBe("true");
    expect(calculator.textContent).toContain("不支持自动匹配");
    calculator.click();
    await flushPromises();
    expect(wrapper.emitted("select")).toHaveLength(1);
  });

  it("shows the foreground app as its own group", async () => {
    mountDialog({ foregroundApp: "chrome.exe" });
    await flushPromises();

    const groups = queryAll(".app-picker-group").map((element) => element.textContent?.trim());
    expect(groups).toContain("当前前台应用");
    expect(optionByText("chrome.exe")).toBeTruthy();
  });

  it("shows a retry action when loading fails and recovers on retry", async () => {
    listLaunchableApps
      .mockRejectedValueOnce(new Error("scan failed"))
      .mockResolvedValueOnce(appsResult());
    mountDialog();
    await flushPromises();

    expect(document.body.textContent).toContain("无法加载应用列表");
    const retry = queryAll<HTMLElement>(".app-picker-status button").find(
      (element) => element.textContent?.trim() === "重试",
    );
    expect(retry).toBeTruthy();
    retry!.click();
    await flushPromises();
    await flushPromises();

    expect(listLaunchableApps).toHaveBeenCalledTimes(2);
    expect(optionByText("微信")).toBeTruthy();
  });

  it("closes on Escape", async () => {
    const wrapper = mountDialog();
    await flushPromises();

    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await flushPromises();

    expect(wrapper.emitted("close")).toHaveLength(1);
  });

  it("validates a manually entered target before confirming it", async () => {
    canonicalizeAppTarget.mockResolvedValue(
      info({
        input: "chrome.exe",
        target: "chrome",
        displayName: "Chrome",
        executableName: "chrome.exe",
      }),
    );
    const wrapper = mountDialog();
    await flushPromises();

    buttonByText("手动填写（高级）").click();
    await flushPromises();

    const input = query<HTMLInputElement>(".app-picker-manual input");
    input.value = "chrome.exe";
    input.dispatchEvent(new Event("input"));
    await vi.waitFor(() => expect(canonicalizeAppTarget).toHaveBeenCalledWith("chrome.exe"));
    await flushPromises();

    const confirm = queryAll<HTMLButtonElement>(".app-picker-manual-foot button").find(
      (element) => element.textContent?.trim() === "确认",
    );
    expect(confirm).toBeTruthy();
    expect(confirm!.disabled).toBe(false);
    confirm!.click();
    await flushPromises();

    expect(wrapper.emitted("select")?.[0]).toEqual(["chrome"]);
  });

  it("keeps confirm disabled when the manual target cannot be launched", async () => {
    canonicalizeAppTarget.mockResolvedValue(
      info({ input: "ghost.exe", target: "ghost.exe", launchable: false, issue: "path_missing" }),
    );
    mountDialog();
    await flushPromises();

    buttonByText("手动填写（高级）").click();
    await flushPromises();
    const input = query<HTMLInputElement>(".app-picker-manual input");
    input.value = "ghost.exe";
    input.dispatchEvent(new Event("input"));
    await vi.waitFor(() => expect(canonicalizeAppTarget).toHaveBeenCalled());
    await flushPromises();

    expect(document.body.textContent).toContain("路径不存在");
    const confirm = queryAll<HTMLButtonElement>(".app-picker-manual-foot button").find(
      (element) => element.textContent?.trim() === "确认",
    );
    expect(confirm!.disabled).toBe(true);
  });

  it("browses for a file and emits its canonical target", async () => {
    openDialog.mockResolvedValueOnce("C:\\Apps\\chrome.exe");
    canonicalizeAppTarget.mockResolvedValue(
      info({ input: "C:\\Apps\\chrome.exe", target: "chrome", displayName: "Chrome" }),
    );
    const wrapper = mountDialog();
    await flushPromises();

    buttonByText("浏览文件…").click();
    await flushPromises();
    await flushPromises();

    expect(openDialog).toHaveBeenCalledTimes(1);
    expect(wrapper.emitted("select")?.[0]).toEqual(["chrome"]);
  });
});
