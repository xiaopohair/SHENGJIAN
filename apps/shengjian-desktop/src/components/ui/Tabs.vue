<script setup lang="ts">
import { nextTick, ref } from "vue";
import { trDynamic } from "../../lib/i18n";

interface TabItem {
  id: string;
  label: string;
  count?: number;
}

const props = withDefaults(
  defineProps<{
    items: TabItem[];
    ariaLabel?: string;
  }>(),
  { ariaLabel: "标签页" },
);

const model = defineModel<string>({ required: true });
const buttons = ref<HTMLButtonElement[]>([]);

function select(id: string) {
  model.value = id;
  void nextTick(() => {
    const index = props.items.findIndex((item) => item.id === id);
    buttons.value[index]?.focus();
  });
}

function onKeydown(event: KeyboardEvent, index: number) {
  let next: number | null = null;
  if (event.key === "ArrowRight") next = (index + 1) % props.items.length;
  else if (event.key === "ArrowLeft") next = (index - 1 + props.items.length) % props.items.length;
  else if (event.key === "Home") next = 0;
  else if (event.key === "End") next = props.items.length - 1;
  if (next === null) return;
  event.preventDefault();
  select(props.items[next].id);
}
</script>

<template>
  <div class="ui-tabs" role="tablist" :aria-label="trDynamic(ariaLabel)">
    <button
      v-for="(item, index) in items"
      :id="`tab-${item.id}`"
      :key="item.id"
      ref="buttons"
      type="button"
      role="tab"
      class="ui-tab"
      :class="{ active: model === item.id }"
      :aria-selected="model === item.id"
      :aria-controls="model === item.id ? `panel-${item.id}` : undefined"
      :tabindex="model === item.id ? 0 : -1"
      @click="select(item.id)"
      @keydown="onKeydown($event, index)"
    >
      {{ trDynamic(item.label) }}
      <span v-if="item.count !== undefined" class="ui-tab-count">{{ item.count }}</span>
    </button>
  </div>
</template>

<style scoped>
.ui-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
  border-bottom: 1px solid var(--border);
  padding-bottom: var(--space-2);
}

.ui-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  padding: 6px 10px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.ui-tab:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.ui-tab.active {
  background: var(--accent-soft);
  color: var(--text);
  font-weight: 600;
}

.ui-tab-count {
  min-width: 20px;
  border-radius: var(--radius-pill);
  background: var(--bg-hover);
  padding: 0 6px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  text-align: center;
}

.ui-tab.active .ui-tab-count {
  background: var(--bg-raised);
  color: var(--text);
}
</style>
