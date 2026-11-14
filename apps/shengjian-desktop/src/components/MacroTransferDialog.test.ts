/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { enableAutoUnmount, flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";
import type { MacroDefinition, MacroLibraryDocument } from "../lib/types";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
  save: vi.fn(async () => null),
}));

vi.mock("../lib/api", () => ({
  errorMessage: (cause: unknown) => String(cause),
  exportMacroLibrary: vi.fn(async () => undefined),
  previewImportMacroLibrary: vi.fn(),
  saveMacroLibrary: vi.fn(async (document: MacroLibraryDocument) => document),
}));

import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import * as api from "../lib/api";
import MacroTransferDialog from "./MacroTransferDialog.vue";

enableAutoUnmount(afterEach);
setLanguagePreference("zh-Hans");

// 每个用例只看自己的调用次数，避免上一个用例留下的记录干扰断言。
beforeEach(() => {
  vi.clearAllMocks();
});

function macro(macroId: string, name: string, version = "1"): MacroDefinition {
  return {
    schemaVersion: "1.0",
    macroId,
    version,
    name,
    summary: null,
    steps: [],
  };
}

const DOCUMENT: MacroLibraryDocument = {
  formatVersion: 1,
  definitions: [
    macro("a", "播放 / 暂停"),
    macro("b", "截图并标注", "3"),
    macro("c", "唤醒客厅电脑"),
  ],
};

const INCOMING: MacroLibraryDocument = {
  formatVersion: 1,
  definitions: [macro("a", "播放 / 暂停"), macro("b", "截图并标注", "4"), macro("d", "清理桌面")],
};

function mountDialog(doc: MacroLibraryDocument = DOCUMENT) {
  return mount(MacroTransferDialog, {
    attachTo: document.body,
    props: { open: true, document: doc },
    global: { stubs: { teleport: true } },
  });
}

function primaryButton(wrapper: ReturnType<typeof mountDialog>) {
  return wrapper.findAll(".transfer-foot .foot-actions button").at(-1)!;
}

async function openPreview(wrapper: ReturnType<typeof mountDialog>) {
  await wrapper.findAll(".transfer-seg button")[1].trigger("click");
  await wrapper.find(".drop-zone").trigger("click");
  await flushPromises();
}

describe("MacroTransferDialog", () => {
  it("exports the picked subset into one file", async () => {
    vi.mocked(saveDialog).mockResolvedValueOnce("C:\\tmp\\macros.json");
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    expect(wrapper.findAll(".macro-row")).toHaveLength(3);
    // 取消第二个宏，只导出剩下两条。
    await wrapper.findAll(".macro-row")[1].trigger("click");
    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const exported = vi.mocked(api.exportMacroLibrary);
    expect(exported).toHaveBeenCalledTimes(1);
    const [path, document] = exported.mock.calls[0];
    expect(path).toBe("C:\\tmp\\macros.json");
    expect(document.definitions.map((item) => item.macroId)).toEqual(["a", "c"]);
    expect(wrapper.text()).toContain("已导出 2 个宏");
  });

  it("previews an import and merges it into the local library", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\tmp\\in.json");
    vi.mocked(api.previewImportMacroLibrary).mockResolvedValueOnce(INCOMING);

    const wrapper = mountDialog();
    await openPreview(wrapper);

    const rows = wrapper.findAll(".import-row");
    expect(rows).toHaveLength(3);
    expect(wrapper.text()).toContain("相同");
    expect(wrapper.text()).toContain("更新");
    expect(wrapper.text()).toContain("新增");
    expect(wrapper.find(".foot-summary").text()).toContain("将导入 2 个宏");

    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const saved = vi.mocked(api.saveMacroLibrary);
    expect(saved).toHaveBeenCalledTimes(1);
    const merged = saved.mock.calls[0][0];
    // 本机独有的 c 保留，b 被 v4 覆盖，d 追加。
    expect(merged.definitions.map((item) => `${item.macroId}:${item.version}`)).toEqual([
      "a:1",
      "b:4",
      "c:1",
      "d:1",
    ]);
    expect(wrapper.emitted("applied")).toHaveLength(1);
    expect(wrapper.text()).toContain("已导入 2 个宏");
  });

  it("replaces the library on request and lists the macros it deletes", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\tmp\\in.json");
    vi.mocked(api.previewImportMacroLibrary).mockResolvedValueOnce(INCOMING);

    const wrapper = mountDialog();
    await openPreview(wrapper);

    // 第一组分段是「导入方式」：切到替换整库。
    await wrapper.findAll(".option-seg")[0].findAll("button")[1].trigger("click");
    expect(wrapper.text()).toContain("会删除 1 个本机独有的宏");

    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const saved = vi.mocked(api.saveMacroLibrary);
    const replaced = saved.mock.calls[0][0];
    expect(replaced.definitions.map((item) => item.macroId)).toEqual(["a", "b", "d"]);
    expect(wrapper.text()).toContain("整库已替换");
    // 结果页把被删掉的宏名字列出来。
    expect(wrapper.find(".dropped-list").text()).toContain("唤醒客厅电脑");
  });

  it("undoes an import by writing the previous library back", async () => {
    vi.mocked(openDialog).mockResolvedValueOnce("C:\\tmp\\in.json");
    vi.mocked(api.previewImportMacroLibrary).mockResolvedValueOnce(INCOMING);

    const wrapper = mountDialog();
    await openPreview(wrapper);
    await primaryButton(wrapper).trigger("click");
    await flushPromises();

    const undo = wrapper.findAll(".result-actions button")[0];
    expect(undo.text()).toContain("撤销导入");
    await undo.trigger("click");
    await flushPromises();

    const saved = vi.mocked(api.saveMacroLibrary);
    expect(saved).toHaveBeenCalledTimes(2);
    expect(saved.mock.calls[1][0].definitions.map((item) => item.macroId)).toEqual(["a", "b", "c"]);
    expect(wrapper.emitted("applied")).toHaveLength(2);
    expect(wrapper.find(".result-title").exists()).toBe(false);
    expect(wrapper.findAll(".import-row")).toHaveLength(3);
  });
});
