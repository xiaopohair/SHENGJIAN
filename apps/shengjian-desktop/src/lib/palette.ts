import type { Component } from "vue";

export interface PaletteCommand {
  id: string;
  label: string;
  group: string;
  icon?: Component;
  keywords?: string;
  shortcut?: string;
}
