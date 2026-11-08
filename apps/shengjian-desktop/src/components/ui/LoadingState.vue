<script setup lang="ts">
import { trDynamic } from "../../lib/i18n";

withDefaults(
  defineProps<{
    label?: string;
    description?: string;
  }>(),
  { label: "正在加载…", description: "" },
);
</script>

<template>
  <div class="ui-loading" role="status" aria-live="polite">
    <span class="ui-loading-spinner" aria-hidden="true" />
    <p class="ui-loading-label">{{ trDynamic(label) }}</p>
    <p v-if="description" class="ui-loading-description">{{ trDynamic(description) }}</p>
  </div>
</template>

<style scoped>
.ui-loading {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  min-height: 180px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-subtle);
  padding: var(--space-6) var(--space-4);
  text-align: center;
}

.ui-loading-spinner {
  width: 20px;
  height: 20px;
  border: 2px solid var(--border-strong);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

.ui-loading-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: 600;
}

.ui-loading-description {
  max-width: 46ch;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.6;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (prefers-reduced-motion: reduce) {
  .ui-loading-spinner {
    animation-duration: 1.8s;
  }
}
</style>
