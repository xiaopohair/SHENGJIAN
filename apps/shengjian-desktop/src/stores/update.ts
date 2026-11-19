import { defineStore } from "pinia";
import { getVersion } from "@tauri-apps/api/app";
import {
  downloadUpdateRelease,
  errorMessage,
  getSettings,
  getUpdateSupport,
  installDownloadedUpdate,
  saveSettings,
  setTrayUpdate,
} from "../lib/api";
import {
  checkForUpdates,
  compareVersions,
  updateDownloadPercent,
  type UpdateRelease,
} from "../lib/update";
import { pushToast } from "../lib/toast";
import { tr } from "../lib/i18n";
import { SHOW_UPDATE_EVENT } from "../lib/preferences";
import type { DownloadedUpdate, UpdateDownloadProgress } from "../lib/types";

export type UpdateStatus =
  | "idle"
  | "checking"
  | "current"
  | "available"
  | "downloading"
  | "ready"
  | "installing"
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
  /** 启动后自动检查更新（设置页「软件更新」，默认开启）。 */
  autoCheckEnabled: boolean;
  /** 发现新版本后自动下载（安装仍需确认，默认开启）。 */
  autoDownloadEnabled: boolean;
  checking: boolean;
  status: UpdateStatus;
  error: string | null;
  hydrated: boolean;
  notifiedVersion: string | null;
  /** 后端下载安装包时推送的最近一次进度。 */
  downloadProgress: UpdateDownloadProgress | null;
  /** 已下载并校验通过的安装包；`install()` 只消费这一个。 */
  ready: DownloadedUpdate | null;
  /** 当前构建能否自己升级（安装版支持；便携版/开发版不支持）。 */
  downloadSupported: boolean;
  updateKind: string;
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
      lastCheckedAt: typeof parsed.lastCheckedAt === "number" ? parsed.lastCheckedAt : null,
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
    autoCheckEnabled: true,
    autoDownloadEnabled: true,
    checking: false,
    status: "idle",
    error: null,
    hydrated: false,
    notifiedVersion: null,
    lastCheckedAt: null,
    latest: null,
    available: false,
    dismissedVersion: null,
    downloadProgress: null,
    ready: null,
    downloadSupported: false,
    updateKind: "unknown",
  }),

  getters: {
    hasUpdate(state): boolean {
      return (
        state.available && state.latest !== null && state.dismissedVersion !== state.latest.tagName
      );
    },
    downloading(state): boolean {
      return state.status === "downloading" && state.downloadProgress !== null;
    },
    downloadPercent(state): number {
      if (!state.downloadProgress) return 0;
      return updateDownloadPercent(state.downloadProgress.downloaded, state.downloadProgress.total);
    },
    /** 安装前必须同时满足：有校验过的包 + 这份构建支持自升级。 */
    canInstall(state): boolean {
      return state.ready !== null && state.downloadSupported;
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
      await this.loadPreferences();
      const support = await getUpdateSupport().catch(() => null);
      if (support) {
        this.downloadSupported = support.supported;
        this.updateKind = support.kind;
      }
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
      if (this.ready && this.latest && this.ready.tag !== this.latest.tagName) {
        this.ready = null;
        this.downloadProgress = null;
      }
      if (!this.available) {
        // 已经升级到最新版（或版本号变了）：旧的下载包不再适用。
        this.ready = null;
        this.downloadProgress = null;
        if (["available", "downloading", "ready", "installing"].includes(this.status)) {
          this.status = "current";
        }
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

    async loadPreferences() {
      try {
        const settings = await getSettings();
        this.includePrereleases = settings.global.check_prerelease_updates ?? false;
        this.autoCheckEnabled = settings.global.check_updates_automatically ?? true;
        this.autoDownloadEnabled = settings.global.download_updates_automatically ?? true;
      } catch {
        // Keep the current preferences if settings are temporarily unavailable.
      }
    },

    async setIncludePrereleases(value: boolean) {
      this.includePrereleases = value;
      await this.savePreference("check_prerelease_updates", value);
    },

    async setAutoCheckEnabled(value: boolean) {
      this.autoCheckEnabled = value;
      await this.savePreference("check_updates_automatically", value);
    },

    async setAutoDownloadEnabled(value: boolean) {
      this.autoDownloadEnabled = value;
      await this.savePreference("download_updates_automatically", value);
      if (value) this.maybeAutoDownload();
    },

    async savePreference(
      key:
        | "check_prerelease_updates"
        | "check_updates_automatically"
        | "download_updates_automatically",
      value: boolean,
    ) {
      try {
        const settings = await getSettings();
        await saveSettings({
          ...settings,
          global: {
            ...settings.global,
            [key]: value,
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
        await this.loadPreferences();
        const result = await checkForUpdates(this.currentVersion, this.includePrereleases);
        this.latest = result.latest;
        this.available = result.updateAvailable;
        // 发布了一个更新的版本时，上一版下载好的安装包已经过时。
        if (this.ready && result.latest && this.ready.tag !== result.latest.tagName) {
          this.ready = null;
          this.downloadProgress = null;
        }
        if (!result.updateAvailable) {
          // 已升级到最新版：清掉上一轮留下的安装包状态。
          this.ready = null;
          this.downloadProgress = null;
        }
        this.lastCheckedAt = Date.now();
        this.status = !result.latest
          ? "unavailable"
          : result.updateAvailable
            ? this.ready?.tag === result.latest.tagName
              ? "ready"
              : "available"
            : "current";
        // 手动检查是用户的明确意图：清掉「忽略此版本」，让结果重新可见。
        if (manual && result.updateAvailable) {
          this.dismissedVersion = null;
        }
        this.saveCache();
        await this.syncTray();
        if (!manual && this.hasUpdate) {
          this.notify();
        }
        this.maybeAutoDownload();
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

    /** 自动检查/自动下载的统一入口：设置关闭时一次网络请求都不发。 */
    maybeAutoDownload() {
      if (!this.autoDownloadEnabled || !this.downloadSupported) return;
      if (!this.hasUpdate || this.ready !== null || this.downloadProgress !== null) return;
      if (this.latest && this.dismissedVersion === this.latest.tagName) return;
      void this.download(false);
    },

    async download(manual = true) {
      if (!this.latest || !this.hasUpdate) return;
      if (!this.downloadSupported) {
        const message = tr("当前版本不支持应用内升级，请手动下载安装包");
        this.error = message;
        if (manual) pushToast(message, "warning");
        return;
      }
      if (this.downloadProgress !== null || this.status === "installing") return;
      const tag = this.latest.tagName;
      this.downloadProgress = { tag, downloaded: 0, total: 0 };
      this.status = "downloading";
      this.error = null;
      try {
        const ready = await downloadUpdateRelease(tag);
        if (this.latest?.tagName !== tag) {
          // 下载期间检查到了更新的版本：这份包仍然有效，只是已经不是最新，
          // 不把它放进 ready，避免用户安装到一个过时版本。
          this.ready = null;
          this.downloadProgress = null;
          this.status = this.hasUpdate ? "available" : "current";
          this.maybeAutoDownload();
          return;
        }
        this.ready = ready;
        this.downloadProgress = {
          tag,
          downloaded: ready.sizeBytes,
          total: ready.sizeBytes,
        };
        this.status = "ready";
        pushToast(`${tr("新版本已下载")} ${tag}`, "accent", {
          actionLabel: tr("立即重启"),
          durationMs: 20000,
          onAction: () => {
            void this.install();
          },
        });
      } catch (cause) {
        this.downloadProgress = null;
        this.status = this.hasUpdate ? "available" : "current";
        const message = errorMessage(cause);
        this.error = message;
        if (manual) {
          pushToast(message, "error");
        }
      }
    },

    applyProgress(progress: UpdateDownloadProgress) {
      if (!this.downloadProgress || progress.tag !== this.downloadProgress.tag) return;
      this.downloadProgress = {
        tag: progress.tag,
        downloaded: Math.max(0, progress.downloaded),
        total: Math.max(0, progress.total),
      };
    },

    /** 调用后应用会退出，由 NSIS 的 `/UPDATE /R` 重新拉起新版本。 */
    async install() {
      if (!this.ready) return;
      if (!this.downloadSupported) {
        this.reportError(tr("当前版本不支持应用内升级，请手动下载安装包"));
        return;
      }
      this.status = "installing";
      this.error = null;
      try {
        await installDownloadedUpdate();
      } catch (cause) {
        this.status = "ready";
        const message = errorMessage(cause);
        this.reportError(message);
        pushToast(message, "error");
      }
    },

    async autoCheck() {
      await this.hydrate();
      if (!this.autoCheckEnabled) return;
      const fresh =
        this.lastCheckedAt !== null && Date.now() - this.lastCheckedAt < CHECK_INTERVAL_MS;
      if (fresh) {
        await this.syncTray();
        if (this.hasUpdate) this.notify();
        this.maybeAutoDownload();
        return;
      }
      await this.check(false);
    },

    notify() {
      if (!this.hasUpdate || !this.latest) return;
      if (this.notifiedVersion === this.latest.tagName) return;
      this.notifiedVersion = this.latest.tagName;
      const version = this.latest.tagName;
      // accent 而非 info：发现新版本需要主动吸引注意，但不是错误；停留 20 秒，
      // 让用户来得及看到并点「查看」。
      pushToast(`${tr("发现新版本")} ${version}`, "accent", {
        actionLabel: tr("查看"),
        durationMs: 20000,
        // 「查看」是「带我去看」，不是「忽略」：不能在这里 dismiss，
        // 否则跳到关于页时 dismissedVersion 已置上，升级横幅会立刻消失。
        onAction: () => {
          window.dispatchEvent(new Event(SHOW_UPDATE_EVENT));
        },
        // 关闭提示只是收起这条 Toast，同样不改变「有没有新版本」的判定；
        // 要彻底不再提示，用横幅上的「忽略此版本」。
        onClose: () => undefined,
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
