<script setup lang="ts">
import { computed } from "vue";
import type { MacroDefinition, MacroRunStatus } from "../lib/types";
import type { MacroDraft } from "../lib/macros";
import { useMacroDraftEditor } from "../composables/useMacroDraftEditor";
import MacroActionPanel from "./macro-editor/MacroActionPanel.vue";
import MacroStepList from "./macro-editor/MacroStepList.vue";

/**
 * 宏编辑器主体：步骤列表 + 动作面板。
 *
 * 编辑工具栏（保存/试运行/属性/复制/删除/取消）已整体删除，这些动作统一在
 * 页面导航栏（宏页工具栏）里，避免同一批按钮出现在两处。
 */
const props = defineProps<{
  otherMacros: MacroDefinition[];
  runStatus?: MacroRunStatus | null;
}>();

const draft = defineModel<MacroDraft>("draft", { required: true });

const {
  selectedStepId,
  selectedStep,
  selectedStepIndex,
  stepRows,
  issues,
  selectStep,
  addStep,
  insertStep,
  removeStep,
  moveStep,
  duplicateStep,
  dropStep,
  updateStep,
} = useMacroDraftEditor(draft);

const runReport = computed(() => props.runStatus?.report ?? null);
const selectedIssue = computed(
  () => issues.value.find((item) => item.index === selectedStepIndex.value)?.issue ?? "",
);

function selectByIndex(index: number) {
  const step = stepRows.value[index]?.step;
  if (step) selectStep(step);
}

function reorder(from: number, to: number) {
  dropStep(from, to);
}

function removeSelected() {
  if (selectedStep.value) removeStep(selectedStep.value.stepId);
}
</script>

<template>
  <div class="macro-editor">
    <div class="editor-grid">
      <MacroStepList
        :rows="stepRows"
        :selected-step-id="selectedStepId"
        :run-steps="runReport?.steps ?? []"
        @select="selectByIndex"
        @add="addStep"
        @insert="insertStep"
        @move="selectedStep && moveStep(selectedStep.stepId, $event)"
        @reorder="reorder"
      />

      <MacroActionPanel
        :step="selectedStep"
        :issue="selectedIssue"
        :index="selectedStepIndex"
        :total="draft.steps.length"
        :other-macros="otherMacros"
        @update-step="selectedStep && updateStep(selectedStep.stepId, $event)"
        @duplicate="selectedStep && duplicateStep(selectedStep.stepId)"
        @remove="removeSelected"
      />
    </div>
  </div>
</template>

<style scoped>
.macro-editor {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  min-width: 0;
}
.editor-grid {
  display: grid;
  grid-template-columns: 300px minmax(0, 1fr);
  gap: 12px;
  align-items: stretch;
}
.macro-panel {
  min-width: 0;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-sm);
}
@media (max-width: 1320px) {
  .editor-grid {
    grid-template-columns: 276px minmax(0, 1fr);
  }
}
/* Layout stays multi-column down to the app minimum window width (1200px);
   the content area is ~960px there once the 208px sidebar is subtracted.
   These rules only apply below the supported minimum, as a safety net. */
@media (max-width: 1100px) {
  .editor-grid {
    grid-template-columns: 1fr;
  }
}
</style>
