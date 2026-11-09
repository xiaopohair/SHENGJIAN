import { defineStore } from "pinia";
import { getVersion } from "@tauri-apps/api/app";
import { errorMessage, getSettings, saveSettings, setTrayUpdate } from "../lib/api";
import { checkForUpdates, compareVersions, type UpdateRelease } from "../lib/update";
import { pushToast } from "../lib/toast";
import { tr } from "../lib/i18n";
import { SHOW_UPDATE_EVENT } from "../lib/preferences";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "unavailable"
  | "error";

interface UpdateCache {
  lastCheckedAt: number | null;
  latest: UpdateRelease | null;
  available: boolean;
  dismissedVersion: string | null;
}

interface UpdateState extends UpdateCache {
  currentVersion: string;
  includePrereleases: boolean;
  checking: boolean;
  status: UpdateStatus;
  error: string | null;
  hydrated: boolean;
  notifiedVersion: string | null;
}

const CACHE_KEY = "shengjian-update-cache-v1";
const CHECK_INTERVAL_MS = 24 * 60 * 60 * 1000;

function safeStorage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

function loadCache(): UpdateCache | null {
  const storage = safeStorage();
  if (!storage) return null;
  try {
    const raw = storage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<UpdateCache>;
    return {
      lastCheckedAt:
        typeof parsed.lastCheckedAt === "number" ? parsed.lastCheckedAt : null,
      latest: parsed.latest ?? null,
      available: parsed.available === true,
      dismissedVersion:
        typeof parsed.dismissedVersion === "string" ? parsed.dismissedVersion : null,
    };
  } catch {
    return null;
  }
}

export const useUpdateStore = defineStore("update", {
  state: (): UpdateState => ({
    currentVersion: "",
    includePrereleases: false,
    checking: false,
    status: "idle",
    error: null,
    hydrated: false,
    notifiedVersion: null,
    lastCheckedAt: null,
    latest: null,
    available: false,
    dismissedVersion: null,
  }),

  getters: {
    hasUpdate(state): boolean {
      return (
        state.available &&
        state.latest !== null &&
        state.dismissedVersion !== state.latest.tagName
      );
    },
  },

  actions: {
    async hydrate() {
      if (this.hydrated) return;
      this.hydrated = true;
      const cache = loadCache();
      if (cache) {
        this.lastCheckedAt = cache.lastCheckedAt;
        this.latest = cache.latest;
        this.available = cache.available;
        this.dismissedVersion = cache.dismissedVersion;
        this.recomputeAvailability();
        if (this.latest) {
          this.status = this.available ? "available" : "current";
        }
      }
      if (!this.currentVersion) {
        this.currentVersion = await getVersion().catch(() => "");
      }
      await this.loadIncludePrereleases();
      this.recomputeAvailability();
      await this.syncTray();
    },

    setCurrentVersion(version: string) {
      if (!version || this.currentVersion === version) return;
      this.currentVersion = version;
      this.recomputeAvailability();
      if (this.hydrated) this.saveCache();
    },

    recomputeAvailability() {
      if (!this.latest || !this.currentVersion) return;
      this.available = compareVersions(this.latest.version, this.currentVersion) > 0;
      if (!this.available && this.status === "available") {
        this.status = "current";
      }
    },

    saveCache() {
      const storage = safeStorage();
      if (!storage) return;
      const cache: UpdateCache = {
        lastCheckedAt: this.lastCheckedAt,
        latest: this.latest,
        available: this.available,
        dismissedVersion: this.dismissedVersion,
      };
      try {
        storage.setItem(CACHE_KEY, JSON.stringify(cache));
      } catch {
        // Cache is best-effort; update checks still work without storage.
      }
    },

    async loadIncludePrereleases() {
      try {
        const settings = await getSettings();
        this.includePrereleases = settings.global.check_prerelease_updates ?? false;
      } catch {
        // Keep the current preference if settings are temporarily unavailable.
      }
    },

    async setIncludePrereleases(value: boolean) {
      this.includePrereleases = value;
      try {
        const settings = await getSettings();
        await saveSettings({
          ...settings,
          global: {
            ...settings.global,
            check_prerelease_updates: value,
          },
        });
      } catch (cause) {
        this.error = errorMessage(cause);
      }
    },

    async check(manual: boolean) {
      if (this.checking) return;
      const previousStatus = this.status;
      this.checking = true;
      this.status = "checking";
      this.error = null;
      try {
        if (!this.currentVersion) {
          this.currentVersion = await getVersion().catch(() => "");
        }
        await this.loadIncludePrereleases();
        const result = await checkForUpdates(
          this.currentVersion,
          this.includePrereleases,
        );
        this.latest = result.latest;
        this.available = result.updateAvailable;
        this.lastCheckedAt = Date.now();
        this.status = !result.latest
          ? "unavailable"
          : result.updateAvailable
            ? "available"
            : "current";
        this.saveCache();
        await this.syncTray();
        if (!manual && this.hasUpdate) {
          this.notify();
        }
      } catch (cause) {
        if (manual) {
          this.error = errorMessage(cause);
          this.status = "error";
        } else {
          this.status = previousStatus;
        }
      } finally {
        this.checking = false;
      }
    },

    async autoCheck() {
      await this.hydrate();
      const fresh =
        this.lastCheckedAt !== null &&
        Date.now() - this.lastCheckedAt < CHECK_INTERVAL_MS;
      if (fresh) {
        await this.syncTray();
        if (this.hasUpdate) this.notify();
        return;
      }
      await this.check(false);
    },

    notify() {
      if (!this.hasUpdate || !this.latest) return;
      if (this.notifiedVersion === this.latest.tagName) return;
      this.notifiedVersion = this.latest.tagName;
      const version = this.latest.tagName;
      pushToast(`${tr("发现新版本")} ${version}`, "info", {
        actionLabel: tr("查看"),
        durationMs: 12000,
        onAction: () => {
          this.dismiss();
          window.dispatchEvent(new Event(SHOW_UPDATE_EVENT));
        },
        onClose: () => this.dismiss(),
      });
    },

    dismiss() {
      if (!this.latest) return;
      this.dismissedVersion = this.latest.tagName;
      this.saveCache();
      void this.syncTray();
    },

    async syncTray() {
      const version = this.hasUpdate ? (this.latest?.tagName ?? null) : null;
      await setTrayUpdate(version).catch(() => undefined);
    },

    reportError(message: string) {
      this.error = message;
      this.status = "error";
    },
  },
});
