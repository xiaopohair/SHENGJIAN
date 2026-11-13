/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { createPinia } from "pinia";
import { setLanguagePreference } from "../lib/i18n";
import type { ButtonProfilesDocument } from "../lib/types";

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
  importProfiles: vi.fn(),
  resolveAppTarget: vi.fn(async () => null),
  listLaunchableApps: vi.fn(async () => ({ apps: [], scanned: true, scanError: null })),
  describeAppTargets: vi.fn(async () => []),
  canonicalizeAppTarget: vi.fn(),
  appIconDataUrls: vi.fn(async () => []),
  saveButtonProfiles: vi.fn(async () => undefined),
}));

import * as api from "../lib/api";
import ButtonsPage from "./ButtonsPage.vue";

enableAutoUnmount(afterEach);

setLanguagePreference("zh-Hans");

const getButtonProfileState = vi.mocked(api.getButtonProfileState);
const saveButtonProfiles = vi.mocked(api.saveButtonProfiles);

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
  currentDocument = emptyDocument();
  getButtonProfileState.mockImplementation(async () => ({
    document: currentDocument,
    activeProfileId: null,
  }));
  saveButtonProfiles.mockImplementation(async (document: ButtonProfilesDocument) => {
    currentDocument = document;
  });
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
});
