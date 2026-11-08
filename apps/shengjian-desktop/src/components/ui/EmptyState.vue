<script setup lang="ts">
import { trDynamic } from "../../lib/i18n";

withDefaults(
  defineProps<{
    title: string;
    description?: string;
  }>(),
  { description: "" },
);
</script>

<template>
  <div class="ui-empty">
    <div v-if="$slots.icon" class="ui-empty-icon">
      <slot name="icon" />
    </div>
    <p class="ui-empty-title">{{ trDynamic(title) }}</p>
    <p v-if="description" class="ui-empty-description">{{ trDynamic(description) }}</p>
    <div v-if="$slots.actions" class="ui-empty-actions">
      <slot name="actions" />
    </div>
  </div>
</template>

<style scoped>
.ui-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  min-height: 180px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-subtle);
  padding: var(--space-6) var(--space-4);
  text-align: center;
}

.ui-empty-icon {
  display: grid;
  place-items: center;
  width: 42px;
  height: 42px;
  border-radius: var(--radius-lg);
  background: var(--accent-soft);
  color: var(--accent);
}

.ui-empty-title {
  font-weight: 650;
}

.ui-empty-description {
  max-width: 46ch;
  color: var(--text-muted);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.ui-empty-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--space-2);
  margin-top: var(--space-2);
}
</style>
