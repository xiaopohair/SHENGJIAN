/** @vitest-environment jsdom */
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

/**
 * Toast 模块有模块级状态（当前列表、计时器、去重窗口），每个用例用
 * `vi.resetModules()` + 动态 import 拿一份干净实例。
 */
async function loadToast() {
  vi.resetModules();
  return import("./toast");
}

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(new Date("2026-11-14T12:00:00Z"));
});

afterEach(() => {
  vi.useRealTimers();
});

describe("pushToast", () => {
  it("pushes a toast with the requested kind and default duration", async () => {
    const { pushToast, toasts } = await loadToast();
    pushToast("已保存", "success");
    expect(toasts.value).toHaveLength(1);
    expect(toasts.value[0]).toMatchObject({ message: "已保存", kind: "success" });

    vi.advanceTimersByTime(4400);
    expect(toasts.value).toHaveLength(1);
    vi.advanceTimersByTime(200);
    expect(toasts.value).toHaveLength(0);
  });

  it("keeps the action callbacks and custom duration", async () => {
    const { pushToast, toasts } = await loadToast();
    const onAction = vi.fn();
    pushToast("发现新版本 v9", "accent", {
      actionLabel: "查看",
      onAction,
      durationMs: 20000,
    });
    expect(toasts.value[0]).toMatchObject({ actionLabel: "查看" });
    vi.advanceTimersByTime(19900);
    expect(toasts.value).toHaveLength(1);
    vi.advanceTimersByTime(200);
    expect(toasts.value).toHaveLength(0);
    expect(toasts.value[0]?.onAction).toBeUndefined();
    void onAction;
  });
});

describe("dedupe", () => {
  it("collapses the same message+kind inside the 5s window", async () => {
    const { pushToast, toasts } = await loadToast();
    pushToast("无法加载应用列表", "error");
    vi.advanceTimersByTime(1000);
    pushToast("无法加载应用列表", "error");
    expect(toasts.value).toHaveLength(1);
  });

  it("treats different kinds as different messages", async () => {
    const { pushToast, toasts } = await loadToast();
    pushToast("遥控器电量低", "warning");
    pushToast("遥控器电量低", "error");
    expect(toasts.value).toHaveLength(2);
  });

  it("allows the same message again after the window", async () => {
    const { pushToast, toasts } = await loadToast();
    pushToast("无法解析应用目标", "error");
    vi.advanceTimersByTime(5200);
    pushToast("无法解析应用目标", "error");
    expect(toasts.value).toHaveLength(1); // 第一条已到期消失，新的一条在
    expect(toasts.value[0]?.message).toBe("无法解析应用目标");
  });
});

describe("stack limit", () => {
  it("keeps at most three toasts and drops the oldest", async () => {
    const { pushToast, toasts } = await loadToast();
    pushToast("一", "info");
    pushToast("二", "info");
    pushToast("三", "info");
    pushToast("四", "info");
    expect(toasts.value.map((toast) => toast.message)).toEqual(["二", "三", "四"]);
  });
});

describe("dismissToast", () => {
  it("removes the toast and cancels its timer", async () => {
    const { pushToast, dismissToast, toasts } = await loadToast();
    pushToast("导出完成", "success");
    const id = toasts.value[0]!.id;
    dismissToast(id);
    expect(toasts.value).toHaveLength(0);
    // 计时器被清掉：再推进时间也不会出现幽灵条目。
    vi.advanceTimersByTime(10000);
    expect(toasts.value).toHaveLength(0);
  });
});
