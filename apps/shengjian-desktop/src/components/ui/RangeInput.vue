<script setup lang="ts">
import { computed } from "vue";

const props = withDefaults(
  defineProps<{
    min?: number;
    max?: number;
    step?: number;
    disabled?: boolean;
  }>(),
  { min: 0, max: 100, step: 1, disabled: false },
);

const model = defineModel<number>({ default: 0 });

const percent = computed(() => {
  const span = props.max - props.min;
  if (span <= 0) return 0;
  return Math.max(0, Math.min(100, ((model.value - props.min) / span) * 100));
});
</script>

<template>
  <input
    v-model.number="model"
    class="ui-range"
    type="range"
    :min="min"
    :max="max"
    :step="step"
    :disabled="disabled"
    :style="{ '--range-percent': `${percent}%` }"
  />
</template>

<style scoped>
.ui-range {
  width: 100%;
  height: 20px;
  margin: 0;
  background: transparent;
  cursor: pointer;
  appearance: none;
}

.ui-range:disabled {
  cursor: default;
  opacity: 0.5;
}

.ui-range::-webkit-slider-runnable-track {
  height: 6px;
  border-radius: var(--radius-pill);
  background: linear-gradient(
    to right,
    var(--accent) 0,
    var(--accent) var(--range-percent),
    var(--bg-hover) var(--range-percent),
    var(--bg-hover) 100%
  );
}

.ui-range::-webkit-slider-thumb {
  width: 16px;
  height: 16px;
  margin-top: -5px;
  appearance: none;
  border: 2px solid var(--bg-raised);
  border-radius: 50%;
  background: var(--accent);
  box-shadow: var(--shadow-sm);
}

.ui-range:focus-visible::-webkit-slider-thumb {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.ui-range::-moz-range-track {
  height: 6px;
  border-radius: var(--radius-pill);
  background: var(--bg-hover);
}

.ui-range::-moz-range-progress {
  height: 6px;
  border-radius: var(--radius-pill);
  background: var(--accent);
}

.ui-range::-moz-range-thumb {
  width: 14px;
  height: 14px;
  border: 2px solid var(--bg-raised);
  border-radius: 50%;
  background: var(--accent);
}
</style>
