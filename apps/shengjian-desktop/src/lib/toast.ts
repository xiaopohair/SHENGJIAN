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

export const toasts = ref<Toast[]>([]);
let nextId = 1;
const timers = new Map<number, number>();

export function pushToast(message: string, kind: ToastKind = "info", options: ToastOptions = {}) {
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
}

export function dismissToast(id: number) {
  const timer = timers.get(id);
  if (timer !== undefined) {
    window.clearTimeout(timer);
    timers.delete(id);
  }
  toasts.value = toasts.value.filter((toast) => toast.id !== id);
}
