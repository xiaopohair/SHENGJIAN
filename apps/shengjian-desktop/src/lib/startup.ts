import { invoke } from "@tauri-apps/api/core";

let finished = false;

/**
 * Hand control from the splash window to the main window. The boot sequence
 * calls this only after settings, theme and accent are applied, so the shell is
 * never revealed half-themed. Failures are ignored in browser-only dev.
 */
export async function finishStartup(): Promise<void> {
  if (finished) return;
  finished = true;
  try {
    await invoke("finish_startup");
  } catch {
    // Not running inside Tauri, or the host predates the command.
  }
}
