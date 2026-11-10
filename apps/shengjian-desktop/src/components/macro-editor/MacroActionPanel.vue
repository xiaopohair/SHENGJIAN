<script setup lang="ts">
import { computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { CircleAlert, CircleCheck, Copy, Trash2 } from "lucide-vue-next";
import { defaultMacroParameters } from "../../lib/macros";
import { tr, trDynamic } from "../../lib/i18n";
import { RUNNABLE_EXTENSIONS } from "../../lib/runnable";
import {
  ACTIONS,
  SYSTEM_KEYS,
  actionIcon,
  actionLabel,
  actionTone,
  actionUsesTimeout,
  chord,
} from "../../lib/macroEditor";
import type { MacroChord, MacroDefinition, MacroParameters, MacroStep } from "../../lib/types";
import Button from "../ui/Button.vue";
import IconButton from "../ui/IconButton.vue";
import SelectInput from "../ui/SelectInput.vue";
import TextInput from "../ui/TextInput.vue";
import MacroChordInput from "../ui/ChordInput.vue";

const props = defineProps<{
  step: MacroStep | null;
  issue: string;
  index: number;
  total: number;
  otherMacros: MacroDefinition[];
}>();

const emit = defineEmits<{
  "update-step": [step: MacroStep];
  duplicate: [];
  remove: [];
}>();

const showTargetField = computed(
  () =>
    props.step !== null &&
    ["open_application", "focus_input", "wait_for_application"].includes(props.step.action),
);
const showWaitField = computed(() => props.step?.action === "wait_for_application");
const showUrlField = computed(() => props.step?.action === "open_url");
const showChordField = computed(() => props.step?.action === "send_shortcut");
const showSystemField = computed(() => props.step?.action === "system_action");
const showNestedField = computed(() => props.step?.action === "run_macro");
const showTimeoutField = computed(
  () => props.step !== null && actionUsesTimeout(props.step.action),
);
const maxTimeout = computed(() => (props.step?.action === "run_macro" ? 60000 : 10000));
/** Actions such as focus_frontmost take no parameters at all. */
const hasParameters = computed(
  () =>
    showTargetField.value ||
    showUrlField.value ||
    showChordField.value ||
    showSystemField.value ||
    showNestedField.value ||
    showTimeoutField.value,
);
/** Launching or focusing an app benefits from picking the executable. */
const showProgramPicker = computed(
  () => props.step?.action === "open_application" || props.step?.action === "focus_input",
);

async function pickProgram() {
  try {
    const selected = await open({
      title: tr("选择程序"),
      multiple: false,
      filters: [{ name: tr("可运行文件"), extensions: RUNNABLE_EXTENSIONS }],
    });
    if (!selected || Array.isArray(selected)) return;
    // Store the full path: a bare executable name may not be on PATH, which is
    // what made "open application" fail for programs outside the presets.
    patchParameters({ target: selected });
  } catch {
    // A failed picker leaves the current target untouched.
  }
}
const chordValue = computed<MacroChord>(() =>
  props.step ? chord(props.step) : { modifiers: [], key: "" },
);

function patch(next: Partial<MacroStep>) {
  if (!props.step) return;
  emit("update-step", { ...props.step, ...next });
}

function patchParameters(parameters: Partial<MacroParameters>) {
  if (!props.step) return;
  patch({ parameters: { ...props.step.parameters, ...parameters } });
}

function changeAction(action: string) {
  if (!props.step) return;
  emit("update-step", {
    ...props.step,
    action,
    parameters: defaultMacroParameters(action),
    // Actions that never wait do not carry a timeout.
    timeoutMs: actionUsesTimeout(action) ? props.step.timeoutMs : null,
  });
}

function setStepTimeout(value: string) {
  const timeout = Number(value);
  patch({ timeoutMs: Number.isFinite(timeout) && timeout > 0 ? timeout : null });
}

function setChord(next: MacroChord) {
  patchParameters({ chord: next });
}

/** System actions never carry modifiers; the runtime maps the key name alone. */
function setSystemKey(key: string) {
  patchParameters({ chord: { modifiers: [], key } });
}

function setNestedMacro(macroId: string) {
  const target = props.otherMacros.find((definition) => definition.macroId === macroId);
  patchParameters({
    nestedMacroId: macroId,
    nestedMacroName: target?.name ?? macroId,
  });
}
</script>

<template>
  <section class="macro-panel action-pane">
    <template v-if="step">
      <div class="action-head">
        <div class="action-title">
          <span class="action-kicker">{{ tr("当前步骤") }}</span>
          <h3>
            {{ actionLabel(step.action) }}
            <span class="step-position">#{{ index + 1 }} / {{ total }}</span>
          </h3>
        </div>
        <div class="action-head-actions">
          <IconButton size="sm" :label="trDynamic('复制步骤')" @click="emit('duplicate')">
            <Copy :size="14" />
          </IconButton>
          <IconButton
            size="sm"
            variant="danger"
            :label="trDynamic('删除步骤')"
            @click="emit('remove')"
          >
            <Trash2 :size="14" />
          </IconButton>
        </div>
      </div>

      <div class="action-grid-label">{{ tr("选择动作") }}</div>
      <div class="action-grid" role="group" :aria-label="tr('选择动作')">
        <button
          v-for="action in ACTIONS"
          :key="action.id"
          type="button"
          class="action-option"
          :class="{ selected: step.action === action.id }"
          :aria-pressed="step.action === action.id"
          @click="changeAction(action.id)"
        >
          <span class="action-ico" :class="`tone-${actionTone(action.id)}`" aria-hidden="true">
            <component :is="actionIcon(action.id)" :size="16" />
          </span>
          <strong>{{ trDynamic(action.label) }}</strong>
        </button>
      </div>

      <div v-if="hasParameters" class="action-config">
        <h4 class="config-head">{{ tr("动作参数") }}</h4>

        <div v-if="showTargetField" class="config-row">
          <div class="config-field">
            <label for="macro-step-target">{{ tr("应用目标") }}</label>
            <div class="target-row">
              <TextInput
                id="macro-step-target"
                :model-value="step.parameters.target ?? ''"
                :placeholder="trDynamic('选择或填写应用')"
                @update:model-value="patchParameters({ target: $event })"
              />
              <Button v-if="showProgramPicker" size="sm" @click="pickProgram">
                {{ tr("选择程序") }}
              </Button>
            </div>
          </div>
          <div v-if="showWaitField" class="config-field config-wait">
            <label>{{ tr("启动成功标志") }}</label>
            <SelectInput
              :model-value="step.parameters.state ?? 'running'"
              @update:model-value="patchParameters({ state: $event })"
            >
              <option value="running">{{ tr("进程运行") }}</option>
              <option value="visible_window">{{ tr("窗口可见") }}</option>
            </SelectInput>
          </div>
          <div v-if="showTimeoutField" class="config-field config-timeout">
            <label>{{ tr("步骤超时 (ms)") }}</label>
            <TextInput
              type="number"
              min="100"
              :max="maxTimeout"
              step="100"
              :placeholder="trDynamic('默认')"
              :model-value="step.timeoutMs == null ? '' : String(step.timeoutMs)"
              @update:model-value="setStepTimeout"
            />
          </div>
        </div>

        <div v-if="showUrlField" class="config-field">
          <label>{{ tr("网址") }}</label>
          <TextInput
            :model-value="step.parameters.url ?? ''"
            placeholder="https://example.com"
            @update:model-value="patchParameters({ url: $event })"
          />
        </div>
        <div v-if="showChordField" class="config-field">
          <label>{{ tr("快捷键") }}</label>
          <MacroChordInput :model-value="chordValue" @update:model-value="setChord" />
        </div>
        <div v-if="showSystemField" class="config-field">
          <label>{{ tr("系统动作") }}</label>
          <SelectInput :model-value="chordValue.key" @update:model-value="setSystemKey">
            <option value="">{{ tr("选择系统动作") }}</option>
            <option v-for="item in SYSTEM_KEYS" :key="item.key" :value="item.key">
              {{ trDynamic(item.label) }}
            </option>
          </SelectInput>
        </div>
        <div v-if="showNestedField" class="config-row">
          <div class="config-field">
            <label>{{ tr("目标宏") }}</label>
            <SelectInput
              :model-value="step.parameters.nestedMacroId ?? ''"
              @update:model-value="setNestedMacro"
            >
              <option value="">{{ tr("选择宏") }}</option>
              <option
                v-for="definition in otherMacros"
                :key="definition.macroId"
                :value="definition.macroId"
              >
                {{ definition.name }}
              </option>
            </SelectInput>
          </div>
          <div v-if="showTimeoutField" class="config-field config-timeout">
            <label>{{ tr("步骤超时 (ms)") }}</label>
            <TextInput
              type="number"
              min="100"
              :max="maxTimeout"
              step="100"
              :placeholder="trDynamic('默认')"
              :model-value="step.timeoutMs == null ? '' : String(step.timeoutMs)"
              @update:model-value="setStepTimeout"
            />
          </div>
        </div>

        <p v-if="issue" class="config-issue" role="alert">
          <CircleAlert :size="13" />
          {{ issue }}
        </p>
        <p v-else class="config-note">
          <CircleCheck :size="13" />
          {{ tr("参数完整，可以试运行。") }}
        </p>
      </div>
    </template>

    <div v-else class="action-empty">
      <CircleAlert :size="18" />
      <strong>{{ tr("还没有步骤") }}</strong>
      <span>{{ tr("先添加一个步骤，再选择动作。") }}</span>
    </div>
  </section>
</template>

<style scoped>
.action-pane {
  padding: 16px;
}
.action-head {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
  border-bottom: 1px solid var(--border);
  padding-bottom: 12px;
}
.action-title {
  min-width: 0;
}
.action-head-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: flex-end;
  gap: 4px;
}
.action-kicker {
  color: var(--text-muted);
  font-size: 12px;
}
.action-head h3 {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin: 2px 0 0;
  font-size: 20px;
  font-weight: 500;
  line-height: 1.25;
  letter-spacing: -0.015em;
}
.step-position {
  margin-left: 8px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 12px;
  font-weight: 400;
  letter-spacing: 0;
}
.action-head p {
  margin: 0;
  color: var(--text-muted);
  font-size: 12px;
}
.action-grid-label {
  margin: 12px 0 8px;
  color: var(--text-muted);
  font-size: 12px;
}
.action-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}
.action-option {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
  min-height: 72px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-raised);
  padding: 8px;
  color: var(--text-muted);
  text-align: left;
}
.action-option:hover {
  border-color: var(--border-strong);
  background: var(--bg-subtle);
}
.action-option:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
.action-option.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.action-ico {
  display: grid;
  place-items: center;
  width: 24px;
  height: 24px;
  color: var(--tile-ink, var(--text-muted));
}
.action-ico.tone-green {
  --tile-ink: var(--success);
}
.action-ico.tone-blue {
  --tile-ink: var(--info);
}
.action-ico.tone-amber {
  --tile-ink: var(--warning);
}
.action-ico.tone-violet {
  --tile-ink: #7c5cff;
}
.action-ico.tone-accent {
  --tile-ink: var(--accent);
}
.action-option.selected .action-ico {
  color: var(--accent);
}
.action-option strong {
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  line-height: 1.35;
}
.action-option.selected strong {
  color: var(--accent-ink);
}
.action-config {
  border-top: 1px solid var(--border);
  margin-top: 16px;
  padding-top: 12px;
}
.config-head {
  margin: 0 0 8px;
  font-size: 13px;
  font-weight: 500;
}
.config-row {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 12px;
}
.config-row .config-field {
  margin-bottom: 8px;
}
/* The first field takes the remaining width; the rest keep their own size. */
.config-row > .config-field:first-child {
  flex: 1 1 200px;
  min-width: 0;
}
.config-wait {
  flex: 0 0 150px;
}
.target-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.config-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-bottom: 8px;
}
.config-field label {
  color: var(--text-muted);
  font-size: 12px;
}
.config-timeout {
  flex: 0 0 140px;
}
.config-issue,
.config-note {
  display: flex;
  align-items: flex-start;
  gap: 4px;
  margin: 4px 0 0;
  font-size: 12px;
  line-height: 1.5;
}
.config-issue {
  color: var(--warning-text);
}
.config-note {
  color: var(--text-muted);
}
.action-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 240px;
  gap: 6px;
  color: var(--text-muted);
  text-align: center;
}
.action-empty strong {
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 500;
}
.action-empty span {
  font-size: 12px;
}
@media (max-width: 1120px) {
  .action-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
@media (max-width: 980px) {
  .action-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}
@media (max-width: 620px) {
  .action-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
