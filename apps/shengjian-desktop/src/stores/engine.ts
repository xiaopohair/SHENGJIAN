import { defineStore } from "pinia";
import {
  connectRemote,
  disconnectRemote,
  errorMessage,
  getEngineEventCursor,
  getEngineSnapshot,
  scanRemotes,
} from "../lib/api";
import type { EngineSnapshot, EventEnvelope, RemoteCandidate } from "../lib/types";

interface VoiceEvent {
  seq: number;
  state: string;
}

interface ButtonEvent {
  seq: number;
  type: "edge" | "gesture";
  button: string;
  pressed?: boolean;
  trigger?: string;
}

/**
 * A scan can only infer the model from the Bluetooth name, which is generic for
 * some remotes. Once connected, the connection snapshot carries the model read
 * from GATT; backfill it into the matching discovered row so the list and the
 * status card agree.
 */
function mergeConnectedRemote(
  remotes: RemoteCandidate[],
  connected: RemoteCandidate | null,
): RemoteCandidate[] {
  if (!connected) return remotes;
  return remotes.map((remote) =>
    remote.remoteId === connected.remoteId
      ? {
          ...remote,
          name: connected.name || remote.name,
          model: remote.model ?? connected.model,
          deviceId: remote.deviceId ?? connected.deviceId,
          token: remote.token ?? connected.token,
          isSupported: remote.isSupported || connected.isSupported,
        }
      : remote,
  );
}

interface EngineState {
  snapshot: EngineSnapshot | null;
  cursor: number;
  remotes: RemoteCandidate[];
  scanning: boolean;
  connecting: boolean;
  error: string | null;
  voiceHistory: VoiceEvent[];
  buttonHistory: ButtonEvent[];
}

export const useEngineStore = defineStore("engine", {
  state: (): EngineState => ({
    snapshot: null,
    cursor: 0,
    remotes: [],
    scanning: false,
    connecting: false,
    error: null,
    voiceHistory: [],
    buttonHistory: [],
  }),
  getters: {
    phase: (state) => state.snapshot?.connection.phase ?? "idle",
    voiceState: (state) => state.snapshot?.voice.state ?? "idle",
    audioState: (state) => state.snapshot?.audio.state ?? "unconfigured",
    isConnected: (state) => state.snapshot?.connection.phase === "ready",
  },
  actions: {
    async hydrate() {
      this.error = null;
      try {
        this.cursor = await getEngineEventCursor();
        this.snapshot = await getEngineSnapshot();
      } catch (error) {
        this.error = errorMessage(error);
      }
    },
    async refresh() {
      try {
        this.snapshot = await getEngineSnapshot();
      } catch (error) {
        this.error = errorMessage(error);
      }
    },
    async scan() {
      this.scanning = true;
      this.error = null;
      try {
        const remotes = await scanRemotes();
        this.remotes = mergeConnectedRemote(
          remotes,
          this.snapshot?.connection.remote ?? null,
        );
      } catch (error) {
        this.error = errorMessage(error);
      } finally {
        this.scanning = false;
      }
    },
    async connect(remoteId: string) {
      this.connecting = true;
      this.error = null;
      try {
        const connection = await connectRemote(remoteId);
        const snapshot = this.snapshot ?? (await getEngineSnapshot());
        this.snapshot = { ...snapshot, connection };
        this.remotes = mergeConnectedRemote(this.remotes, connection.remote);
      } catch (error) {
        this.error = errorMessage(error);
      } finally {
        this.connecting = false;
      }
    },
    async disconnect() {
      this.connecting = true;
      this.error = null;
      try {
        const connection = await disconnectRemote();
        const snapshot = this.snapshot ?? (await getEngineSnapshot());
        this.snapshot = { ...snapshot, connection };
      } catch (error) {
        this.error = errorMessage(error);
      } finally {
        this.connecting = false;
      }
    },
    applyEvent(envelope: EventEnvelope) {
      if (envelope.seq <= this.cursor) return;
      this.cursor = envelope.seq;
      if (!this.snapshot) return;

      if (envelope.event.type === "connection.changed" && envelope.event.snapshot) {
        const connection = envelope.event.snapshot as EngineSnapshot["connection"];
        this.snapshot = {
          ...this.snapshot,
          connection,
        };
        this.remotes = mergeConnectedRemote(this.remotes, connection.remote);
      } else if (envelope.event.type === "voice.changed" && envelope.event.snapshot) {
        const voice = envelope.event.snapshot as EngineSnapshot["voice"];
        this.snapshot = {
          ...this.snapshot,
          voice,
        };
        this.voiceHistory = [
          ...this.voiceHistory.slice(-11),
          { seq: envelope.seq, state: voice.state },
        ];
      } else if (envelope.event.type === "audio.changed" && envelope.event.snapshot) {
        this.snapshot = {
          ...this.snapshot,
          audio: envelope.event.snapshot as EngineSnapshot["audio"],
        };
      } else if (envelope.event.type === "button.edge" && envelope.event.edge) {
        const edge = envelope.event.edge as {
          button: string;
          pressed: boolean;
          atMs: number;
        };
        this.buttonHistory = [
          ...this.buttonHistory.slice(-19),
          { seq: envelope.seq, type: "edge", button: edge.button, pressed: edge.pressed },
        ];
      } else if (envelope.event.type === "button.gesture" && envelope.event.gesture) {
        const gesture = envelope.event.gesture as {
          button: string;
          trigger: string;
          atMs: number;
        };
        this.buttonHistory = [
          ...this.buttonHistory.slice(-19),
          { seq: envelope.seq, type: "gesture", button: gesture.button, trigger: gesture.trigger },
        ];
      } else if (envelope.event.type === "engine.error" && envelope.event.error) {
        this.error = envelope.event.error.code;
      }
    },
  },
});
