import { beforeEach, describe, expect, it } from "vitest";
import {
  applyRememberedModels,
  lastConnectedAt,
  loadDeviceRecords,
  markDeviceConnected,
  rememberDeviceModel,
} from "./deviceMemory";

function fakeStorage(): Storage {
  const data = new Map<string, string>();
  return {
    get length() {
      return data.size;
    },
    clear: () => data.clear(),
    getItem: (key: string) => data.get(key) ?? null,
    key: (index: number) => [...data.keys()][index] ?? null,
    removeItem: (key: string) => void data.delete(key),
    setItem: (key: string, value: string) => void data.set(key, value),
  } as Storage;
}

describe("device record memory", () => {
  beforeEach(() => {
    Object.defineProperty(globalThis, "localStorage", {
      value: fakeStorage(),
      configurable: true,
      writable: true,
    });
  });

  it("records the model and connection time per device token", () => {
    markDeviceConnected("a1b2c3d4", "RC003", 1_700_000_000_000);
    markDeviceConnected("0fab7862", "RC001", 1_700_000_100_000);
    expect(loadDeviceRecords()).toEqual({
      a1b2c3d4: { model: "RC003", lastConnectedAt: 1_700_000_000_000 },
      "0fab7862": { model: "RC001", lastConnectedAt: 1_700_000_100_000 },
    });
    expect(lastConnectedAt(loadDeviceRecords(), "a1b2c3d4")).toBe(1_700_000_000_000);
    expect(lastConnectedAt(loadDeviceRecords(), "missing")).toBeNull();
  });

  it("keeps a previously known model when a connection reports no model", () => {
    rememberDeviceModel("a1b2c3d4", "RC003");
    markDeviceConnected("a1b2c3d4", null, 1_700_000_000_000);
    expect(loadDeviceRecords()).toEqual({
      a1b2c3d4: { model: "RC003", lastConnectedAt: 1_700_000_000_000 },
    });
  });

  it("migrates the legacy token-to-model string format", () => {
    localStorage.setItem(
      "shengjian.device-models",
      JSON.stringify({ a1b2c3d4: "ARN9", broken: "", "0fab7862": 42 }),
    );
    expect(loadDeviceRecords()).toEqual({ a1b2c3d4: { model: "ARN9" } });
  });

  it("survives corrupted storage by falling back to an empty map", () => {
    localStorage.setItem("shengjian.device-models", "{not json");
    expect(loadDeviceRecords()).toEqual({});
    localStorage.setItem("shengjian.device-models", JSON.stringify(["a1b2c3d4"]));
    expect(loadDeviceRecords()).toEqual({});
  });

  it("backfills only rows whose model is missing", () => {
    const remotes = [
      { remoteId: "a1b2c3d4", model: null },
      { remoteId: "0fab7862", model: "ARN9" },
      { remoteId: "ffffffff", model: null },
    ];
    expect(
      applyRememberedModels(remotes, {
        a1b2c3d4: { model: "RC003" },
        "0fab7862": { model: "RC001" },
      }),
    ).toEqual([
      { remoteId: "a1b2c3d4", model: "RC003" },
      { remoteId: "0fab7862", model: "ARN9" },
      { remoteId: "ffffffff", model: null },
    ]);
  });
});
