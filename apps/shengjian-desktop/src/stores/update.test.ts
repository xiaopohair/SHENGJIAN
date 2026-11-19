import { beforeEach, describe, expect, it, vi } from "vitest";
import { createPinia, setActivePinia } from "pinia";

vi.mock("@tauri-apps/api/app", () => ({
  getVersion: vi.fn(async () => "0.9.5"),
}));

vi.mock("../lib/api", () => ({
  downloadUpdateRelease: vi.fn(),
  errorMessage: (error: unknown) => String(error),
  getSettings: vi.fn(),
  getUpdateSupport: vi.fn(),
  installDownloadedUpdate: vi.fn(),
  saveSettings: vi.fn(async () => undefined),
  setTrayUpdate: vi.fn(async () => undefined),
}));

vi.mock("../lib/toast", () => ({
  pushToast: vi.fn(),
}));

vi.mock("../lib/update", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../lib/update")>();
  return { ...actual, checkForUpdates: vi.fn() };
});

import * as api from "../lib/api";
import { checkForUpdates } from "../lib/update";
import { useUpdateStore } from "./update";
import type { DownloadedUpdate, SettingsDocument } from "../lib/types";
import type { UpdateCheckResult, UpdateRelease } from "../lib/update";

const release: UpdateRelease = {
  version: "0.10.0",
  tagName: "v0.10.0",
  name: "0.10.0",
  url: "https://github.com/xiaopohair/SHENGJIAN/releases/tag/v0.10.0",
  prerelease: false,
  publishedAt: "2026-12-01T00:00:00Z",
};

const downloaded: DownloadedUpdate = {
  tag: "v0.10.0",
  version: "0.10.0",
  path: "C:\\Users\\me\\AppData\\Local\\Shengjian\\Updates\\shengjian_0.10.0_x64-setup.exe",
  sizeBytes: 1024,
  sha256: "a".repeat(64),
  signed: false,
};

function settings(overrides: Record<string, boolean> = {}): SettingsDocument {
  return {
    schemaVersion: 8,
    global: {
      gain_db: 0,
      check_prerelease_updates: false,
      check_updates_automatically: true,
      download_updates_automatically: true,
      ...overrides,
    },
    devices: {},
  };
}

function updateResult(available = true): UpdateCheckResult {
  return {
    currentVersion: "0.9.5",
    latest: available ? release : null,
    updateAvailable: available,
  };
}

function installMocks() {
  vi.mocked(api.getSettings).mockResolvedValue(settings());
  vi.mocked(api.getUpdateSupport).mockResolvedValue({
    supported: true,
    kind: "installed",
    reason: null,
  });
  vi.mocked(api.setTrayUpdate).mockResolvedValue(undefined);
  vi.mocked(api.downloadUpdateRelease).mockResolvedValue(downloaded);
  vi.mocked(api.installDownloadedUpdate).mockResolvedValue(undefined);
  vi.mocked(checkForUpdates).mockResolvedValue(updateResult());
}

describe("update store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    vi.clearAllMocks();
    installMocks();
  });

  it("hydrates preferences and detects an installable build", async () => {
    const store = useUpdateStore();
    await store.hydrate();

    expect(store.currentVersion).toBe("0.9.5");
    expect(store.autoCheckEnabled).toBe(true);
    expect(store.autoDownloadEnabled).toBe(true);
    expect(store.downloadSupported).toBe(true);
    expect(store.updateKind).toBe("installed");
  });

  it("does not touch the network when automatic checks are disabled", async () => {
    vi.mocked(api.getSettings).mockResolvedValue(settings({ check_updates_automatically: false }));
    const store = useUpdateStore();
    await store.hydrate();
    await store.autoCheck();

    expect(checkForUpdates).not.toHaveBeenCalled();
  });

  it("checks, then downloads automatically and ends in ready", async () => {
    const store = useUpdateStore();
    await store.hydrate();
    await store.check(true);

    await vi.waitFor(() => expect(api.downloadUpdateRelease).toHaveBeenCalledWith("v0.10.0"));
    await vi.waitFor(() => expect(store.status).toBe("ready"));
    expect(store.ready?.tag).toBe("v0.10.0");
    expect(store.downloadPercent).toBe(100);
  });

  it("reports download progress for the matching tag only", async () => {
    const store = useUpdateStore();
    await store.hydrate();
    store.latest = release;
    store.available = true;
    store.status = "available";

    let finish: (value: DownloadedUpdate) => void = () => undefined;
    vi.mocked(api.downloadUpdateRelease).mockReturnValue(
      new Promise<DownloadedUpdate>((resolve) => {
        finish = resolve;
      }),
    );
    const pending = store.download(false);
    store.applyProgress({ tag: "v9.9.9", downloaded: 1, total: 2 });
    expect(store.downloadPercent).toBe(0);
    store.applyProgress({ tag: "v0.10.0", downloaded: 512, total: 1024 });
    expect(store.downloadPercent).toBe(50);

    finish(downloaded);
    await pending;
    expect(store.status).toBe("ready");
  });

  it("installs only the downloaded update", async () => {
    const store = useUpdateStore();
    await store.hydrate();
    store.ready = downloaded;
    store.downloadSupported = true;

    await store.install();
    expect(api.installDownloadedUpdate).toHaveBeenCalledTimes(1);
    expect(store.status).toBe("installing");
  });

  it("keeps the available state when a manual download fails", async () => {
    vi.mocked(api.downloadUpdateRelease).mockRejectedValue(new Error("network down"));
    const store = useUpdateStore();
    await store.hydrate();
    store.latest = release;
    store.available = true;
    store.status = "available";

    await store.download(true);
    expect(store.status).toBe("available");
    expect(store.error).toBe("Error: network down");
    expect(store.ready).toBeNull();
  });

  it("does not auto-download a version the user skipped", async () => {
    const store = useUpdateStore();
    await store.hydrate();
    store.latest = release;
    store.available = true;
    store.dismissedVersion = release.tagName;

    store.maybeAutoDownload();
    expect(api.downloadUpdateRelease).not.toHaveBeenCalled();
  });

  it("persists automatic-update preferences through settings", async () => {
    const store = useUpdateStore();
    await store.hydrate();
    await store.setAutoDownloadEnabled(false);

    expect(api.saveSettings).toHaveBeenCalledTimes(1);
    const saved = vi.mocked(api.saveSettings).mock.calls[0]![0];
    expect(saved.global.download_updates_automatically).toBe(false);
    expect(saved.global.check_updates_automatically).toBe(true);
  });

  it("drops a downloaded package when a newer release appears", async () => {
    vi.mocked(api.getSettings).mockResolvedValue(
      settings({ download_updates_automatically: false }),
    );
    vi.mocked(checkForUpdates).mockResolvedValue({
      currentVersion: "0.9.5",
      latest: { ...release, version: "0.11.0", tagName: "v0.11.0" },
      updateAvailable: true,
    });
    const store = useUpdateStore();
    await store.hydrate();
    store.latest = release;
    store.available = true;
    store.ready = downloaded;
    store.status = "ready";

    await store.check(true);
    expect(store.ready).toBeNull();
    expect(store.status).toBe("available");
  });

  it("clears ready state once the running version catches up", async () => {
    const store = useUpdateStore();
    await store.hydrate();
    store.latest = release;
    store.ready = downloaded;
    store.available = true;
    store.status = "ready";

    store.currentVersion = "0.10.0";
    store.recomputeAvailability();
    expect(store.ready).toBeNull();
    expect(store.available).toBe(false);
    expect(store.status).toBe("current");
  });
});
