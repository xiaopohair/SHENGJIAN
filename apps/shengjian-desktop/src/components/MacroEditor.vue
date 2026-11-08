<script setup lang="ts">
import { ref } from "vue";
import { ArrowDown, ArrowUp, Copy, Plus, Trash2 } from "lucide-vue-next";
import {
  createMacroStep,
  defaultMacroParameters,
  MACRO_ACTIONS,
  MACRO_MODIFIERS,
  type MacroDraft,
} from "../lib/macros";
import { tr, trDynamic } from "../lib/i18n";
import type { MacroChord, MacroDefinition, MacroStep } from "../lib/types";
import Button from "./ui/Button.vue";
import Card from "./ui/Card.vue";
import Checkbox from "./ui/Checkbox.vue";
import Field from "./ui/Field.vue";
import IconButton from "./ui/IconButton.vue";
import SectionHeader from "./ui/SectionHeader.vue";
import SelectInput from "./ui/SelectInput.vue";
import StatusPill from "./ui/StatusPill.vue";
import TextInput from "./ui/TextInput.vue";

const props = defineProps<{
  editingNew: boolean;
  busy: boolean;
  dirty: boolean;
  otherMacros: MacroDefinition[];
}>();

const emit = defineEmits<{
  save: [];
  cancel: [];
}>();

const draft = defineModel<MacroDraft>("draft", { required: true });
const dragStepIndex = ref<number | null>(null);

const ACTIONS = MACRO_ACTIONS;
const MODIFIERS = MACRO_MODIFIERS;

const KEY_LABELS: Record<string, string> = {
  space: "Space",
  enter: "Enter",
  tab: "Tab",
  backspace: "Backspace",
  delete: "Delete",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
};

function addStep() {
  draft.value.steps.push(createMacroStep());
}

function removeStep(index: number) {
  draft.value.steps.splice(index, 1);
}

function moveStep(index: number, delta: number) {
  const steps = draft.value.steps;
  const target = index + delta;
  if (target < 0 || target >= steps.length) return;
  const [step] = steps.splice(index, 1);
  steps.splice(target, 0, step);
}

function duplicateStep(index: number) {
  const source = draft.value.steps[index];
  if (!source) return;
  const copy = JSON.parse(JSON.stringify(source)) as MacroStep;
  copy.stepId = `step-${crypto.randomUUID()}`;
  draft.value.steps.splice(index + 1, 0, copy);
}

function dropStep(index: number) {
  const steps = draft.value.steps;
  const from = dragStepIndex.value;
  dragStepIndex.value = null;
  if (from === null || from === index) return;
  const [step] = steps.splice(from, 1);
  steps.splice(index, 0, step);
}

function changeAction(step: MacroStep, action: string) {
  step.action = action;
  step.parameters = defaultMacroParameters(action);
}

function setStepTimeout(step: MacroStep, value: string) {
  const timeout = Number(value);
  step.timeoutMs = Number.isFinite(timeout) && timeout > 0 ? timeout : null;
}

function chord(step: MacroStep): MacroChord {
  return step.parameters.chord ?? { modifiers: [], key: "" };
}

function toggleModifier(step: MacroStep, modifier: string, checked: boolean) {
  const current = chord(step);
  const modifiers = checked
    ? [...current.modifiers, modifier].filter((item, index, list) => list.indexOf(item) === index)
    : current.modifiers.filter((item) => item !== modifier);
  step.parameters.chord = { ...current, modifiers };
}

function setChordKey(step: MacroStep, key: string) {
  step.parameters.chord = { ...chord(step), key };
}

function setNestedMacro(step: MacroStep, macroId: string) {
  const target = props.otherMacros.find((definition) => definition.macroId === macroId);
  step.parameters.nestedMacroId = macroId;
  step.parameters.nestedMacroName = target?.name ?? macroId;
}

function chordLabel(value: MacroChord): string {
  const parts = value.modifiers
    .map((modifier) => MODIFIERS.find((item) => item.id === modifier)?.label ?? modifier)
    .filter(Boolean);
  const key =
    KEY_LABELS[value.key] ?? (value.key.length === 1 ? value.key.toUpperCase() : value.key);
  if (key) parts.push(key);
  return parts.join(" + ");
}
</script>

<template>
  <Card class="stack editor-card">
    <SectionHeader :title="editingNew ? '新建宏' : '编辑宏'">
      <template #actions>
        <StatusPill v-if="dirty" :label="trDynamic('未保存')" tone="warning" dot />
      </template>
    </SectionHeader>

    <Field label="名称">
      <TextInput v-model="draft.name" :maxlength="20" />
    </Field>
    <Field label="说明">
      <TextInput v-model="draft.summary" :maxlength="300" />
    </Field>
    <p class="muted version-note">
      {{
        editingNew
          ? tr("新宏版本 v1.0.0")
          : tr("保存后追加为新版本 v{version}", { version: draft.version })
      }}
    </p>

    <div class="steps">
      <div class="steps-head">
        <h3>{{ tr("步骤") }}</h3>
        <Button size="sm" @click="addStep">
          <Plus :size="14" />
          {{ tr("添加步骤") }}
        </Button>
      </div>

      <div
        v-for="(step, index) in draft.steps"
        :key="step.stepId"
        class="step-card"
        :class="{ dragging: dragStepIndex === index }"
        @dragover.prevent
        @drop="dropStep(index)"
      >
        <div class="step-head">
          <span
            class="drag-handle"
            draggable="true"
            :title="trDynamic('拖拽排序')"
            aria-hidden="true"
            @dragstart="dragStepIndex = index"
            @dragend="dragStepIndex = null"
          >
            ⋮⋮
          </span>
          <span class="step-index mono muted">#{{ index + 1 }}</span>
          <SelectInput
            class="step-action"
            :model-value="step.action"
            :aria-label="`${trDynamic('步骤')} #${index + 1}`"
            @update:model-value="changeAction(step, $event)"
          >
            <option v-for="action in ACTIONS" :key="action.id" :value="action.id">
              {{ trDynamic(action.label) }}
            </option>
          </SelectInput>
          <div class="row-actions step-actions">
            <IconButton
              size="sm"
              :label="trDynamic('上移步骤')"
              :disabled="index === 0"
              @click="moveStep(index, -1)"
            >
              <ArrowUp :size="14" />
            </IconButton>
            <IconButton
              size="sm"
              :label="trDynamic('下移步骤')"
              :disabled="index === draft.steps.length - 1"
              @click="moveStep(index, 1)"
            >
              <ArrowDown :size="14" />
            </IconButton>
            <IconButton size="sm" :label="trDynamic('复制步骤')" @click="duplicateStep(index)">
              <Copy :size="14" />
            </IconButton>
            <IconButton
              size="sm"
              variant="danger"
              :label="trDynamic('删除步骤')"
              @click="removeStep(index)"
            >
              <Trash2 :size="14" />
            </IconButton>
          </div>
        </div>

        <div class="step-body">
          <Field
            v-if="['open_application', 'focus_input', 'wait_for_application'].includes(step.action)"
            label="应用目标"
          >
            <TextInput
              :model-value="step.parameters.target ?? ''"
              placeholder="notepad.exe"
              @update:model-value="step.parameters.target = $event"
            />
          </Field>
          <Field v-if="step.action === 'wait_for_application'" label="等待状态">
            <SelectInput
              :model-value="step.parameters.state ?? 'running'"
              @update:model-value="step.parameters.state = $event"
            >
              <option value="running">{{ tr("进程运行") }}</option>
              <option value="visible_window">{{ tr("窗口可见") }}</option>
            </SelectInput>
          </Field>
          <Field v-if="step.action === 'open_url'" label="网址">
            <TextInput
              :model-value="step.parameters.url ?? ''"
              placeholder="https://example.com"
              @update:model-value="step.parameters.url = $event"
            />
          </Field>
          <div v-if="['send_shortcut', 'system_action'].includes(step.action)" class="chord-editor">
            <span class="field-label">{{ tr("快捷键") }}</span>
            <div class="chord-controls">
              <div class="modifier-row">
                <Checkbox
                  v-for="modifier in MODIFIERS"
                  :key="modifier.id"
                  :model-value="chord(step).modifiers.includes(modifier.id)"
                  @update:model-value="toggleModifier(step, modifier.id, $event)"
                >
                  {{ modifier.label }}
                </Checkbox>
                <TextInput
                  class="key-input"
                  placeholder="n"
                  :aria-label="trDynamic('快捷键按键')"
                  :model-value="chord(step).key"
                  @update:model-value="setChordKey(step, $event)"
                />
              </div>
              <span v-if="chordLabel(chord(step))" class="chord-preview mono muted">
                {{ chordLabel(chord(step)) }}
              </span>
            </div>
          </div>
          <Field v-if="step.action === 'run_macro'" label="目标宏">
            <SelectInput
              :model-value="step.parameters.nestedMacroId ?? ''"
              @update:model-value="setNestedMacro(step, $event)"
            >
              <option value="">{{ tr("选择宏") }}</option>
              <option
                v-for="definition in otherMacros"
                :key="definition.macroId"
                :value="definition.macroId"
              >
                {{ definition.name }} · v{{ definition.version }}
              </option>
            </SelectInput>
          </Field>
          <Field label="步骤超时 (ms)">
            <TextInput
              type="number"
              min="100"
              :max="step.action === 'run_macro' ? 60000 : 10000"
              step="100"
              :placeholder="trDynamic('默认')"
              :model-value="step.timeoutMs == null ? '' : String(step.timeoutMs)"
              @update:model-value="setStepTimeout(step, $event)"
            />
          </Field>
        </div>
      </div>
    </div>

    <div class="editor-actions editor-footer">
      <span class="editor-state" :class="{ 'is-dirty': dirty }">
        {{ dirty ? tr("有未保存的修改") : tr("没有未保存的修改") }}
      </span>
      <div class="footer-actions">
        <Button :disabled="busy" @click="emit('cancel')">{{ tr("取消") }}</Button>
        <Button variant="primary" :loading="busy" @click="emit('save')">
          {{ trDynamic(editingNew ? "保存宏" : "保存新版本") }}
        </Button>
      </div>
    </div>
  </Card>
</template>

<style scoped>
.editor-card {
  min-width: 0;
}

.version-note {
  font-size: var(--text-xs);
}

.steps {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  padding-top: var(--space-4);
}

.steps-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.steps-head h3 {
  font-size: var(--text-base);
  font-weight: 650;
}

.step-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  background: var(--bg-subtle);
}

.step-card.dragging {
  border-color: var(--accent);
  opacity: 0.7;
}

.step-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.drag-handle {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  letter-spacing: -1px;
  cursor: grab;
  user-select: none;
}

.step-index {
  flex: 0 0 auto;
}

.step-head .step-action {
  flex: 1 1 160px;
  min-width: 140px;
  max-width: 320px;
}

.step-actions {
  margin-left: auto;
}

.step-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.chord-editor {
  display: grid;
  grid-template-columns: minmax(140px, 200px) minmax(0, 1fr);
  align-items: center;
  gap: var(--space-2) var(--space-3);
}

.field-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.chord-controls {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.modifier-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
}

.modifier-row .key-input {
  flex: 0 0 90px;
  width: 90px;
}

.chord-preview {
  font-size: var(--text-xs);
}

.editor-actions.editor-footer {
  margin-top: var(--space-2);
  background: var(--bg-raised);
  padding-bottom: var(--space-2);
}

.editor-state {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.editor-state.is-dirty {
  color: var(--warning-text);
  font-weight: 600;
}

@media (max-width: 900px) {
  .chord-editor {
    grid-template-columns: 1fr;
  }
}
</style>
