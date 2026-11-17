<script setup lang="ts">
import { trDynamic } from "../../lib/i18n";

withDefaults(
  defineProps<{
    /** Row title. */
    label: string;
    /** Optional description shown under the title. */
    hint?: string;
  }>(),
  { hint: "" },
);
</script>

<template>
  <div class="setting-row">
    <div class="setting-row-copy">
      <span class="setting-row-label">{{ trDynamic(label) }}</span>
      <span v-if="hint" class="setting-row-hint">{{ trDynamic(hint) }}</span>
    </div>
    <div class="setting-row-control">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  min-width: 0;
  /* 没有副标题的行也要有稳定高度：用栅格上的 --row-min-height 撑节奏，
     否则 12px 上下内边距会让"只有标题 + 开关"的行显得挤在一起。 */
  min-height: var(--row-min-height);
  padding: var(--space-3) 0;
}

/* Separator between rows, but not after the last one. */
.setting-row + .setting-row {
  border-top: 1px solid var(--border);
}

.setting-row-copy {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.setting-row-label {
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 600;
  line-height: 1.5;
}

.setting-row-hint {
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
  /* 允许 hint 里用 \n 分成两行（用户要求的"两行说明"）。 */
  white-space: pre-line;
}

.setting-row-control {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}
</style>
