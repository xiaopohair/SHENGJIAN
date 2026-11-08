import { ref } from "vue";

export type ToastKind = "info" | "success" | "error" | "warning";

export interface Toast {
  id: number;
  message: string;
  kind: ToastKind;
}

export const toasts = ref<Toast[]>([]);
let nextId = 1;
const timers = new Map<number, number>();

export function pushToast(message: string, kind: ToastKind = "info") {
  const id = nextId++;
  toasts.value.push({ id, message, kind });
  const timer = window.setTimeout(() => dismissToast(id), 4500);
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
