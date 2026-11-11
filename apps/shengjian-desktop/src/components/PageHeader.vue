<script setup lang="ts">
import { trDynamic } from "../lib/i18n";

withDefaults(
  defineProps<{
    eyebrow?: string;
    title: string;
    description?: string;
  }>(),
  { eyebrow: "", description: "" },
);
</script>

<template>
  <header class="ui-page-header">
    <div class="ui-page-heading">
      <p v-if="eyebrow" class="ui-page-eyebrow">{{ trDynamic(eyebrow) }}</p>
      <h1 class="ui-page-title">{{ trDynamic(title) }}</h1>
      <p v-if="description" class="ui-page-description">{{ trDynamic(description) }}</p>
      <slot name="meta" />
    </div>
    <div v-if="$slots.actions" class="ui-page-actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.ui-page-header {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  padding-top: var(--space-2);
}

.ui-page-heading {
  min-width: 0;
}

.ui-page-eyebrow {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.ui-page-title {
  margin-top: 3px;
  font-size: var(--text-2xl);
  font-weight: 650;
  letter-spacing: -0.02em;
  line-height: 1.2;
  overflow-wrap: anywhere;
}

.ui-page-description {
  margin-top: 6px;
  max-width: 68ch;
  color: var(--text-muted);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.ui-page-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  padding-top: 2px;
}

@media (max-width: 1260px) {
  .ui-page-header {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
