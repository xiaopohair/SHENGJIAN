<script setup lang="ts">
withDefaults(
  defineProps<{
    disabled?: boolean;
    label?: string;
    /** `sm` = 紧凑尺寸，用在工具栏这类高密度位置。 */
    size?: "md" | "sm";
  }>(),
  { disabled: false, label: "", size: "md" },
);

const model = defineModel<boolean>({ default: false });
</script>

<template>
  <button
    type="button"
    class="ui-switch"
    :class="[`size-${size}`, { checked: model }]"
    role="switch"
    :aria-checked="model"
    :aria-label="label || undefined"
    :disabled="disabled"
    @click="model = !model"
  >
    <span class="ui-switch-thumb" />
  </button>
</template>

<style scoped>
.ui-switch {
  position: relative;
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  width: 38px;
  height: 22px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: var(--bg-hover);
  padding: 0;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease;
}

.ui-switch:hover:not(:disabled) {
  border-color: var(--text-disabled);
}

.ui-switch.checked {
  border-color: var(--accent);
  background: var(--accent);
}

.ui-switch:disabled {
  cursor: default;
  opacity: 0.5;
}

.ui-switch-thumb {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--text-muted);
  transform: translateX(2px);
  transition:
    background var(--duration-fast) ease,
    transform var(--duration-fast) ease;
}

.ui-switch.checked .ui-switch-thumb {
  background: var(--text-on-accent);
  transform: translateX(18px);
}

/* 紧凑尺寸（工具栏）：更小的轨与滑块，滑块位移同步缩小。 */
.ui-switch.size-sm {
  width: 30px;
  height: 17px;
}

.ui-switch.size-sm .ui-switch-thumb {
  width: 12px;
  height: 12px;
  transform: translateX(2px);
}

.ui-switch.size-sm.checked .ui-switch-thumb {
  transform: translateX(14px);
}
</style>
