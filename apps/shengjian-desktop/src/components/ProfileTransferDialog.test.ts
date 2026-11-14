/** @vitest-environment jsdom */
import { afterEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";
import type { ButtonProfile, ButtonProfilesDocument } from "../lib/types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => null),
}));

vi.mock("../lib/api", () => ({
  errorMessage: (cause: unknown) => String(cause),
  exportProfiles: vi.fn(async () => undefined),
  exportProfileAsPreset: vi.fn(async () => ({})),
  previewImportProfiles: vi.fn(),
  saveButtonProfiles: vi.fn(async () => undefined),
}));

import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import * as api from "../lib/api";
import ProfileTransferDialog from "./ProfileTransferDialog.vue";

enableAutoUnmount(afterEach);
setLanguagePreference("zh-Hans");

function profile(overrides: Partial<ButtonProfile> = {}): ButtonProfile {
  return {
    id: "profile-a",
    name: "方案 A",
    appTargets: [],
    autoApply: false,
    deviceModel: "RC003",
    bindings: {},
    ...overrides,
  };
}

const DOCUMENT: ButtonProfilesDocument = {
  schemaVersion: 2,
  activeManualProfileId: "profile-a",
  profiles: [
    profile({ id: "default-rc003", name: "默认", isDefault: true }),
    profile({ id: "profile-a", name: "方案 A" }),
    profile({ id: "profile-b", name: "方案 B", deviceModel: null }),
  ],
};

const INCOMING: ButtonProfilesDocument = {
  schemaVersion: 2,
  activeManualProfileId: null,
  profiles: [
    profile({ id: "profile-a", name: "方案 A", autoApply: true }),
    profile({ id: "profile-c", name: "方案 C" }),
    profile({ id: "incoming-b", name: "方案 B" }),
  ],
};

function mountDialog(doc: ButtonProfilesDocument = DOCUMENT) {
  return mount(ProfileTransferDialog, {
    attachTo: document.body,
    props: { open: true, document: doc, model: "RC003" },
    // 弹窗用 Teleport 挂到 body；stub 掉后内容留在 wrapper 里便于断言。
    global: { stubs: { teleport: true } },
  });
}

function primaryButton(wrapper: ReturnType<typeof mountDialog>) {
  return wrapper.findAll(".transfer-foot button").at(-1)!;
}

describe("ProfileTransferDialog", () => {
  it("selects every profile by default and exports the chosen subset", async () => {
    vi.mocked(saveDialog).mockResolvedValueOnce("C:\\tmp\\out.json");
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    const rows = wrapper.findAll(".profile-row");
    expect(rows).toHaveLength(3);
    expect(wrapper.findAll(".profile-row.selected")).toHaveLength(3);
    expect(wrapper.find(".pane-count").text()).toContain("3/3");

    // 去掉方案 B，只导出前两个。
    await wrapper.findAll(".profile-row")[2].trigger("click");
    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const exported = vi.mocked(api.exportProfiles);
    expect(exported).toHaveBeenCalledTimes(1);
    const [path, document] = exported.mock.calls[0];
    expect(path).toBe("C:\\tmp\\out.json");
    expect(document.profiles.map((item) => item.id)).toEqual(["default-rc003", "profile-a"]);
    expect(wrapper.text()).toContain("已导出 2 个方案");
  });

  it("writes one preset file per profile and reports the count", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\tmp\\presets");
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    // 第二张格式卡片是「推荐方案」：选文件夹，逐条写文件。
    await wrapper.findAll(".option-card")[1].trigger("click");
    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const writer = vi.mocked(api.exportProfileAsPreset);
    expect(writer).toHaveBeenCalledTimes(2);
    expect(writer.mock.calls.map((call) => call[0].presetId)).toEqual([
      "preset.rc003.方案-a",
      "preset.rc003.方案-b",
    ]);
    expect(wrapper.text()).toContain("已导出 2 个方案");
  });

  it("drops default profiles when exporting recommended profiles", async () => {
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    // 第二张格式卡片是「推荐方案」。
    await wrapper.findAll(".option-card")[1].trigger("click");
    await wrapper.vm.$nextTick();

    const rows = wrapper.findAll(".profile-row");
    expect(rows[0].classes()).toContain("disabled");
    expect(wrapper.findAll(".profile-row.selected")).toHaveLength(2);

    await rows[0].trigger("click");
    expect(wrapper.findAll(".profile-row.selected")).toHaveLength(2);
  });

  it("previews an import, resolves the conflict per row, then applies it", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\tmp\\in.json");
    vi.mocked(api.previewImportProfiles).mockResolvedValueOnce(INCOMING);

    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    await wrapper.findAll(".transfer-seg button")[1].trigger("click");
    expect(wrapper.find(".drop-zone").exists()).toBe(true);

    await wrapper.find(".drop-zone").trigger("click");
    await flushPromises();

    const rows = wrapper.findAll(".import-row");
    expect(rows).toHaveLength(3);
    expect(wrapper.text()).toContain("冲突");
    expect(wrapper.text()).toContain("更新");

    // 冲突行默认跟随策略（覆盖）；手动改成保留两者。
    const segment = wrapper.find(".mini-segment");
    await segment.findAll("button")[1].trigger("click");
    await wrapper.vm.$nextTick();
    expect(wrapper.find(".foot-summary").text()).toContain("保留 1");

    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const saved = vi.mocked(api.saveButtonProfiles);
    expect(saved).toHaveBeenCalledTimes(1);
    const merged = saved.mock.calls[0][0];
    const names = merged.profiles.map((item) => item.name);
    expect(names).toContain("方案 A");
    expect(names).toContain("方案 C");
    expect(names).toContain("方案 B（副本）");
    // 本机原有的方案一条都没少。
    expect(merged.profiles).toHaveLength(5);
    expect(wrapper.emitted("applied")).toHaveLength(1);
  });

  it("keeps the file readable when parsing fails", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\tmp\\broken.json");
    vi.mocked(api.previewImportProfiles).mockRejectedValueOnce(new Error("invalid profiles file"));

    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    await wrapper.findAll(".transfer-seg button")[1].trigger("click");
    await wrapper.find(".drop-zone").trigger("click");
    await flushPromises();

    expect(wrapper.text()).toContain("这个文件读不了");
    expect(wrapper.text()).toContain("invalid profiles file");
    expect(wrapper.emitted("applied")).toBeUndefined();
  });
});
