<script setup lang="ts">
withDefaults(
  defineProps<{
    variant?: "primary" | "secondary" | "ghost" | "danger" | "danger-solid";
    size?: "sm" | "md";
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
    loading?: boolean;
  }>(),
  {
    variant: "secondary",
    size: "md",
    type: "button",
    disabled: false,
    loading: false,
  },
);
</script>

<template>
  <button
    :type="type"
    class="ui-button"
    :class="[`variant-${variant}`, `size-${size}`, { 'is-loading': loading }]"
    :disabled="disabled || loading"
    :aria-busy="loading || undefined"
  >
    <span v-if="loading" class="ui-spinner" aria-hidden="true" />
    <slot />
  </button>
</template>

<style scoped>
.ui-button {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  min-width: 0;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  padding: 0 13px;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  font-weight: 600;
  line-height: 1;
  white-space: nowrap;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.ui-button.size-md {
  min-height: 34px;
}

.ui-button.size-sm {
  min-height: 30px;
  padding: 0 10px;
  font-size: var(--text-xs);
}

.ui-button:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--bg-hover);
}

.ui-button:active:not(:disabled) {
  transform: translateY(1px);
}

.ui-button:disabled {
  cursor: default;
  opacity: 0.55;
}

.variant-primary {
  border-color: transparent;
  background: var(--accent);
  color: var(--text-on-accent);
  font-weight: 650;
}

.variant-primary:hover:not(:disabled) {
  border-color: transparent;
  background: var(--accent-hover);
}

.variant-ghost {
  border-color: transparent;
  background: transparent;
  color: var(--text-muted);
}

.variant-ghost:hover:not(:disabled) {
  border-color: transparent;
  background: var(--bg-hover);
  color: var(--text);
}

.variant-danger {
  border-color: var(--danger-soft);
  background: transparent;
  color: var(--danger-text);
}

.variant-danger:hover:not(:disabled) {
  border-color: var(--danger);
  background: var(--danger-soft);
}

.variant-danger-solid {
  border-color: transparent;
  background: var(--danger);
  color: var(--text-on-danger);
}

.variant-danger-solid:hover:not(:disabled) {
  border-color: transparent;
  background: var(--danger-hover);
}

.ui-spinner {
  width: 14px;
  height: 14px;
  border: 2px solid currentColor;
  border-right-color: transparent;
  border-radius: 50%;
  animation: spin 0.7s linear infinite;
  opacity: 0.8;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
