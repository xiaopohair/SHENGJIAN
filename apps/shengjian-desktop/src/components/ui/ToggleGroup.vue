<script setup lang="ts">
import { trDynamic } from "../../lib/i18n";

export interface ToggleGroupItem {
  value: string;
  label: string;
  /** Optional swatch color shown before the label (e.g. accent choices). */
  swatch?: string;
}

withDefaults(
  defineProps<{
    items: readonly ToggleGroupItem[];
    /** Accessible name for the group. */
    ariaLabel?: string;
  }>(),
  { ariaLabel: "" },
);

const model = defineModel<string>({ required: true });
</script>

<template>
  <div class="toggle-group" role="group" :aria-label="ariaLabel ? trDynamic(ariaLabel) : undefined">
    <button
      v-for="item in items"
      :key="item.value"
      type="button"
      class="toggle-item"
      :class="{ active: model === item.value }"
      :aria-pressed="model === item.value"
      @click="model = item.value"
    >
      <span
        v-if="item.swatch"
        class="toggle-swatch"
        :style="{ backgroundColor: item.swatch }"
        aria-hidden="true"
      />
      {{ trDynamic(item.label) }}
    </button>
  </div>
</template>

<style scoped>
.toggle-group {
  display: inline-flex;
  gap: 3px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 3px;
}

.toggle-item {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  padding: 6px 10px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-xs);
  font-weight: 600;
  white-space: nowrap;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.toggle-item:hover {
  color: var(--text);
}

.toggle-item.active {
  background: var(--bg-raised);
  color: var(--text);
  box-shadow: var(--shadow-sm);
}

.toggle-item:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -1px;
}

.toggle-swatch {
  flex: 0 0 auto;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  box-shadow: inset 0 0 0 1px rgba(0, 0, 0, 0.12);
}
</style>
