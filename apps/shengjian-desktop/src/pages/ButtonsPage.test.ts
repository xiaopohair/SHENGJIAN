/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { setLanguagePreference } from "../lib/i18n";
import { RECOMMENDED_HIDDEN_KEY, RECOMMENDED_SEEN_VERSION_KEY } from "../lib/recommended";
import type {
  ButtonProfile,
  ButtonProfilesDocument,
  MacroDefinition,
  RecommendedButtonProfile,
} from "../lib/types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => null),
}));

vi.mock("../lib/api", () => ({
  getButtonProfileState: vi.fn(),
  activateButtonProfile: vi.fn(async () => true),
  clearButtonProfileActivation: vi.fn(async () => undefined),
  errorMessage: (cause: unknown) => String(cause),
  exportProfiles: vi.fn(async () => undefined),
  getForegroundExecutable: vi.fn(async () => null),
  getHostCapabilities: vi.fn(async () => ({
    platform: "windows",
    enhancedCapture: true,
    macroRuntime: true,
    autostart: true,
    tray: true,
    commonPhrases: false,
  })),
  getMacroLibrary: vi.fn(async () => ({ formatVersion: 1, definitions: [] })),
  getVoiceToolCatalog: vi.fn(async () => [
    { id: "wechat", label: "微信输入法", defaultHotkey: ["ControlLeft", "MetaLeft"] },
    { id: "doubao", label: "豆包输入法", defaultHotkey: ["AltRight"] },
  ]),
  importProfiles: vi.fn(),
  resolveAppTarget: vi.fn(async () => null),
  listLaunchableApps: vi.fn(async () => ({ apps: [], scanned: true, scanError: null })),
  describeAppTargets: vi.fn(async () => []),
  canonicalizeAppTarget: vi.fn(),
  appIconDataUrls: vi.fn(async () => []),
  getRecommendedButtonProfiles: vi.fn(),
  copyRecommendedButtonProfile: vi.fn(),
  getRecommendedMacros: vi.fn(),
  saveButtonProfiles: vi.fn(async () => undefined),
}));

import * as api from "../lib/api";
import ButtonsPage from "./ButtonsPage.vue";

enableAutoUnmount(afterEach);

setLanguagePreference("zh-Hans");

const getButtonProfileState = vi.mocked(api.getButtonProfileState);
const saveButtonProfiles = vi.mocked(api.saveButtonProfiles);
const activateButtonProfile = vi.mocked(api.activateButtonProfile);
const getRecommendedButtonProfiles = vi.mocked(api.getRecommendedButtonProfiles);
const copyRecommendedButtonProfile = vi.mocked(api.copyRecommendedButtonProfile);
const getMacroLibrary = vi.mocked(api.getMacroLibrary);
const getRecommendedMacros = vi.mocked(api.getRecommendedMacros);

function macroDefinition(macroId: string, version: string, name: string): MacroDefinition {
  return { schemaVersion: "0.1", macroId, version, name, steps: [] };
}

function emptyProfile(id: string, name: string): ButtonProfile {
  return { id, name, appTargets: [], autoApply: false, bindings: {} };
}

function recommendedProfile(): RecommendedButtonProfile {
  return {
    id: "preset.desktop.shortcuts",
    name: "桌面快捷键",
    deviceModel: null,
    compatibleModels: [],
    appTargets: [],
    autoApply: false,
    bindCount: 2,
    profile: emptyProfile("preset.desktop.shortcuts", "桌面快捷键"),
  };
}

function copiedProfile(): ButtonProfile {
  return {
    ...emptyProfile("profile-copied", "桌面快捷键（我的）"),
    deviceModel: null,
    isDefault: false,
  };
}

function emptyDocument(): ButtonProfilesDocument {
  return { schemaVersion: 1, profiles: [], activeManualProfileId: null };
}

function documentWithProfile(name: string): ButtonProfilesDocument {
  return {
    schemaVersion: 1,
    activeManualProfileId: null,
    profiles: [
      {
        id: "profile-existing",
        name,
        appTargets: [],
        autoApply: false,
        bindings: {},
      },
    ],
  };
}

let currentDocument: ButtonProfilesDocument;

beforeEach(() => {
  vi.clearAllMocks();
  window.localStorage.clear();
  currentDocument = emptyDocument();
  getButtonProfileState.mockImplementation(async () => ({
    document: currentDocument,
    activeProfileId: null,
  }));
  saveButtonProfiles.mockImplementation(async (document: ButtonProfilesDocument) => {
    currentDocument = document;
  });
  activateButtonProfile.mockResolvedValue(true);
  getRecommendedButtonProfiles.mockResolvedValue({
    bundleVersion: "2026.11.1",
    profiles: [],
  });
  copyRecommendedButtonProfile.mockResolvedValue(copiedProfile());
  getRecommendedMacros.mockResolvedValue([]);
});

async function mountPage() {
  // The page reads connection state from the engine store.
  const wrapper = mount(ButtonsPage, { global: { plugins: [createPinia()] } });
  await flushPromises();
  return wrapper;
}

function toolbarButton(wrapper: ReturnType<typeof mount>, label: string) {
  return wrapper
    .findAll(".editor-toolbar-actions button")
    .find((button) => button.text() === label);
}

describe("ButtonsPage profile navigation and editor", () => {
  it("shows the navigation toolbar without chips when there are no profiles", async () => {
    const wrapper = await mountPage();

    expect(wrapper.find(".profile-bar").exists()).toBe(true);
    // No profiles and no draft: the chip strip (and its hint) stays hidden.
    expect(wrapper.find(".profile-chips").exists()).toBe(false);
    expect(wrapper.find(".profile-chips-hint").exists()).toBe(false);

    const toolLabels = wrapper.findAll(".profile-tools button").map((button) => button.text());
    expect(toolLabels).toContain("导入");
    expect(toolLabels).toContain("导出");
    expect(toolLabels).toContain("新建");

    // Nothing to export until a profile exists.
    const exportButton = wrapper.findAll(".profile-tools button")[1];
    expect(exportButton.attributes("disabled")).toBeDefined();

    // Nothing to switch between either.
    expect(wrapper.find(".view-switch").exists()).toBe(false);

    // Without a draft the empty state owns the workspace.
    expect(wrapper.find(".profile-workbench").exists()).toBe(false);
    expect(wrapper.find(".ui-empty").exists()).toBe(true);
  });

  it("opens the edit area right after creating the first profile", async () => {
    const wrapper = await mountPage();

    await wrapper.get(".ui-empty-actions button").trigger("click");
    await flushPromises();

    // Regression: creating the first profile used to render only the title row
    // until the profile had been saved and re-read from disk.
    expect(wrapper.find(".editor-toolbar").exists()).toBe(true);
    expect(wrapper.find(".editor-toolbar").text()).toContain("新建方案");
    expect(wrapper.find(".profile-workbench").exists()).toBe(true);
    expect(wrapper.find(".profile-chip.new-chip").exists()).toBe(true);
    expect(wrapper.find(".view-switch").exists()).toBe(true);
    expect(wrapper.find(".ui-empty").exists()).toBe(false);
  });

  it("keeps existing profile chips while a new draft is being created", async () => {
    currentDocument = documentWithProfile("已有方案");
    const wrapper = await mountPage();

    expect(wrapper.find(".profile-workbench").exists()).toBe(true);
    expect(wrapper.findAll(".profile-chip")).toHaveLength(1);
    expect(wrapper.find(".view-switch").exists()).toBe(true);
    const exportButton = wrapper.findAll(".profile-tools button")[1];
    expect(exportButton.attributes("disabled")).toBeUndefined();

    const newButton = wrapper
      .findAll(".profile-tools button")
      .find((button) => button.text() === "新建");
    expect(newButton).toBeDefined();
    await newButton!.trigger("click");
    await flushPromises();

    expect(wrapper.find(".profile-chip.new-chip").exists()).toBe(true);
    expect(wrapper.findAll(".profile-chip")).toHaveLength(2);
    expect(wrapper.find(".profile-workbench").exists()).toBe(true);
  });

  it("returns to the empty state when a new draft is cancelled", async () => {
    const wrapper = await mountPage();

    await wrapper.get(".ui-empty-actions button").trigger("click");
    await flushPromises();

    expect(wrapper.find(".profile-chips").exists()).toBe(true);
    expect(wrapper.find(".profile-workbench").exists()).toBe(true);

    const cancelButton = toolbarButton(wrapper, "取消");
    expect(cancelButton).toBeDefined();
    await cancelButton!.trigger("click");
    await flushPromises();

    expect(wrapper.find(".profile-chips").exists()).toBe(false);
    expect(wrapper.find(".profile-workbench").exists()).toBe(false);
    expect(wrapper.find(".ui-empty").exists()).toBe(true);
  });

  it("keeps the edit area open after the new profile is saved", async () => {
    const wrapper = await mountPage();

    await wrapper.get(".ui-empty-actions button").trigger("click");
    await flushPromises();

    // The edit area must already exist before the profile is persisted.
    expect(wrapper.find(".profile-workbench").exists()).toBe(true);

    const saveButton = toolbarButton(wrapper, "保存");
    expect(saveButton).toBeDefined();
    await saveButton!.trigger("click");
    await flushPromises();

    expect(saveButtonProfiles).toHaveBeenCalledTimes(1);
    expect(wrapper.find(".profile-chip.new-chip").exists()).toBe(false);
    expect(wrapper.findAll(".profile-chip")).toHaveLength(1);
    expect(wrapper.find(".profile-workbench").exists()).toBe(true);
  });

  it("shows the recommended profiles entry with an update badge and clears it after viewing", async () => {
    window.localStorage.setItem(RECOMMENDED_SEEN_VERSION_KEY, "2026.10.1");
    const wrapper = await mountPage();

    const recommendedButton = wrapper
      .findAll(".profile-tools button")
      .find((button) => button.text().includes("推荐方案"));
    expect(recommendedButton).toBeDefined();
    expect(recommendedButton!.find(".recommended-update-badge").exists()).toBe(true);
    expect(recommendedButton!.text()).toContain("有更新");

    await recommendedButton!.trigger("click");
    await flushPromises();

    expect(document.querySelector(".recommended-dialog")).not.toBeNull();
    expect(window.localStorage.getItem(RECOMMENDED_SEEN_VERSION_KEY)).toBe("2026.11.1");
    expect(wrapper.find(".recommended-update-badge").exists()).toBe(false);
  });

  it("copies a recommended profile, activates it and opens it for editing", async () => {
    getRecommendedButtonProfiles.mockResolvedValue({
      bundleVersion: "2026.11.1",
      profiles: [recommendedProfile()],
    });
    const wrapper = await mountPage();

    const recommendedButton = wrapper
      .findAll(".profile-tools button")
      .find((button) => button.text().includes("推荐方案"));
    await recommendedButton!.trigger("click");
    await flushPromises();

    const copyButton = document.querySelector<HTMLButtonElement>(".recommended-use");
    expect(copyButton).not.toBeNull();
    copyButton!.click();
    await flushPromises();
    await flushPromises();

    expect(copyRecommendedButtonProfile).toHaveBeenCalledWith("preset.desktop.shortcuts", null);
    expect(saveButtonProfiles).toHaveBeenCalledTimes(1);
    const saved = saveButtonProfiles.mock.calls[0][0];
    expect(saved.profiles.map((profile) => profile.id)).toContain("profile-copied");
    expect(activateButtonProfile).toHaveBeenCalledWith("profile-copied");
    expect(wrapper.find(".editor-name").text()).toContain("（我的）");
    expect(document.querySelector(".recommended-dialog")).toBeNull();
  });

  it("hides the recommended profiles entry when the preference is set", async () => {
    window.localStorage.setItem(RECOMMENDED_HIDDEN_KEY, "1");
    const wrapper = await mountPage();

    expect(wrapper.find(".profile-tools").text()).not.toContain("推荐方案");
    expect(getRecommendedButtonProfiles).not.toHaveBeenCalled();
  });

  it("shows recommended-only macros in the run-macro dropdown and prefers user versions", async () => {
    currentDocument = {
      schemaVersion: 1,
      activeManualProfileId: null,
      profiles: [
        {
          id: "profile-macros",
          name: "宏测试方案",
          appTargets: [],
          autoApply: false,
          bindings: {
            home: {
              single: { type: "run-macro", macro_id: "recommended-only" },
            },
          },
        },
      ],
    };
    getMacroLibrary.mockResolvedValueOnce({
      formatVersion: 1,
      definitions: [macroDefinition("shared", "1.0.0", "用户版本")],
    });
    getRecommendedMacros.mockResolvedValue([
      macroDefinition("shared", "9.0.0", "推荐版本"),
      macroDefinition("recommended-only", "1.0.0", "推荐宏"),
    ]);
    const wrapper = await mountPage();

    const macroSelect = wrapper.find(".profile-workbench select");
    expect(macroSelect.exists()).toBe(true);
    const options = macroSelect.findAll("option").map((option) => ({
      value: option.attributes("value"),
      text: option.text(),
    }));
    expect(options).toContainEqual({ value: "recommended-only", text: "推荐宏" });
    // User library wins for the same macro id, even against a newer version.
    expect(options).toContainEqual({ value: "shared", text: "用户版本" });
    expect(options.some((option) => option.text === "推荐版本")).toBe(false);
    // The saved reference keeps its selection and is considered valid.
    expect((macroSelect.element as HTMLSelectElement).value).toBe("recommended-only");
    expect(wrapper.text()).not.toContain("还没有选择宏");
  });
});
