import { defineStore } from "pinia";
import { getDeviceStatus } from "../lib/api";
import type { DeviceStatus } from "../lib/types";

interface StatusState {
  batteryLevel: number | null;
  deviceStatus: DeviceStatus | null;
  voiceInputTool: string;
}

export const useStatusStore = defineStore("status", {
  state: (): StatusState => ({
    batteryLevel: null,
    deviceStatus: null,
    voiceInputTool: "",
  }),
  actions: {
    async refreshDeviceStatus() {
      try {
        const status = await getDeviceStatus();
        this.deviceStatus = status;
        this.batteryLevel = status?.batteryLevel ?? null;
      } catch {
        this.deviceStatus = null;
        this.batteryLevel = null;
      }
    },
    setVoiceInputTool(value: string | null | undefined) {
      this.voiceInputTool = String(value ?? "");
    },
    clearDeviceStatus() {
      this.deviceStatus = null;
      this.batteryLevel = null;
    },
  },
});
