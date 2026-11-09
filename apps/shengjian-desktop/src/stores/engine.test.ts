import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

vi.mock("../lib/api", () => ({
  getEngineEventCursor: vi.fn(async () => 0),
  getEngineSnapshot: vi.fn(async () => null),
  scanRemotes: vi.fn(async () => []),
  connectRemote: vi.fn(),
  disconnectRemote: vi.fn(),
  errorMessage: (error: unknown) => String(error),
}));

import { useEngineStore } from "./engine";
import { connectRemote, scanRemotes } from "../lib/api";
import type { EngineSnapshot, EventEnvelope } from "../lib/types";

function snapshot(): EngineSnapshot {
  return {
    connection: { phase: "ready", remote: null, generation: 1, lastError: null },
    voice: { state: "idle", generation: 1, lastError: null },
    audio: {
      state: "ready",
      endpoint: null,
      format: null,
      gainDb: 8,
      generation: 0,
      lastError: null,
    },
    capabilities: {},
  };
}

function voiceEvent(seq: number, state: string): EventEnvelope {
  return {
    seq,
    generation: 1,
    event: {
      type: "voice.changed",
      snapshot: { state, generation: 1, lastError: null },
    },
  };
}

function buttonEvent(seq: number, event: EventEnvelope["event"]): EventEnvelope {
  return { seq, generation: 2, event };
}

describe("engine store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("records voice history and ignores older sequence numbers", () => {
    const store = useEngineStore();
    store.snapshot = snapshot();

    store.applyEvent(voiceEvent(8, "streaming"));
    store.applyEvent(voiceEvent(10, "draining"));
    store.applyEvent(voiceEvent(12, "idle"));
    store.applyEvent(voiceEvent(8, "streaming"));

    expect(store.voiceHistory.map((event) => event.state)).toEqual([
      "streaming",
      "draining",
      "idle",
    ]);
    expect(store.cursor).toBe(12);
    expect(store.voiceState).toBe("idle");
  });

  it("records button edges and gestures", () => {
    const store = useEngineStore();
    store.snapshot = snapshot();

    store.applyEvent(
      buttonEvent(13, {
        type: "button.edge",
        edge: { button: "ok", pressed: true, atMs: 1 },
      }),
    );
    store.applyEvent(
      buttonEvent(14, {
        type: "button.gesture",
        gesture: { button: "ok", trigger: "single", atMs: 2 },
      }),
    );

    expect(store.buttonHistory).toEqual([
      { seq: 13, type: "edge", button: "ok", pressed: true },
      { seq: 14, type: "gesture", button: "ok", trigger: "single" },
    ]);
  });

  it("backfills the discovered remote model from the connected snapshot", async () => {
    const store = useEngineStore();
    store.snapshot = snapshot();
    store.remotes = [
      {
        remoteId: "remote-1",
        name: "小米蓝牙语音遥控器",
        model: null,
        deviceId: null,
        isSupported: true,
        token: null,
      },
    ];
    vi.mocked(connectRemote).mockResolvedValue({
      phase: "ready",
      remote: {
        remoteId: "remote-1",
        name: "小米蓝牙语音遥控器",
        model: "RC003",
        deviceId: "xiaomi-rc003",
        isSupported: true,
        token: "token-1",
      },
      generation: 1,
      lastError: null,
    });

    await store.connect("remote-1");

    expect(store.remotes[0]?.model).toBe("RC003");
    expect(store.remotes[0]?.deviceId).toBe("xiaomi-rc003");
    expect(store.remotes[0]?.token).toBe("token-1");
  });

  it("backfills the discovered remote model from a connection event", () => {
    const store = useEngineStore();
    store.snapshot = snapshot();
    store.remotes = [
      {
        remoteId: "remote-1",
        name: "小米蓝牙语音遥控器",
        model: null,
        deviceId: null,
        isSupported: true,
        token: null,
      },
    ];

    store.applyEvent({
      seq: 1,
      generation: 1,
      event: {
        type: "connection.changed",
        snapshot: {
          phase: "ready",
          remote: {
            remoteId: "remote-1",
            name: "小米蓝牙语音遥控器",
            model: "RC003",
            deviceId: "xiaomi-rc003",
            isSupported: true,
            token: "token-1",
          },
          generation: 1,
          lastError: null,
        },
      },
    });

    expect(store.remotes[0]?.model).toBe("RC003");
  });

  it("keeps the connected model when the list is re-scanned", async () => {
    const store = useEngineStore();
    store.snapshot = {
      ...snapshot(),
      connection: {
        phase: "ready",
        remote: {
          remoteId: "remote-1",
          name: "小米蓝牙语音遥控器",
          model: "RC003",
          deviceId: "xiaomi-rc003",
          isSupported: true,
          token: "token-1",
        },
        generation: 1,
        lastError: null,
      },
    };
    vi.mocked(scanRemotes).mockResolvedValue([
      {
        remoteId: "remote-1",
        name: "小米蓝牙语音遥控器",
        model: null,
        deviceId: null,
        isSupported: true,
        token: "token-1",
      },
    ]);

    await store.scan();

    expect(store.remotes[0]?.model).toBe("RC003");
  });
});
