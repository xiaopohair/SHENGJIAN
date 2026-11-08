import { reactive } from "vue";

export interface ConfirmOptions {
  title: string;
  message?: string;
  detail?: string;
  confirmLabel?: string;
  cancelLabel?: string;
  danger?: boolean;
}

interface ConfirmState extends ConfirmOptions {
  open: boolean;
  resolve: ((value: boolean) => void) | null;
}

export const confirmState = reactive<ConfirmState>({
  open: false,
  title: "",
  message: "",
  detail: "",
  confirmLabel: "",
  cancelLabel: "",
  danger: false,
  resolve: null,
});

export function confirmAction(options: ConfirmOptions): Promise<boolean> {
  confirmState.resolve?.(false);
  Object.assign(confirmState, {
    open: true,
    title: options.title,
    message: options.message ?? "",
    detail: options.detail ?? "",
    confirmLabel: options.confirmLabel ?? "",
    cancelLabel: options.cancelLabel ?? "",
    danger: options.danger ?? false,
  });
  return new Promise<boolean>((resolve) => {
    confirmState.resolve = resolve;
  });
}

export function settleConfirm(value: boolean) {
  const resolve = confirmState.resolve;
  confirmState.open = false;
  confirmState.resolve = null;
  resolve?.(value);
}
