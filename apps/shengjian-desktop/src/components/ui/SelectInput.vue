<script setup lang="ts">
import { ChevronDown } from "lucide-vue-next";

withDefaults(
  defineProps<{
    disabled?: boolean;
    ariaLabel?: string;
    selectId?: string;
  }>(),
  { disabled: false, ariaLabel: "", selectId: "" },
);

const model = defineModel<string>({ default: "" });
</script>

<template>
  <span class="ui-select">
    <select
      :id="selectId || undefined"
      v-model="model"
      class="ui-select-input"
      :disabled="disabled"
      :aria-label="ariaLabel || undefined"
    >
      <slot />
    </select>
    <ChevronDown :size="15" class="ui-select-icon" aria-hidden="true" />
  </span>
</template>

<style scoped>
.ui-select {
  position: relative;
  display: inline-flex;
  width: 100%;
  min-width: 0;
}

.ui-select-input {
  width: 100%;
  min-width: 0;
  min-height: 34px;
  appearance: none;
  border: 1px solid var(--border-control);
  border-radius: var(--radius-md);
  background: var(--bg);
  padding: 6px 30px 6px 10px;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
  transition: border-color var(--duration-fast) ease;
}

.ui-select-input:hover:not(:disabled) {
  border-color: var(--text-disabled);
}

.ui-select-input:focus-visible {
  border-color: var(--accent);
}

.ui-select-input:disabled {
  cursor: default;
  opacity: 0.55;
}

.ui-select-input option {
  background: var(--bg-raised);
  color: var(--text);
}

.ui-select-icon {
  position: absolute;
  top: 50%;
  right: 10px;
  color: var(--text-muted);
  pointer-events: none;
  transform: translateY(-50%);
}
</style>
