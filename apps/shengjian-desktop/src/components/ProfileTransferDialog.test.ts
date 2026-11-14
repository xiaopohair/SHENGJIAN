/** @vitest-environment jsdom */
import { afterEach, describe, expect, it } from "vitest";
import { enableAutoUnmount, mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";
import type { ButtonProfile } from "../lib/types";
import ProfileTransferDialog from "./ProfileTransferDialog.vue";

enableAutoUnmount(afterEach);

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

const PROFILES: ButtonProfile[] = [
  profile({ id: "default-rc003", name: "默认", isDefault: true }),
  profile({ id: "profile-a", name: "方案 A" }),
  profile({ id: "profile-b", name: "方案 B", deviceModel: null }),
];

setLanguagePreference("zh-Hans");

function mountDialog() {
  return mount(ProfileTransferDialog, {
    attachTo: document.body,
    props: { open: true, profiles: PROFILES },
    // 弹窗用 Teleport 挂到 body；stub 掉后内容留在 wrapper 里便于断言。
    global: { stubs: { teleport: true } },
  });
}

function primaryButton(wrapper: ReturnType<typeof mountDialog>) {
  return wrapper.findAll(".transfer-foot button").at(-1)!;
}

function formatButton(wrapper: ReturnType<typeof mountDialog>, title: string) {
  return wrapper
    .findAll(".transfer-formats button")
    .find((button) => button.text().includes(title))!;
}

describe("ProfileTransferDialog", () => {
  it("starts with every profile selected and exports the chosen subset", async () => {
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    const boxes = wrapper.findAll('.transfer-item input[type="checkbox"]');
    expect(boxes).toHaveLength(3);
    expect(boxes.every((box) => (box.element as HTMLInputElement).checked)).toBe(true);
    expect(wrapper.find(".transfer-count").text()).toContain("3/3");

    // 取消勾选方案 B，只导出前两个。
    await boxes[2].setValue(false);
    await primaryButton(wrapper).trigger("click");

    expect(wrapper.emitted("exportDocument")?.[0]?.[0]).toEqual(["default-rc003", "profile-a"]);
  });

  it("switches to import mode and emits import from the footer action", async () => {
    const wrapper = mountDialog();
    const importTab = wrapper
      .findAll(".transfer-tab-group button")
      .find((button) => button.text() === "导入")!;
    await importTab.trigger("click");

    expect(wrapper.find(".transfer-import").exists()).toBe(true);
    expect(wrapper.find(".transfer-list").exists()).toBe(false);

    await primaryButton(wrapper).trigger("click");
    expect(wrapper.emitted("import")).toHaveLength(1);
  });

  it("drops default profiles when exporting recommended profiles", async () => {
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    await formatButton(wrapper, "推荐方案").trigger("click");
    await wrapper.vm.$nextTick();

    const boxes = wrapper.findAll('.transfer-item input[type="checkbox"]');
    // 默认方案在推荐方案模式下不可选，且已被自动剔除。
    expect((boxes[0].element as HTMLInputElement).disabled).toBe(true);
    expect((boxes[0].element as HTMLInputElement).checked).toBe(false);
    expect(wrapper.find(".transfer-count").text()).toContain("2/3");
    expect(wrapper.find(".transfer-format.active").text()).toContain("推荐方案");

    await primaryButton(wrapper).trigger("click");
    expect(wrapper.emitted("exportPreset")?.[0]?.[0]).toEqual(["profile-a", "profile-b"]);
  });

  it("disables the export action when nothing is selected", async () => {
    const wrapper = mountDialog();
    await wrapper.vm.$nextTick();

    const clear = wrapper
      .findAll(".transfer-list-head button")
      .find((button) => button.text() === "全不选")!;
    await clear.trigger("click");

    expect(primaryButton(wrapper).attributes("disabled")).toBeDefined();
    expect(wrapper.find(".transfer-count").text()).toContain("0/3");
  });
});
