import { ref } from "vue";

export type ToastKind = "info" | "success" | "error" | "warning";

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
