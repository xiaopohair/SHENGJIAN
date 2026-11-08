<script setup lang="ts">
withDefaults(
  defineProps<{
    disabled?: boolean;
  }>(),
  { disabled: false },
);

const model = defineModel<boolean>({ default: false });
</script>

<template>
  <label class="ui-checkbox" :class="{ disabled }">
    <input v-model="model" type="checkbox" :disabled="disabled" />
    <span class="ui-checkbox-box" aria-hidden="true" />
    <span class="ui-checkbox-label"><slot /></span>
  </label>
</template>

<style scoped>
.ui-checkbox {
  /* Keep the absolutely positioned native input inside this label. Without a
     positioned ancestor its containing block is the initial containing block,
     so it escapes the `.main-scroll` clip, inflates the document scroll height
     and keyboard focus scrolls the whole app shell. */
  position: relative;
  display: inline-flex;
  align-items: flex-start;
  gap: var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.45;
  cursor: pointer;
}

.ui-checkbox.disabled {
  cursor: default;
  opacity: 0.55;
}

.ui-checkbox input {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: 0;
  padding: 0;
  border: 0;
  overflow: hidden;
  opacity: 0;
  clip-path: inset(50%);
}

.ui-checkbox-box {
  display: grid;
  flex: 0 0 16px;
  place-items: center;
  width: 16px;
  height: 16px;
  margin-top: 1px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xs);
  background: var(--bg);
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease;
}

.ui-checkbox-box::after {
  width: 8px;
  height: 4px;
  border-bottom: 2px solid var(--text-on-accent);
  border-left: 2px solid var(--text-on-accent);
  content: "";
  opacity: 0;
  transform: translateY(-1px) rotate(-45deg) scale(0.7);
  transition:
    opacity var(--duration-fast) ease,
    transform var(--duration-fast) ease;
}

.ui-checkbox input:checked + .ui-checkbox-box {
  border-color: var(--accent);
  background: var(--accent);
}

.ui-checkbox input:checked + .ui-checkbox-box::after {
  opacity: 1;
  transform: translateY(-1px) rotate(-45deg) scale(1);
}

.ui-checkbox input:focus-visible + .ui-checkbox-box {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.ui-checkbox-label {
  min-width: 0;
}
</style>
