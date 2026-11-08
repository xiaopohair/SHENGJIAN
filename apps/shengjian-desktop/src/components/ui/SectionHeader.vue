<script setup lang="ts">
import { trDynamic } from "../../lib/i18n";

withDefaults(
  defineProps<{
    title: string;
    hint?: string;
  }>(),
  { hint: "" },
);
</script>

<template>
  <header class="ui-section-head">
    <div class="ui-section-copy">
      <h2>{{ trDynamic(title) }}</h2>
      <p v-if="hint" class="ui-section-hint">{{ trDynamic(hint) }}</p>
    </div>
    <div v-if="$slots.actions" class="ui-section-actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.ui-section-head {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
}

.ui-section-copy {
  min-width: 0;
}

h2 {
  font-size: var(--text-lg);
  font-weight: 650;
}

.ui-section-hint {
  margin-top: 3px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.ui-section-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}
</style>
