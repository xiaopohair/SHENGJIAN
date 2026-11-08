import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  timeout: 60_000,
  expect: { timeout: 10_000 },
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  use: {
    trace: "retain-on-failure",
  },
  webServer: {
    command: "powershell -NoProfile -ExecutionPolicy Bypass -File scripts/e2e-app.ps1",
    url: "http://127.0.0.1:9223/json/version",
    reuseExistingServer: true,
    timeout: 120_000,
    cwd: process.cwd(),
  },
  globalTeardown: "./e2e/global-teardown.ts",
});
