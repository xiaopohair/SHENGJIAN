<script setup lang="ts">
import { CheckCircle2, Info, TriangleAlert, X, XCircle } from "lucide-vue-next";
import { dismissToast, toasts } from "../lib/toast";
import { trDynamic } from "../lib/i18n";

const ICONS = {
  info: Info,
  success: CheckCircle2,
  error: XCircle,
  warning: TriangleAlert,
} as const;
</script>

<template>
  <div class="toast-host" aria-live="polite">
    <div
      v-for="toast in toasts"
      :key="toast.id"
      class="toast"
      :class="toast.kind"
      :role="toast.kind === 'error' || toast.kind === 'warning' ? 'alert' : 'status'"
    >
      <component :is="ICONS[toast.kind]" :size="16" class="toast-icon" />
      <span class="toast-message">{{ toast.message }}</span>
      <button
        type="button"
        class="toast-close"
        :aria-label="trDynamic('关闭')"
        @click="dismissToast(toast.id)"
      >
        <X :size="14" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  right: 20px;
  bottom: 20px;
  z-index: 130;
  display: flex;
  flex-direction: column;
  gap: 8px;
  max-width: min(380px, calc(100vw - 40px));
}

.toast {
  display: flex;
  align-items: flex-start;
  gap: 9px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  padding: 10px 10px 10px 12px;
  color: var(--text);
  font-size: var(--text-sm);
  box-shadow: var(--shadow-md);
  animation: toast-in var(--duration-base) var(--ease-out);
}

.toast.success {
  border-color: color-mix(in srgb, var(--success) 55%, var(--border));
}

.toast.error {
  border-color: var(--danger);
}

.toast.warning {
  border-color: var(--warning);
}

.toast-icon {
  margin-top: 1px;
  color: var(--text-muted);
}

.toast.success .toast-icon {
  color: var(--success-text);
}

.toast.error .toast-icon {
  color: var(--danger-text);
}

.toast.warning .toast-icon {
  color: var(--warning-text);
}

.toast-message {
  flex: 1;
  min-width: 0;
  line-height: 1.45;
  overflow-wrap: anywhere;
}

.toast-close {
  display: grid;
  flex: 0 0 22px;
  place-items: center;
  width: 22px;
  height: 22px;
  border: 0;
  border-radius: var(--radius-xs);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.toast-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(6px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
