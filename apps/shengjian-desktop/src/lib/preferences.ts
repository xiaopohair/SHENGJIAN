import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

export const ONBOARDING_KEY = "shengjian-onboarding-complete";
export const RESTART_ONBOARDING_EVENT = "shengjian:restart-onboarding";
export const SHOW_UPDATE_EVENT = "shengjian:show-update";
export const ACCENT_KEY = "shengjian-accent";
export const SIDEBAR_KEY = "shengjian-sidebar-collapsed";

export type ThemePreference = "system" | "light" | "dark";
export type AccentPreference = "default" | "blue" | "purple" | "orange";

const ACCENTS = new Set<AccentPreference>(["default", "blue", "purple", "orange"]);

let currentThemePreference = "system";

export function getThemePreference(): string {
  return currentThemePreference;
}

export function resolveTheme(preference: string): "light" | "dark" {
  if (preference === "light" || preference === "dark") return preference;
  return window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light";
}

/**
 * Accent colors live in tokens.css as `:root[data-accent]` selectors, keyed by
 * the resolved theme. Setting the dataset keeps the light/dark contrast pairs
 * in CSS instead of patching two variables from JavaScript.
 */
export function applyAccent(preference: string) {
  const root = document.documentElement;
  const accent = ACCENTS.has(preference as AccentPreference)
    ? (preference as AccentPreference)
    : "default";
  root.dataset.accent = accent;
  // Clear the inline variables written by earlier builds.
  root.style.removeProperty("--accent");
  root.style.removeProperty("--accent-soft");
}

export function applyTheme(preference: string) {
  currentThemePreference = preference;
  const resolved = resolveTheme(preference);
  document.documentElement.dataset.theme = resolved;
  void syncWindowTheme(preference === "system" ? null : resolved);
}

async function syncWindowTheme(theme: "light" | "dark" | null) {
  try {
    await getCurrentWindow().setTheme(theme);
  } catch {
    // Older hosts or missing capability: keep the webview theme only.
  }
  try {
    // Tray icons use the window theme; refresh them without waiting for the
    // battery-status polling loop.
    await invoke("sync_tray_icons");
  } catch {
    // Browser-only dev or an older host.
  }
}
