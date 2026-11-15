/** @vitest-environment jsdom */
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { VueWrapper } from "@vue/test-utils";
import { flushPromises, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";
import type { CommonPhrasesDocument } from "../lib/types";

const SEED = [
  { button: "up", label: "搜索", text: "用语音输入你想搜的内容" },
  { button: "down", label: "投屏", text: "打开投屏设置" },
  { button: "left", label: "", text: "" },
  { button: "right", label: "稍后处理", text: "这条我先记下了。" },
  { button: "ok", label: "", text: "" },
];

const mocks = vi.hoisted(() => ({
  save: vi.fn(async (document: { slots: unknown[] }) => {
    void document;
  }),
  open: vi.fn(async () => true),
  confirm: vi.fn(async () => true),
  slots: [] as { button: string; label: string; text: string }[],
}));

vi.mock("../lib/api", () => ({
  getCommonPhrasesDocument: vi.fn(async () => ({
    slots: mocks.slots.map((slot) => ({ ...slot })),
  })),
  saveCommonPhrases: mocks.save,
  openCommonPhrasesPanel: mocks.open,
  errorMessage: (cause: unknown) => String(cause),
}));

vi.mock("../lib/confirm", () => ({ confirmAction: mocks.confirm }));

import CommonPhrasesSettings from "./CommonPhrasesSettings.vue";

/** 等待自动保存的合并窗口（250ms）走完。 */
async function settleAutoSave() {
  await new Promise((resolve) => setTimeout(resolve, 320));
  await flushPromises();
}

function buttonByText(wrapper: VueWrapper, text: string) {
  const button = wrapper.findAll("button").find((item) => item.text().trim() === text);
  if (!button) throw new Error(`missing button: ${text}`);
  return button;
}

/** 图标按钮没有文字，按 aria-label 找。 */
function buttonByLabel(wrapper: VueWrapper, label: string) {
  const button = wrapper.findAll("button").find((item) => item.attributes("aria-label") === label);
  if (!button) throw new Error(`missing button: ${label}`);
  return button;
}

beforeEach(() => {
  setLanguagePreference("zh-Hans");
  mocks.slots.splice(0, mocks.slots.length, ...SEED.map((slot) => ({ ...slot })));
  mocks.save.mockClear();
  mocks.open.mockClear();
  mocks.confirm.mockClear();
  mocks.confirm.mockResolvedValue(true);
});

describe("CommonPhrasesSettings", () => {
  it("lists every phrase with its name and text", async () => {
    const wrapper = mount(CommonPhrasesSettings);
    await flushPromises();

    expect(wrapper.findAll("thead th").map((cell) => cell.text())).toEqual([
      "#",
      "名称",
      "内容",
      "操作",
    ]);

    const rows = wrapper.findAll(".phrase-row");
    expect(rows).toHaveLength(5);
    expect(rows.map((row) => (row.find("input").element as HTMLInputElement).value)).toEqual([
      "搜索",
      "投屏",
      "",
      "稍后处理",
      "",
    ]);
    const texts = rows.map((row) => (row.find("textarea").element as HTMLTextAreaElement).value);
    expect(texts[0]).toBe("用语音输入你想搜的内容");
    expect(texts[2]).toBe("");
    // 字数统计在内容输入框内，不再占一条独立的行。
    expect(wrapper.findAll(".phrase-text-count")[0]!.text()).toContain("/500");
    expect(wrapper.text()).toContain("已保存");
  });

  it("adds a phrase and saves it once it has content", async () => {
    const wrapper = mount(CommonPhrasesSettings);
    await flushPromises();

    await buttonByText(wrapper, "添加").trigger("click");
    expect(wrapper.findAll(".phrase-row")).toHaveLength(6);

    // 空行不落盘；填了内容才会进保存载荷。
    const added = () => wrapper.findAll(".phrase-row")[5]!;
    await added().find("input").setValue("新条目");
    await added().find("textarea").setValue("要插入的文本");

    await settleAutoSave();
    const payload = mocks.save.mock.calls.at(-1)![0] as CommonPhrasesDocument;
    expect(payload.slots).toHaveLength(4);
    expect(payload.slots[3]!.label).toBe("新条目");
  });

  it("confirms before removing a phrase and persists the removal", async () => {
    const wrapper = mount(CommonPhrasesSettings);
    await flushPromises();

    const deleteButton = wrapper.findAll(".phrase-row")[3]!.findAll("button")[2]!;
    await deleteButton.trigger("click");
    await flushPromises();

    expect(mocks.confirm).toHaveBeenCalledTimes(1);
    expect(wrapper.findAll(".phrase-row")).toHaveLength(4);
    expect(
      (wrapper.findAll(".phrase-row")[3]!.find("input").element as HTMLInputElement).value,
    ).toBe("");

    await settleAutoSave();
    // 取最后一次落盘：首次挂载期间其它路径也可能触发一次保存，判「最终写的是什么」才有意义。
    const payload = mocks.save.mock.calls.at(-1)![0] as CommonPhrasesDocument;
    // 删掉的「稍后处理」不再出现；两行空白（left / ok）也不落盘。
    expect(payload.slots.map((slot) => slot.label)).toEqual(["搜索", "投屏"]);
  });

  it("does not persist a row added but left blank", async () => {
    const wrapper = mount(CommonPhrasesSettings);
    await flushPromises();

    await buttonByText(wrapper, "添加").trigger("click");
    await flushPromises();
    expect(wrapper.findAll(".phrase-row")).toHaveLength(6);

    await settleAutoSave();
    const payload = mocks.save.mock.calls.at(-1)![0] as CommonPhrasesDocument;
    expect(payload.slots).toHaveLength(3);
    expect(payload.slots.some((slot) => slot.label === "" && slot.text === "")).toBe(false);
  });

  it("saves pending edits before opening the floating panel", async () => {
    const wrapper = mount(CommonPhrasesSettings);
    await flushPromises();

    await wrapper.findAll("textarea")[0]!.setValue("用语音输入你想搜的内容。");
    // 自动保存窗口还没走完就点预览：面板会重新读盘，必须先落盘。
    await buttonByLabel(wrapper, "预览面板").trigger("click");
    await flushPromises();

    expect(mocks.save).toHaveBeenCalledTimes(1);
    const payload = mocks.save.mock.calls[0]![0] as CommonPhrasesDocument;
    expect(payload.slots[0]!.text).toBe("用语音输入你想搜的内容。");
    expect(mocks.open).toHaveBeenCalledTimes(1);
  });

  it("shows the empty state when no phrase is configured", async () => {
    mocks.slots.splice(0, mocks.slots.length);
    const wrapper = mount(CommonPhrasesSettings);
    await flushPromises();

    expect(wrapper.findAll(".phrase-row")).toHaveLength(0);
    expect(wrapper.text()).toContain("还没有常用语");
  });
});
