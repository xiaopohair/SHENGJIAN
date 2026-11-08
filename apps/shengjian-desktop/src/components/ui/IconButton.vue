<script setup lang="ts">
withDefaults(
  defineProps<{
    label: string;
    variant?: "default" | "danger";
    size?: "sm" | "md";
    type?: "button" | "submit" | "reset";
    disabled?: boolean;
  }>(),
  {
    variant: "default",
    size: "md",
    type: "button",
    disabled: false,
  },
);
</script>

<template>
  <button
    :type="type"
    class="ui-icon-button"
    :class="[`variant-${variant}`, `size-${size}`]"
    :disabled="disabled"
    :aria-label="label"
    :title="label"
  >
    <slot />
  </button>
</template>

<style scoped>
.ui-icon-button {
  display: inline-grid;
  flex: 0 0 auto;
  place-items: center;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  padding: 0;
  color: var(--text-muted);
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.ui-icon-button.size-md {
  width: 32px;
  height: 32px;
}

.ui-icon-button.size-sm {
  width: 28px;
  height: 28px;
}

.ui-icon-button:hover:not(:disabled) {
  border-color: var(--border-strong);
  background: var(--bg-hover);
  color: var(--text);
}

.ui-icon-button:disabled {
  cursor: default;
  opacity: 0.35;
}

.ui-icon-button.variant-danger:hover:not(:disabled) {
  border-color: var(--danger-soft);
  background: var(--danger-soft);
  color: var(--danger-text);
}
</style>
