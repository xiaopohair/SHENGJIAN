<script setup lang="ts">
import { CheckCircle2, Info, TriangleAlert, XCircle } from "lucide-vue-next";

withDefaults(
  defineProps<{
    kind?: "info" | "success" | "warning" | "error";
  }>(),
  { kind: "info" },
);
</script>

<template>
  <div
    class="ui-notice"
    :class="`kind-${kind}`"
    :role="kind === 'error' || kind === 'warning' ? 'alert' : 'status'"
  >
    <component
      :is="
        kind === 'error'
          ? XCircle
          : kind === 'warning'
            ? TriangleAlert
            : kind === 'success'
              ? CheckCircle2
              : Info
      "
      :size="16"
      class="ui-notice-icon"
    />
    <div class="ui-notice-body">
      <slot />
    </div>
  </div>
</template>

<style scoped>
.ui-notice {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--notice-bg);
  padding: var(--space-3) var(--space-4);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.5;
}

.ui-notice-icon {
  margin-top: 2px;
  color: var(--text-muted);
}

.kind-success {
  border-color: var(--success);
  background: color-mix(in srgb, var(--success) 8%, transparent);
}

.kind-success .ui-notice-icon {
  color: var(--success-text);
}

.kind-warning {
  border-color: color-mix(in srgb, var(--warning) 55%, transparent);
  background: color-mix(in srgb, var(--warning) 8%, transparent);
}

.kind-warning .ui-notice-icon {
  color: var(--warning-text);
}

.kind-error {
  border-color: var(--danger-soft);
  background: color-mix(in srgb, var(--danger) 7%, transparent);
  color: var(--danger-text);
}

.kind-error .ui-notice-icon {
  color: var(--danger);
}

.ui-notice-body {
  min-width: 0;
  flex: 1;
}

.ui-notice-body :deep(strong) {
  color: var(--text);
  font-weight: 650;
}
</style>
