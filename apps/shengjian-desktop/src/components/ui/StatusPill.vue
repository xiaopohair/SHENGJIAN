<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string;
    tone?: "neutral" | "success" | "warning" | "danger" | "info" | "accent";
    dot?: boolean;
  }>(),
  { tone: "neutral", dot: false },
);
</script>

<template>
  <span class="ui-status" :class="`tone-${tone}`">
    <span v-if="dot" class="ui-status-dot" aria-hidden="true" />
    {{ label }}
  </span>
</template>

<style scoped>
.ui-status {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-raised);
  padding: 2px 8px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
  white-space: nowrap;
}

.ui-status-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: currentColor;
}

.tone-success {
  border-color: color-mix(in srgb, var(--success) 40%, var(--border));
  color: var(--success-text);
}

.tone-warning {
  border-color: color-mix(in srgb, var(--warning) 45%, var(--border));
  color: var(--warning-text);
}

.tone-danger {
  border-color: var(--danger-soft);
  color: var(--danger-text);
}

.tone-info {
  border-color: color-mix(in srgb, var(--info) 40%, var(--border));
  color: var(--info-text);
}

.tone-accent {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  background: var(--accent-soft);
  color: var(--text);
}
</style>
