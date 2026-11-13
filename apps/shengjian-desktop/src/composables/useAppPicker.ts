/**
 * 「打开应用」选择器的共享状态：
 *
 * - 打开/关闭（业务组件只负责触发，浮层本身由 AppPickerDialog 渲染）；
 * - 应用列表的加载与刷新（模块级缓存，重复打开不会重复扫描）；
 * - `describe_app_targets` 的批量缓存：模块级 Map + 同一 target 的 in-flight 去重，
 *   让徽标、摘要、对话框共用一次请求；
 * - 失败统一走 toast，不再静默吞掉错误。
 */
import { reactive, ref, shallowRef } from "vue";
import { appIconDataUrls, describeAppTargets, errorMessage, listLaunchableApps } from "../lib/api";
import { tr } from "../lib/i18n";
import { pushToast } from "../lib/toast";
import type { AppTargetInfo, LaunchableApp } from "../lib/types";

const targetCache = reactive(new Map<string, AppTargetInfo>());
const inFlight = new Map<string, Promise<AppTargetInfo | null>>();
let pendingTargets: string[] = [];
let pendingResolvers: Array<(info: AppTargetInfo | null) => void> = [];
let flushScheduled = false;
let lastDescribeFailureAt = 0;

function reportDescribeFailure(cause: unknown) {
  // A page can describe several targets while typing; one toast per burst is enough.
  const now = Date.now();
  if (now - lastDescribeFailureAt < 5000) return;
  lastDescribeFailureAt = now;
  pushToast(`${tr("无法解析应用目标")}：${errorMessage(cause)}`, "error");
}

function flushDescribeQueue() {
  flushScheduled = false;
  const targets = pendingTargets;
  const resolvers = pendingResolvers;
  pendingTargets = [];
  pendingResolvers = [];
  if (targets.length === 0) return;

  void describeAppTargets(targets)
    .then((infos) => {
      const byInput = new Map<string, AppTargetInfo>();
      for (const info of infos) {
        const input = (info?.input ?? "").trim();
        if (input) byInput.set(input, info);
      }
      targets.forEach((target, index) => {
        const info = byInput.get(target) ?? infos[index] ?? null;
        if (info) targetCache.set(target, info);
        resolvers[index]?.(info);
      });
    })
    .catch((cause: unknown) => {
      for (const resolve of resolvers) resolve(null);
      reportDescribeFailure(cause);
    })
    .finally(() => {
      for (const target of targets) inFlight.delete(target);
    });
}

/** 从缓存同步读取描述（响应式：缓存更新后依赖它的 computed 会重算）。 */
export function cachedAppTarget(target: string | null | undefined): AppTargetInfo | undefined {
  const key = (target ?? "").trim();
  return key ? targetCache.get(key) : undefined;
}

/**
 * 描述一个 target（命中缓存直接返回；未命中时与同批调用合并成一次批量请求，
 * 同一个 target 的并发调用共享 in-flight promise）。失败返回 null 并 toast。
 */
export function describeAppTarget(target: string): Promise<AppTargetInfo | null> {
  const key = (target ?? "").trim();
  if (!key) return Promise.resolve(null);
  const cached = targetCache.get(key);
  if (cached) return Promise.resolve(cached);
  const running = inFlight.get(key);
  if (running) return running;

  const promise = new Promise<AppTargetInfo | null>((resolve) => {
    pendingTargets.push(key);
    pendingResolvers.push(resolve);
    if (!flushScheduled) {
      flushScheduled = true;
      queueMicrotask(flushDescribeQueue);
    }
  });
  inFlight.set(key, promise);
  return promise;
}

/** 批量描述；等价于对每个 target 调用 `describeAppTarget`。 */
export function describeAppTargetsCached(targets: readonly string[]): Promise<void> {
  return Promise.all(targets.map((target) => describeAppTarget(target))).then(() => undefined);
}

/** 摘要/搜索用的友好名：缓存命中时优先，未命中先回退原串，等描述回来后自动更新。 */
export function appTargetFriendlyName(target: string, fallback?: string): string {
  const info = cachedAppTarget(target);
  const display = info?.displayName?.trim();
  if (display) return display;
  const executable = info?.executableName?.trim();
  if (executable) return executable;
  return fallback ?? target;
}

// ---- 应用图标（模块级缓存；取不到时列表用通用占位图标） ----

const iconCache = reactive(new Map<string, string>());
const iconInFlight = new Set<string>();

/** 同步读取已缓存的图标 data URL。 */
export function appIconFor(target: string | null | undefined): string | undefined {
  const key = (target ?? "").trim();
  return key ? iconCache.get(key) : undefined;
}

/** 批量拉取图标；图标只是装饰信息，失败静默、下次打开重试。 */
export async function loadAppIcons(targets: readonly string[]): Promise<void> {
  const missing = [...new Set(targets.map((target) => target.trim()).filter(Boolean))].filter(
    (target) => !iconCache.has(target) && !iconInFlight.has(target),
  );
  if (missing.length === 0) return;
  for (const target of missing) iconInFlight.add(target);
  try {
    const urls = await appIconDataUrls(missing);
    missing.forEach((target, index) => {
      const url = urls[index];
      if (url) iconCache.set(target, url);
    });
  } catch {
    // 图标失败不打扰用户，列表显示通用图标。
  } finally {
    for (const target of missing) iconInFlight.delete(target);
  }
}

// ---- 可启动应用列表（模块级，打开过一次后即时展示） ----

const apps = shallowRef<LaunchableApp[]>([]);
const appsLoaded = ref(false);
const appsLoading = ref(false);
const appsError = shallowRef<string | null>(null);
const appsScanned = ref(false);
const appsScanError = shallowRef<string | null>(null);
let listPromise: Promise<void> | null = null;

async function loadApps(force = false): Promise<void> {
  if (listPromise && !force) return listPromise;
  appsLoading.value = true;
  appsError.value = null;
  const task = (async () => {
    try {
      const result = await listLaunchableApps(force);
      apps.value = result.apps ?? [];
      appsScanned.value = result.scanned;
      appsScanError.value = result.scanError;
      appsLoaded.value = true;
    } catch (cause) {
      appsError.value = errorMessage(cause);
      pushToast(`${tr("无法加载应用列表")}：${appsError.value}`, "error");
    } finally {
      appsLoading.value = false;
      listPromise = null;
    }
  })();
  listPromise = task;
  return task;
}

/** 业务组件用它取打开状态；对话框用它取列表状态。 */
export function useAppPicker() {
  const open = ref(false);

  function openPicker() {
    open.value = true;
  }

  function closePicker() {
    open.value = false;
  }

  return {
    open,
    openPicker,
    closePicker,
    apps,
    appsLoaded,
    appsLoading,
    appsError,
    appsScanned,
    appsScanError,
    loadApps,
    refreshApps: () => loadApps(true),
  };
}

/** 测试辅助：清空模块级缓存，避免用例之间互相污染。 */
export function resetAppPickerState() {
  targetCache.clear();
  iconCache.clear();
  iconInFlight.clear();
  inFlight.clear();
  pendingTargets = [];
  pendingResolvers = [];
  flushScheduled = false;
  lastDescribeFailureAt = 0;
  apps.value = [];
  appsLoaded.value = false;
  appsLoading.value = false;
  appsError.value = null;
  appsScanned.value = false;
  appsScanError.value = null;
  listPromise = null;
}
