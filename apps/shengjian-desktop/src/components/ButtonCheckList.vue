<script setup lang="ts">
/**
 * 按键检测清单（只负责展示）。
 *
 * 首次配置的第 4 步和设备测试面板共用这一份，保证同一个功能只有一种样子。
 * 布局与配色沿用首次配置里的三列网格。
 */
import { computed } from "vue";
import { tr, trDynamic } from "../lib/i18n";
import StatusPill from "./ui/StatusPill.vue";

const props = withDefaults(
  defineProps<{
    buttons: Array<{ id: string; label: string; detected: boolean }>;
    /** 是否在头部显示"全部按键可用"（调用方自己在别处展示时可关掉）。 */
    showCompleteLabel?: boolean;
  }>(),
  { showCompleteLabel: true },
);

const total = computed(() => props.buttons.length);
const detectedCount = computed(() => props.buttons.filter((button) => button.detected).length);
</script>

<template>
  <div class="button-check-panel">
    <div class="button-check-head">
      <span class="muted">{{ tr("已检测") }} {{ detectedCount }}/{{ total }}</span>
      <span v-if="$slots.hint" class="button-check-hint"><slot name="hint" /></span>
      <span v-else-if="showCompleteLabel && detectedCount === total && total > 0" class="muted">
        {{ tr("全部按键可用") }}
      </span>
    </div>

    <div
      class="button-check-progress"
      role="progressbar"
      :aria-valuemin="0"
      :aria-valuemax="total"
      :aria-valuenow="detectedCount"
    >
      <span :style="{ width: total > 0 ? `${(detectedCount / total) * 100}%` : '0%' }" />
    </div>

    <ul class="button-check-list">
      <li
        v-for="button in buttons"
        :key="button.id"
        class="button-check-item"
        :class="{ detected: button.detected }"
      >
        <span class="button-check-name">{{ trDynamic(button.label) }}</span>
        <StatusPill
          :label="trDynamic(button.detected ? '可用' : '待检测')"
          :tone="button.detected ? 'success' : 'neutral'"
        />
      </li>
    </ul>
  </div>
</template>

<style scoped>
.button-check-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  width: 100%;
  text-align: left;
}

.button-check-hint {
  flex: 1;
  min-width: 0;
  text-align: right;
}

.button-check-progress {
  height: 4px;
  overflow: hidden;
  border-radius: 999px;
  background: var(--border);
}

.button-check-progress span {
  display: block;
  height: 100%;
  border-radius: inherit;
  background: var(--accent);
  transition: width 0.2s ease;
}

.button-check-head {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  font-size: var(--text-sm);
}

.button-check-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.button-check-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
}

.button-check-item.detected {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.button-check-name {
  font-size: var(--text-sm);
  font-weight: 600;
  overflow-wrap: anywhere;
}

@media (max-width: 1260px) {
  .button-check-list {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
