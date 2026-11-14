<script setup lang="ts">
import { computed } from "vue";
import { tr, trDynamic } from "../../lib/i18n";

/**
 * 导入/导出等长流程的进度反馈（`docs/ui-design-system.md` §4.1）。
 *
 * 两种形态，语义不同，**不允许编百分比**：
 * - `indeterminate`：单次调用（写一个文件、读一个文件），只表示「正在处理」；
 * - `determinate`：能按条计数（逐条/逐文件推进），给 `done` / `total`。
 */
const props = withDefaults(
  defineProps<{
    /** 进行中说明，如「正在导出…」。 */
    label: string;
    /** 已知总量时才传；不传即不定态。 */
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
  <div class="transfer-progress">
    <span v-if="!determinate" class="progress-spinner" aria-hidden="true" />
    <span class="progress-label">{{ trDynamic(label) }}</span>
    <span
      class="progress-bar"
      role="progressbar"
      aria-valuemin="0"
      aria-valuemax="100"
      :aria-valuenow="determinate ? percent : undefined"
      :aria-label="trDynamic(label)"
    >
      <i
        :class="{ indeterminate: !determinate }"
        :style="determinate ? { width: `${percent}%` } : undefined"
      />
    </span>
    <span v-if="determinate" class="progress-count">{{ done }} / {{ total }}</span>
    <span v-else class="progress-hint">{{ tr("请稍候") }}</span>
  </div>
</template>

<style scoped>
.transfer-progress {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.progress-spinner {
  width: 12px;
  height: 12px;
  border: 2px solid var(--border-strong);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: progress-spin 0.8s linear infinite;
}

@keyframes progress-spin {
  to {
    transform: rotate(360deg);
  }
}

.progress-label {
  color: var(--text);
  font-weight: 550;
}

.progress-bar {
  position: relative;
  flex: 1 1 140px;
  min-width: 120px;
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--bg-subtle);
}

.progress-bar i {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
  transition: width var(--duration-fast) ease;
}

.progress-bar i.indeterminate {
  width: 40%;
  animation: progress-slide 1.1s ease-in-out infinite;
}

@keyframes progress-slide {
  0% {
    transform: translateX(-100%);
  }
  100% {
    transform: translateX(250%);
  }
}

.progress-count,
.progress-hint {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
}
</style>
