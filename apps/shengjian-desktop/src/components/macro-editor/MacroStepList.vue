<script setup lang="ts">
import { computed, onBeforeUnmount, ref, shallowRef } from "vue";
import { ArrowDown, ArrowUp, CircleAlert, GripVertical, Plus } from "lucide-vue-next";
import { statusLabel, tr, trDynamic } from "../../lib/i18n";
import { actionIcon, actionLabel, actionTone, runStepTone } from "../../lib/macroEditor";
import type { MacroStepReport } from "../../lib/types";
import type { MacroStepRow } from "../../composables/useMacroDraftEditor";
import IconButton from "../ui/IconButton.vue";

const props = defineProps<{
  rows: MacroStepRow[];
  selectedStepId: string | null;
  runSteps: MacroStepReport[];
}>();

const emit = defineEmits<{
  select: [index: number];
  add: [];
  insert: [index: number];
  move: [delta: number];
  reorder: [from: number, to: number];
}>();

const selectedIndex = computed(() =>
  props.rows.findIndex((row) => row.step.stepId === props.selectedStepId),
);

const listRef = ref<HTMLElement | null>(null);
const dragFrom = shallowRef<number | null>(null);
const dragOver = shallowRef<number | null>(null);
const dragHandle = ref<HTMLElement | null>(null);
let dragPointerId: number | null = null;
const runByIndex = computed(() => {
  const map = new Map<number, MacroStepReport>();
  for (const step of props.runSteps) map.set(step.index, step);
  return map;
});

/** Index of the step row under the pointer, or null when outside the list. */
function rowIndexAt(clientY: number): number | null {
  const list = listRef.value;
  if (!list) return null;
  const elements = [...list.querySelectorAll<HTMLElement>(".step-row")];
  for (const element of elements) {
    const rect = element.getBoundingClientRect();
    if (clientY >= rect.top && clientY <= rect.bottom) {
      return Number(element.dataset.index);
    }
  }
  if (elements.length === 0) return null;
  const first = elements[0].getBoundingClientRect();
  if (clientY < first.top) return 0;
  const last = elements[elements.length - 1].getBoundingClientRect();
  if (clientY > last.bottom) return elements.length - 1;
  return null;
}

function onPointerMove(event: PointerEvent) {
  const index = rowIndexAt(event.clientY);
  if (index !== null) dragOver.value = index;
}

function stopDrag(commit: boolean) {
  window.removeEventListener("pointermove", onPointerMove);
  window.removeEventListener("pointerup", onPointerUp);
  window.removeEventListener("pointercancel", onPointerCancel);
  if (dragHandle.value && dragPointerId !== null) {
    try {
      dragHandle.value.releasePointerCapture(dragPointerId);
    } catch {
      // Capture already released.
    }
  }
  dragHandle.value = null;
  dragPointerId = null;
  const from = dragFrom.value;
  const to = dragOver.value;
  dragFrom.value = null;
  dragOver.value = null;
  if (commit && from !== null && to !== null && from !== to) {
    emit("reorder", from, to);
  }
}

function onPointerUp() {
  stopDrag(true);
}

function onPointerCancel() {
  stopDrag(false);
}

function startDrag(index: number, event: PointerEvent) {
  if (event.button !== 0) return;
  event.preventDefault();
  event.stopPropagation();
  dragFrom.value = index;
  dragOver.value = index;
  const handle = event.currentTarget as HTMLElement | null;
  if (handle) {
    dragHandle.value = handle;
    dragPointerId = event.pointerId;
    try {
      handle.setPointerCapture(event.pointerId);
    } catch {
      // Capture is an optimization; window listeners still track the drag.
    }
  }
  window.addEventListener("pointermove", onPointerMove);
  window.addEventListener("pointerup", onPointerUp);
  window.addEventListener("pointercancel", onPointerCancel);
}

onBeforeUnmount(() => stopDrag(false));
</script>

<template>
  <section class="macro-panel steps-pane">
    <div class="pane-head">
      <div>
        <h3>{{ tr("步骤") }}</h3>
        <span>{{ rows.length }} {{ tr("个步骤") }}</span>
      </div>
      <div class="step-toolbar">
        <IconButton
          size="sm"
          :label="trDynamic('上移步骤')"
          :disabled="selectedIndex <= 0"
          @click="emit('move', -1)"
        >
          <ArrowUp :size="14" />
        </IconButton>
        <IconButton
          size="sm"
          :label="trDynamic('下移步骤')"
          :disabled="selectedIndex < 0 || selectedIndex >= rows.length - 1"
          @click="emit('move', 1)"
        >
          <ArrowDown :size="14" />
        </IconButton>
        <IconButton size="sm" :label="trDynamic('添加步骤')" @click="emit('add')">
          <Plus :size="14" />
        </IconButton>
      </div>
    </div>

    <div ref="listRef" class="step-list">
      <template v-for="row in rows" :key="row.step.stepId">
        <div
          class="step-row"
          :class="{
            selected: selectedStepId === row.step.stepId,
            dragging: dragFrom === row.index,
            'drag-over': dragOver === row.index && dragFrom !== row.index,
            invalid: Boolean(row.issue),
          }"
          role="button"
          tabindex="0"
          :data-index="row.index"
          :aria-pressed="selectedStepId === row.step.stepId"
          :aria-label="`${trDynamic('步骤')} ${row.index + 1}: ${actionLabel(row.step.action)}, ${row.summary}`"
          @click="emit('select', row.index)"
          @keydown.enter.prevent="emit('select', row.index)"
          @keydown.space.prevent="emit('select', row.index)"
        >
          <kbd class="step-key">{{ row.index + 1 }}</kbd>
          <span
            class="action-tile"
            :class="`tone-${actionTone(row.step.action)}`"
            aria-hidden="true"
          >
            <component :is="actionIcon(row.step.action)" :size="16" />
          </span>
          <div class="step-copy">
            <strong>{{ actionLabel(row.step.action) }}</strong>
            <span class="step-summary" :title="row.summary">{{ row.summary }}</span>
          </div>
          <div class="step-tail">
            <span
              v-if="runByIndex.get(row.index)"
              class="run-result"
              :class="runStepTone(runByIndex.get(row.index)?.outcome ?? '')"
              :title="statusLabel(runByIndex.get(row.index)?.outcome ?? '')"
            >
              {{ statusLabel(runByIndex.get(row.index)?.outcome ?? "") }}
              <span class="run-elapsed">{{ runByIndex.get(row.index)?.elapsedMs }}ms</span>
            </span>
            <span v-if="row.issue" class="issue-icon" :title="row.issue">
              <CircleAlert :size="14" />
            </span>
            <span
              class="drag-handle"
              :title="trDynamic('拖拽排序')"
              aria-hidden="true"
              @pointerdown="startDrag(row.index, $event)"
            >
              <GripVertical :size="14" />
            </span>
          </div>
        </div>
        <div v-if="row.index < rows.length - 1" class="step-insert">
          <button type="button" @click.stop="emit('insert', row.index + 1)">
            <Plus :size="12" />
            {{ tr("在此插入") }}
          </button>
        </div>
      </template>
    </div>
  </section>
</template>

<style scoped>
.steps-pane {
  padding: 16px;
}
.pane-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 8px;
  padding-bottom: 8px;
}
.pane-head h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 500;
}
.pane-head span {
  display: block;
  margin-top: 2px;
  color: var(--text-muted);
  font-size: 12px;
}
.step-toolbar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 2px;
  align-self: flex-end;
}
.step-list {
  display: flex;
  flex-direction: column;
}
.step-row {
  display: grid;
  grid-template-columns: 24px 28px minmax(0, 1fr) auto;
  gap: 8px;
  align-items: center;
  min-height: 48px;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px;
  cursor: pointer;
}
.step-row:hover {
  border-color: var(--border-strong);
  background: var(--bg-subtle);
}
.step-row:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.step-row.selected {
  border-color: var(--accent-soft-strong);
  background: var(--accent-soft);
}
.step-row.invalid .step-key {
  border-color: var(--warning);
}
.step-row.dragging {
  opacity: 0.5;
}
.step-row.drag-over {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.step-key {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  border: 1px solid var(--border);
  border-bottom-color: var(--border-strong);
  border-radius: 6px;
  background: var(--bg-raised);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
  font-weight: 500;
  box-shadow: 0 1px 0 var(--border);
}
.step-row.selected .step-key {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--text-on-accent);
}
.action-tile {
  display: grid;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 6px;
  color: var(--tile-ink, var(--text-muted));
  background: var(--tile-bg, var(--bg-subtle));
}
.action-tile.tone-green {
  --tile-ink: var(--success);
  --tile-bg: color-mix(in srgb, var(--success) 14%, transparent);
}
.action-tile.tone-blue {
  --tile-ink: var(--info);
  --tile-bg: color-mix(in srgb, var(--info) 14%, transparent);
}
.action-tile.tone-amber {
  --tile-ink: var(--warning);
  --tile-bg: color-mix(in srgb, var(--warning) 18%, transparent);
}
.action-tile.tone-violet {
  --tile-ink: #7c5cff;
  --tile-bg: rgba(124, 92, 255, 0.14);
}
.action-tile.tone-accent {
  --tile-ink: var(--accent);
  --tile-bg: var(--accent-soft);
}
.step-row.selected .action-tile {
  --tile-bg: var(--bg-raised);
}
.step-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.step-copy strong {
  overflow: hidden;
  font-size: 13px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.step-summary {
  overflow: hidden;
  color: var(--text-muted);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.step-tail {
  display: flex;
  align-items: center;
  gap: 4px;
}
.run-result {
  color: var(--text-muted);
  font-size: 11px;
  white-space: nowrap;
}
.run-result.done {
  color: var(--accent-ink);
}
.run-result.danger {
  color: var(--danger-text);
}
.run-result.warning {
  color: var(--warning-text);
}
.run-elapsed {
  margin-left: 4px;
  font-family: var(--font-mono);
}
.issue-icon {
  display: grid;
  place-items: center;
  color: var(--warning-text);
}
.drag-handle {
  display: grid;
  place-items: center;
  color: var(--text-muted);
  cursor: grab;
  opacity: 0.45;
  touch-action: none;
}
.drag-handle:hover {
  opacity: 0.9;
}
.step-row.dragging .drag-handle {
  cursor: grabbing;
  opacity: 1;
}
.step-insert {
  display: flex;
  height: 12px;
  align-items: center;
  padding-left: 68px;
}
.step-insert button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  border: 1px dashed transparent;
  border-radius: 6px;
  background: transparent;
  padding: 2px 6px;
  color: transparent;
  font-size: 11px;
}
.step-insert:hover button,
.step-insert button:focus-visible {
  border-color: var(--border);
  background: var(--bg-raised);
  color: var(--text-muted);
}
</style>
