<script setup lang="ts">
import { computed } from "vue";
import { CircleAlert, CircleCheck, Copy, Trash2 } from "lucide-vue-next";
import { defaultMacroParameters } from "../../lib/macros";
import { tr, trDynamic } from "../../lib/i18n";
import { ACTIONS, actionIcon, actionLabel, actionUsesTimeout, chord } from "../../lib/macroEditor";
import { systemActionById, systemActionIdForChord } from "../../lib/systemActions";
import type { MacroChord, MacroDefinition, MacroParameters, MacroStep } from "../../lib/types";
import { useAppPicker } from "../../composables/useAppPicker";
import AppPickerDialog from "../app-picker/AppPickerDialog.vue";
import AppTargetField from "../app-picker/AppTargetField.vue";
import IconButton from "../ui/IconButton.vue";
import SelectInput from "../ui/SelectInput.vue";
import SystemActionGrid from "../ui/SystemActionGrid.vue";
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

/** 动作卡片的补充说明：与按键方案的动作卡一致。 */
const ACTION_DETAILS: Record<string, string> = {
  send_shortcut: "键盘组合",
  open_application: "启动或切换应用",
  open_url: "用默认浏览器打开",
  focus_input: "把光标送回输入框",
  focus_frontmost: "聚焦当前前台输入框",
  wait_for_application: "等待进程或窗口",
  system_action: "音量与播放控制",
  run_macro: "执行另一个宏",
};

/** 动作图标颜色：与按键方案同一套色板。 */
const ACTION_TONES: Record<string, string> = {
  send_shortcut: "teal",
  open_application: "amber",
  open_url: "teal",
  focus_input: "teal",
  focus_frontmost: "teal",
  wait_for_application: "neutral",
  system_action: "neutral",
  run_macro: "violet",
};

function actionCardTone(actionId: string): string {
  return ACTION_TONES[actionId] ?? "neutral";
}

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

const {
  open: appPickerOpen,
  openPicker: openProgramPicker,
  closePicker: closeProgramPicker,
} = useAppPicker();

/** The picker always returns a canonical target (preset id / absolute path / AUMID). */
function applyPickedProgram(target: string) {
  patchParameters({ target });
  closeProgramPicker();
}
const chordValue = computed<MacroChord>(() =>
  props.step ? chord(props.step) : { modifiers: [], key: "" },
);
/** 宏里存的是真实组合键（返回桌面=Win+D 等），选中态需映射回系统动作 id。 */
const systemActionId = computed(() => systemActionIdForChord(chordValue.value));

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

/** 选中系统动作卡片：写入该动作的真实 chord（单键或组合键）。 */
function setSystemAction(id: string) {
  const definition = systemActionById(id);
  if (definition) patchParameters({ chord: { ...definition.chord } });
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
          :class="[`tone-${actionCardTone(action.id)}`, { selected: step.action === action.id }]"
          :aria-pressed="step.action === action.id"
          @click="changeAction(action.id)"
        >
          <span class="action-ico" aria-hidden="true">
            <component :is="actionIcon(action.id)" :size="16" />
          </span>
          <span class="action-copy">
            <strong>{{ trDynamic(action.label) }}</strong>
            <small>{{ trDynamic(ACTION_DETAILS[action.id] ?? "") }}</small>
          </span>
        </button>
      </div>

      <div v-if="step" class="action-config">
        <h4 class="config-head">{{ tr("动作参数") }}</h4>

        <div v-if="showTargetField" class="config-row">
          <div class="config-field">
            <label for="macro-step-target">{{ tr("应用目标") }}</label>
            <AppTargetField
              input-id="macro-step-target"
              :target="step.parameters.target ?? ''"
              :aria-label="tr('应用目标')"
              @choose="openProgramPicker"
            />
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
          <SystemActionGrid :model-value="systemActionId" @update:model-value="setSystemAction" />
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

    <AppPickerDialog
      :open="appPickerOpen"
      mode="launch"
      :current-target="step?.parameters.target ?? ''"
      @select="applyPickedProgram"
      @close="closeProgramPicker"
    />
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
  align-items: flex-start;
  gap: 8px;
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: 8px 10px;
  color: var(--text);
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}
.action-option:hover {
  border-color: var(--accent);
  background: var(--bg-hover);
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
  flex: 0 0 26px;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  color: var(--text-muted);
}
.action-option.tone-violet .action-ico {
  color: var(--violet-text, var(--accent));
}
.action-option.tone-amber .action-ico {
  color: var(--warning-text);
}
.action-option.tone-danger .action-ico {
  color: var(--danger-text);
}
.action-option.selected .action-ico {
  background: var(--accent-soft-strong);
  color: var(--accent);
}
.action-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.action-copy strong {
  color: var(--text);
  font-size: var(--text-xs);
  font-weight: 650;
  line-height: 1.35;
}
.action-option.selected .action-copy strong {
  color: var(--accent-ink);
}
.action-copy small {
  overflow: hidden;
  margin-top: 2px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.35;
  text-overflow: ellipsis;
  white-space: nowrap;
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
/* Rows and standalone fields must consume identical vertical space before the
   hint row below; otherwise switching action types moves the hint up/down by 4px
   (system_action is a single field, run_macro is a .config-row). */
.config-row {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 12px;
  margin-bottom: 8px;
}
.config-row .config-field {
  margin-bottom: 0;
}
/* The first field takes the remaining width; the rest keep their own size. */
.config-row > .config-field:first-child {
  flex: 1 1 200px;
  min-width: 0;
}
.config-wait {
  flex: 0 0 150px;
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
/* Keep 4 per row: the panel is ~600px wide at the minimum window width, which
   still fits four 145px cards without clipping their labels. Only fall back to
   two columns below the supported minimum. */
@media (max-width: 1100px) {
  .action-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
