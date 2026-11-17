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
      <component :is="ICONS[toast.kind]" :size="20" class="toast-icon" />
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
      <!-- 底部倒计时条：一眼能看出"这条会自己消失"，也让它更容易被注意到。 -->
      <span
        class="toast-progress"
        :style="{ animationDuration: `${toast.durationMs}ms` }"
        aria-hidden="true"
      />
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
  /* 顶层：必须高于所有弹层（模态弹窗 300、抽屉 240、标题栏 200），
     否则用户在弹窗里看不到错误提示。层级阶梯见 docs/ui-design-system.md §4.1。 */
  z-index: 400;
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
  gap: 10px;
  min-width: 360px;
  border: 1px solid var(--border-strong);
  /* 左侧色条 + 更大字号：原来 13px + 普通描边，放在页面顶部很容易被当成背景。 */
  border-left: 4px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  padding: 14px 12px 14px 16px;
  color: var(--text);
  font-size: var(--text-base);
  font-weight: 500;
  line-height: 1.5;
  box-shadow: var(--shadow-lg);
  animation: toast-in 240ms var(--ease-out);
  position: relative;
  overflow: hidden;
}

.toast-progress {
  position: absolute;
  bottom: 0;
  left: 0;
  height: 3px;
  width: 100%;
  background: var(--border-strong);
  transform-origin: left center;
  animation: toast-progress linear forwards;
}

.toast.success .toast-progress {
  background: var(--success);
}

.toast.error .toast-progress {
  background: var(--danger);
}

.toast.warning .toast-progress {
  background: var(--warning);
}

@keyframes toast-progress {
  from {
    transform: scaleX(1);
  }
  to {
    transform: scaleX(0);
  }
}

.toast.success {
  border-color: color-mix(in srgb, var(--success) 55%, var(--border));
  border-left-color: var(--success);
  background: color-mix(in srgb, var(--success) 12%, var(--bg-elevated));
}

.toast.error {
  border-color: var(--danger);
  border-left-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 14%, var(--bg-elevated));
}

.toast.warning {
  border-color: var(--warning);
  border-left-color: var(--warning);
  background: color-mix(in srgb, var(--warning) 14%, var(--bg-elevated));
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
