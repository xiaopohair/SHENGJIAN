<script setup lang="ts">
withDefaults(
  defineProps<{
    type?: "text" | "search" | "url" | "password" | "number";
    placeholder?: string;
    disabled?: boolean;
    maxlength?: number;
    inputId?: string;
  }>(),
  {
    type: "text",
    placeholder: "",
    disabled: false,
    maxlength: undefined,
    inputId: "",
  },
);

const model = defineModel<string>({ default: "" });
</script>

<template>
  <input
    :id="inputId || undefined"
    v-model="model"
    class="ui-input"
    :type="type"
    :placeholder="placeholder"
    :disabled="disabled"
    :maxlength="maxlength"
  />
</template>

<style scoped>
.ui-input {
  width: 100%;
  min-width: 0;
  min-height: 34px;
  border: 1px solid var(--border-control);
  border-radius: var(--radius-md);
  background: var(--bg);
  padding: 6px 10px;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.ui-input::placeholder {
  color: var(--text-muted);
}

.ui-input:hover:not(:disabled) {
  border-color: var(--text-disabled);
}

.ui-input:focus-visible {
  border-color: var(--accent);
}

.ui-input:disabled {
  cursor: default;
  opacity: 0.55;
}

.ui-input[type="search"]::-webkit-search-cancel-button {
  cursor: pointer;
}
</style>
