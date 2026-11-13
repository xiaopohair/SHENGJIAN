<script setup lang="ts">
import { computed, watch } from "vue";
import { tr } from "../../lib/i18n";
import {
  appTargetStatus,
  displayNameOf,
  subtitleOf,
  type AppTargetMode,
} from "../../lib/appTargets";
import { cachedAppTarget, describeAppTarget } from "../../composables/useAppPicker";

const props = withDefaults(
  defineProps<{
    target: string;
    mode?: AppTargetMode;
  }>(),
  { mode: "launch" },
);

const emit = defineEmits<{
  repair: [];
}>();

const info = computed(() => cachedAppTarget(props.target));
const status = computed(() => appTargetStatus(info.value, props.mode));
const name = computed(() => displayNameOf(info.value) || props.target.trim());
const subtitle = computed(() => subtitleOf(info.value));
const detail = computed(() =>
  status.value.detailKey
    ? tr(
        status.value.detailKey,
        status.value.reasonKey ? { reason: tr(status.value.reasonKey) } : undefined,
      )
    : "",
);

// Describe on mount and whenever the bound target changes. The cache is reactive,
// so the friendly name and flags update as soon as the batch response lands.
watch(
  () => props.target,
  (target) => {
    if (target.trim()) void describeAppTarget(target.trim());
  },
  { immediate: true },
);
</script>

<template>
  <span v-if="target.trim()" class="app-target-badge" :title="target">
    <span class="app-target-copy">
      <span class="app-target-name">{{ name }}</span>
      <span v-if="subtitle" class="app-target-sub" :title="subtitle">{{ subtitle }}</span>
    </span>
    <span
      v-if="status.labelKey"
      class="app-target-flag"
      :class="`tone-${status.tone}`"
      :title="detail"
    >
      {{ tr(status.labelKey) }}
    </span>
    <button
      v-if="status.repair"
      type="button"
      class="app-target-repair"
      :title="detail || tr('修复')"
      :aria-label="tr('修复')"
      @click="emit('repair')"
    >
      {{ tr("修复") }}
    </button>
  </span>
</template>

<style scoped>
.app-target-badge {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  min-width: 0;
  max-width: 100%;
}

.app-target-copy {
  display: inline-flex;
  flex-direction: column;
  min-width: 0;
  line-height: 1.3;
}

.app-target-name {
  overflow: hidden;
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-target-sub {
  overflow: hidden;
  max-width: 34ch;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-target-flag {
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-subtle);
  padding: 1px 7px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  white-space: nowrap;
}

.app-target-flag.tone-warning {
  border-color: var(--warning);
  color: var(--warning-text);
}

.app-target-flag.tone-danger {
  border-color: var(--danger-soft);
  color: var(--danger-text);
}

.app-target-flag.tone-info {
  border-color: var(--info);
  color: var(--info-text, var(--info));
}

.app-target-repair {
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: transparent;
  padding: 1px 8px;
  color: var(--accent-ink);
  font: inherit;
  font-size: var(--text-2xs);
  font-weight: 650;
  cursor: pointer;
  white-space: nowrap;
}

.app-target-repair:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}
</style>
