<script setup lang="ts">
import { tr, trDynamic } from "../../lib/i18n";
import { SYSTEM_KEYS } from "../../lib/systemActions";

withDefaults(
  defineProps<{
    modelValue?: string;
    disabled?: boolean;
  }>(),
  { modelValue: "", disabled: false },
);

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();
</script>

<template>
  <div class="system-action-grid" role="radiogroup" :aria-label="tr('系统动作')">
    <button
      v-for="item in SYSTEM_KEYS"
      :key="item.key"
      type="button"
      role="radio"
      class="system-action-card"
      :class="{ selected: modelValue === item.key }"
      :aria-checked="modelValue === item.key"
      :disabled="disabled"
      @click="emit('update:modelValue', item.key)"
    >
      <span class="system-action-icon" aria-hidden="true">
        <component :is="item.icon" v-if="item.icon" :size="16" />
        <kbd v-else class="system-action-keycap">{{ item.keycap ?? item.key }}</kbd>
      </span>
      <span v-if="item.icon" class="system-action-label">{{ trDynamic(item.label) }}</span>
    </button>
  </div>
</template>

<style scoped>
.system-action-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-2);
}

.system-action-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 4px;
  min-width: 0;
  min-height: 44px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: 6px 4px;
  color: var(--text-muted);
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.system-action-card:hover:not(:disabled) {
  border-color: var(--border-strong);
  background: var(--bg-hover);
  color: var(--text);
}

.system-action-card:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}

.system-action-card.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent-ink);
}

.system-action-card:disabled {
  cursor: default;
  opacity: 0.55;
}

.system-action-icon {
  display: grid;
  place-items: center;
  height: 18px;
  color: var(--tile-ink, var(--text-muted));
}

.system-action-card.selected .system-action-icon {
  color: var(--accent);
}

.system-action-keycap {
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  padding: 1px 5px;
  color: inherit;
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  line-height: 1.3;
}

.system-action-label {
  overflow: hidden;
  max-width: 100%;
  font-size: var(--text-2xs);
  font-weight: 600;
  line-height: 1.3;
  text-overflow: ellipsis;
  white-space: nowrap;
}
</style>
