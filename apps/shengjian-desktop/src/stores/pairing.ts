import { defineStore } from "pinia";

export type PairingModel = "RC001" | "RC003" | "ARN9";

export function normalizePairingModel(model?: string | null): PairingModel {
  if (model === "RC001") return "RC001";
  if (model === "ARN9") return "ARN9";
  return "RC003";
}

export const usePairingStore = defineStore("pairing", {
  state: (): { open: boolean; model: PairingModel } => ({
    open: false,
    model: "RC003",
  }),

  actions: {
    show(model?: string | null) {
      this.model = normalizePairingModel(model);
      this.open = true;
    },
    close() {
      this.open = false;
    },
  },
});
