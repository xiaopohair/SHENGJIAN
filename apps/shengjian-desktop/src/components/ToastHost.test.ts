/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { setLanguagePreference } from "../lib/i18n";

setLanguagePreference("zh-Hans");

async function mountHost() {
  vi.resetModules();
  const toast = await import("../lib/toast");
  const { default: ToastHost } = await import("./ToastHost.vue");
  const wrapper = mount(ToastHost, { attachTo: document.body });
  return { toast, wrapper };
}

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("ToastHost", () => {
  it("renders message, kind class and role per kind", async () => {
    const { toast, wrapper } = await mountHost();
    toast.pushToast("诊断包已导出", "success");
    toast.pushToast("没有完成安装", "warning");
    await wrapper.vm.$nextTick();

    const nodes = wrapper.findAll(".toast");
    expect(nodes).toHaveLength(2);
    expect(nodes[0]?.text()).toContain("诊断包已导出");
    expect(nodes[0]?.classes()).toContain("success");
    // warning / error 用 role=alert，其余 status（无障碍：只读屏幕的人要知道严重程度）。
    expect(nodes[1]?.attributes("role")).toBe("alert");
    expect(nodes[0]?.attributes("role")).toBe("status");
    wrapper.unmount();
  });

  it("runs the action callback and dismisses on click", async () => {
    const { toast, wrapper } = await mountHost();
    const onAction = vi.fn();
    toast.pushToast("发现新版本 0.8.0", "accent", {
      actionLabel: "查看",
      onAction,
      durationMs: 20000,
    });
    await wrapper.vm.$nextTick();

    await wrapper.get(".toast-action").trigger("click");
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(wrapper.findAll(".toast")).toHaveLength(0);
    wrapper.unmount();
  });

  it("runs the close callback and dismisses on the close button", async () => {
    const { toast, wrapper } = await mountHost();
    const onClose = vi.fn();
    toast.pushToast("已保存", "success", { onClose });
    await wrapper.vm.$nextTick();

    await wrapper.get(".toast-close").trigger("click");
    expect(onClose).toHaveBeenCalledTimes(1);
    expect(wrapper.findAll(".toast")).toHaveLength(0);
    wrapper.unmount();
  });
});
