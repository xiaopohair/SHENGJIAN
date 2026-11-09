import { beforeEach, describe, expect, it, vi } from "vitest";

const { invoke } = vi.hoisted(() => ({ invoke: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke }));

describe("finishStartup", () => {
  beforeEach(() => {
    invoke.mockReset();
    vi.resetModules();
  });

  it("hands control to the host exactly once", async () => {
    invoke.mockResolvedValue(undefined);
    const { finishStartup } = await import("./startup");

    await finishStartup();
    await finishStartup();

    expect(invoke).toHaveBeenCalledTimes(1);
    expect(invoke).toHaveBeenCalledWith("finish_startup");
  });

  it("keeps booting when the host command is unavailable", async () => {
    invoke.mockRejectedValue(new Error("not running inside Tauri"));
    const { finishStartup } = await import("./startup");

    await expect(finishStartup()).resolves.toBeUndefined();
  });
});
