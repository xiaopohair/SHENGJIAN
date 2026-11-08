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
});
