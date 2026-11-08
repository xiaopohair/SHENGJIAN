import { execFileSync } from "node:child_process";

export default function globalTeardown() {
  try {
    execFileSync("taskkill", ["/IM", "shengjian-desktop.exe", "/F"], {
      stdio: "ignore",
    });
  } catch {
    // The desktop process may already be gone; nothing to clean up.
  }
}
