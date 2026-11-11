<script setup lang="ts">
import { tr, trDynamic } from "../../lib/i18n";

withDefaults(
  defineProps<{
    label: string;
    hint?: string;
    optional?: boolean;
    forId?: string;
  }>(),
  { hint: "", optional: false, forId: "" },
);
</script>

<template>
  <label class="ui-field" :for="forId || undefined">
    <span class="ui-field-label">
      {{ trDynamic(label) }}
      <span v-if="optional" class="ui-field-optional">{{ tr("可选") }}</span>
    </span>
    <span class="ui-field-control">
      <slot />
      <span v-if="hint" class="ui-field-hint">{{ trDynamic(hint) }}</span>
    </span>
  </label>
</template>

<style scoped>
.ui-field {
  display: grid;
  grid-template-columns: minmax(140px, 200px) minmax(0, 1fr);
  align-items: center;
  gap: var(--space-2) var(--space-3);
  min-width: 0;
}

.ui-field-label {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.4;
}

.ui-field-optional {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.ui-field-control {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.ui-field-hint {
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.4;
}

@media (max-width: 1260px) {
  .ui-field {
    grid-template-columns: 1fr;
    align-items: stretch;
  }
}
</style>
