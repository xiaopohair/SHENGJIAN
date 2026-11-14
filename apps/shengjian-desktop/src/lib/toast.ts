import { ref } from "vue";

/**
 * 提示类型。
 *
 * `accent` 用于「发现新版本」这类需要主动吸引注意、但又不是错误/警告的通知：
 * 用品牌主色描边与浅底，视觉上比 info 更跳，语义上又不会像 warning 那样让人以为出问题。
 */
export type ToastKind = "info" | "success" | "error" | "warning" | "accent";

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
  actionLabel?: string;
  onAction?: () => void;
  onClose?: () => void;
}

interface ToastOptions {
  actionLabel?: string;
  onAction?: () => void;
  onClose?: () => void;
  durationMs?: number;
}

/**
 * 同时最多显示几条。
 *
 * 标准见 `docs/ui-design-system.md` §4.1：同一时刻最多 3 条，超出丢弃最旧的——
 * Toast 从标题栏下方往下堆，堆太多会盖住页面标题与工具条。
 */
const MAX_VISIBLE = 3;

/**
 * 相同 message + kind 的去重窗口。
 *
 * 同一个错误在短时间内被反复触发（列表刷新、目标解析、轮询）时只提示一次，
 * 否则用户看到的是一叠一模一样的气泡。
 */
const DEDUPE_WINDOW_MS = 5000;

export const toasts = ref<Toast[]>([]);
let nextId = 1;
const timers = new Map<number, number>();
/** `kind\u0000message` → 最近一次推送时间（去重用）。 */
const recent = new Map<string, number>();

function pruneRecent(now: number) {
  for (const [key, at] of recent) {
    if (now - at >= DEDUPE_WINDOW_MS) recent.delete(key);
  }
}

export function pushToast(message: string, kind: ToastKind = "info", options: ToastOptions = {}) {
  const now = Date.now();
  const key = `${kind}\u0000${message}`;
  const last = recent.get(key);
  if (last !== undefined && now - last < DEDUPE_WINDOW_MS) {
    // 同一件事刚刚提示过：静默合并，不叠加第二条。
    return;
  }
  pruneRecent(now);
  recent.set(key, now);

  const id = nextId++;
  toasts.value.push({
    id,
    message,
    kind,
    actionLabel: options.actionLabel,
    onAction: options.onAction,
    onClose: options.onClose,
  });
  const timer = window.setTimeout(() => dismissToast(id), options.durationMs ?? 4500);
  timers.set(id, timer);

  // 超出上限：丢最旧的（连同它的计时器一起清掉）。
  while (toasts.value.length > MAX_VISIBLE) {
    const oldest = toasts.value[0];
    if (!oldest) break;
    dismissToast(oldest.id);
  }
}

export function dismissToast(id: number) {
  const timer = timers.get(id);
  if (timer !== undefined) {
    window.clearTimeout(timer);
    timers.delete(id);
  }
  toasts.value = toasts.value.filter((toast) => toast.id !== id);
}
