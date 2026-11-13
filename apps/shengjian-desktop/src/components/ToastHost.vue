<script setup lang="ts">
import { ArrowUpCircle, CheckCircle2, Info, TriangleAlert, X, XCircle } from "lucide-vue-next";
import { dismissToast, toasts } from "../lib/toast";
import { trDynamic } from "../lib/i18n";

const ICONS = {
  info: Info,
  success: CheckCircle2,
  error: XCircle,
  warning: TriangleAlert,
  accent: ArrowUpCircle,
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
        v-if="toast.actionLabel"
        type="button"
        class="toast-action"
        @click="
          toast.onAction?.();
          dismissToast(toast.id);
        "
      >
        {{ toast.actionLabel }}
      </button>
      <button
        type="button"
        class="toast-close"
        :aria-label="trDynamic('关闭')"
        @click="
          toast.onClose?.();
          dismissToast(toast.id);
        "
      >
        <X :size="14" />
      </button>
    </div>
  </div>
</template>

<style scoped>
.toast-host {
  position: fixed;
  /* 顶部居中：标题栏 36px + 顶栏 56px 之下，落页面内容区最上方，
     既不遮挡窗口按钮和顶栏状态，又比右下角更容易被注意到。 */
  top: calc(var(--titlebar-height) + 64px);
  left: 50%;
  z-index: 130;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  width: max-content;
  max-width: min(560px, calc(100vw - 120px));
  transform: translateX(-50%);
  pointer-events: none;
}

/* 容器本身不挡点击，只有卡片接收事件（顶部位置会压在页面标题区上）。 */
.toast-host > * {
  pointer-events: auto;
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

/* 「发现新版本」一类：品牌色描边 + 浅底 + 外发光，比普通提示更抓眼。 */
.toast.accent {
  border-color: color-mix(in srgb, var(--accent) 70%, var(--border));
  background: color-mix(in srgb, var(--accent-soft) 65%, var(--bg-elevated));
  box-shadow:
    var(--shadow-md),
    0 0 0 1px color-mix(in srgb, var(--accent) 22%, transparent);
}

.toast.accent .toast-icon {
  color: var(--accent);
}

.toast.accent .toast-action {
  background: var(--accent);
  color: var(--bg);
}

.toast.accent .toast-action:hover {
  background: var(--accent-hover);
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

.toast-action {
  flex: 0 0 auto;
  border: 0;
  border-radius: var(--radius-xs);
  background: var(--accent-soft);
  padding: 4px 8px;
  color: var(--accent);
  font: inherit;
  font-size: var(--text-xs);
  font-weight: 650;
  cursor: pointer;
}

.toast-action:hover {
  background: var(--accent-soft-strong);
}

.toast-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

@keyframes toast-in {
  from {
    opacity: 0;
    transform: translateY(-8px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}
</style>
