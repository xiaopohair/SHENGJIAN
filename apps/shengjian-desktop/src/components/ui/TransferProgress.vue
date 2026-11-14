<script setup lang="ts">
import { computed } from "vue";

/**
 * 导入/导出等长流程的进度反馈（`docs/ui-design-system.md` §4.1）。
 *
 * 视觉与按键方案导入导出弹窗的忙碌层一致（转圈 + 说明 + 进度条 + 计数），
 * 语义上**不允许编百分比**：
 * - 不传 `total`：单次调用（写一个文件、读一个文件），只表示「正在处理」；
 * - 传 `total` + `done`：能按条计数（逐文件/逐条推进）才给百分比。
 */
const props = withDefaults(
  defineProps<{
    /** 进行中说明，如「正在导出…」。 */
    label: string;
    done?: number | null;
    total?: number | null;
  }>(),
  { done: null, total: null },
);

const determinate = computed(
  () =>
    props.total !== null &&
    props.total !== undefined &&
    props.total > 0 &&
    props.done !== null &&
    props.done !== undefined,
);
const percent = computed(() => {
  if (!determinate.value) return 0;
  return Math.min(100, Math.round(((props.done ?? 0) / (props.total ?? 1)) * 100));
});
</script>

<template>
  <div class="transfer-progress" :aria-busy="true">
    <span class="busy-spinner" aria-hidden="true" />
    <span class="busy-label">{{ label }}</span>
    <span
      class="busy-bar"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-valuenow="determinate ? percent : undefined"
      :aria-label="label"
    >
      <i
        :class="{ indeterminate: !determinate }"
        :style="determinate ? { width: `${percent}%` } : undefined"
      />
    </span>
    <span v-if="determinate" class="busy-count">{{ done }} / {{ total }}</span>
  </div>
</template>

<style scoped>
.transfer-progress {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-3);
}

.busy-spinner {
  width: 22px;
  height: 22px;
  border: 2px solid var(--border-strong);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: transfer-spin 0.8s linear infinite;
}

@keyframes transfer-spin {
  to {
    transform: rotate(360deg);
  }
}

.busy-label {
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.busy-bar {
  position: relative;
  width: 280px;
  max-width: 70vw;
  height: 3px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--bg-hover);
}

.busy-bar i {
  position: absolute;
  inset: 0 auto 0 0;
  width: 0;
  border-radius: 2px;
  background: var(--accent);
  transition: width 160ms var(--ease-out, ease);
}

.busy-bar i.indeterminate {
  width: 40%;
  transition: none;
  animation: transfer-indeterminate 1.1s ease-in-out infinite;
}

.busy-count {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

@keyframes transfer-indeterminate {
  0% {
    left: -40%;
  }
  100% {
    left: 100%;
  }
}

@media (prefers-reduced-motion: reduce) {
  .busy-spinner,
  .busy-bar i {
    animation: none;
  }
}
</style>
