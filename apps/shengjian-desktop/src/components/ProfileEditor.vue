<script setup lang="ts">
import { computed } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-vue-next";
import { errorMessage, resolveAppTarget } from "../lib/api";
import {
  PROFILE_BUTTONS,
  PROFILE_TRIGGERS,
  type ProfileActionType,
  type ProfileDraft,
  type ProfileTrigger,
} from "../lib/buttons";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { RUNNABLE_EXTENSIONS } from "../lib/runnable";
import type { LogicalAction, LogicalButton, MacroChord, MacroDefinition } from "../lib/types";
import Button from "./ui/Button.vue";
import Card from "./ui/Card.vue";
import Checkbox from "./ui/Checkbox.vue";
import Field from "./ui/Field.vue";
import IconButton from "./ui/IconButton.vue";
import SectionHeader from "./ui/SectionHeader.vue";
import SelectInput from "./ui/SelectInput.vue";
import StatusPill from "./ui/StatusPill.vue";
import TextInput from "./ui/TextInput.vue";
import ChordInput from "./ui/ChordInput.vue";

const MODIFIER_IDS = new Set(["control", "alt", "shift", "win"]);

const props = defineProps<{
  editingNew: boolean;
  busy: boolean;
  dirty: boolean;
  baselineJson: string;
  foregroundApp: string;
  commonPhrasesAvailable: boolean;
  latestMacros: MacroDefinition[];
}>();

const emit = defineEmits<{
  save: [];
  cancel: [];
  error: [message: string];
}>();

const draft = defineModel<ProfileDraft>("draft", { required: true });

const TRIGGERS = PROFILE_TRIGGERS;
type Trigger = ProfileTrigger;
type ActionType = ProfileActionType;

const holdRepeatModel = computed({
  get: () => draft.value.holdRepeat,
  set: (value: string) => {
    draft.value.holdRepeat = value as ProfileDraft["holdRepeat"];
  },
});

const modifiedRows = computed(() => {
  const rows = new Set<string>();
  if (!props.baselineJson) return rows;
  let baseline: ProfileDraft;
  try {
    baseline = JSON.parse(props.baselineJson) as ProfileDraft;
  } catch {
    return rows;
  }
  for (const button of PROFILE_BUTTONS) {
    const current = JSON.stringify(draft.value.bindings[button.id] ?? null);
    const before = JSON.stringify(baseline.bindings?.[button.id] ?? null);
    if (current !== before) rows.add(button.id);
  }
  return rows;
});

const modifiedCount = computed(() => modifiedRows.value.size);

function isRowModified(buttonId: string): boolean {
  return modifiedRows.value.has(buttonId);
}

function actionFor(button: LogicalButton, trigger: Trigger): LogicalAction | undefined {
  return draft.value.bindings[button]?.[trigger];
}

function actionType(button: LogicalButton, trigger: Trigger): ActionType {
  const action = actionFor(button, trigger);
  if (!action) return "inherit";
  if (
    action.type === "disabled" ||
    action.type === "focus-input" ||
    action.type === "shortcut" ||
    action.type === "open-app" ||
    action.type === "common-phrase" ||
    action.type === "mouse" ||
    action.type === "run-macro"
  ) {
    return action.type;
  }
  return "other";
}

function ensureButton(button: LogicalButton): Partial<Record<Trigger, LogicalAction>> {
  draft.value.bindings[button] ??= {};
  return draft.value.bindings[button];
}

function setActionType(button: LogicalButton, trigger: Trigger, value: string) {
  const bindings = ensureButton(button);
  if (value === "inherit") {
    delete bindings[trigger];
    return;
  }
  if (value === "other") return;
  if (value === "disabled") {
    bindings[trigger] = { type: "disabled" };
  } else if (value === "focus-input") {
    bindings[trigger] = { type: "focus-input" };
  } else if (value === "shortcut") {
    bindings[trigger] = { type: "shortcut", label: "", keys: [] };
  } else if (value === "open-app") {
    bindings[trigger] = { type: "open-app", app: "" };
  } else if (value === "common-phrase") {
    bindings[trigger] = { type: "common-phrase" };
  } else if (value === "mouse") {
    bindings[trigger] = {
      type: "mouse",
      kind: "scroll",
      direction: "down",
      amount: 3,
    };
  } else if (value === "run-macro") {
    bindings[trigger] = {
      type: "run-macro",
      macro_id: props.latestMacros[0]?.macroId ?? "",
    };
  }
}

function shortcutChord(button: LogicalButton, trigger: Trigger): MacroChord {
  const action = actionFor(button, trigger);
  const keys = action?.type === "shortcut" && Array.isArray(action.keys) ? action.keys : [];
  const modifiers: string[] = [];
  let key = "";
  for (const entry of keys) {
    if (MODIFIER_IDS.has(entry)) {
      modifiers.push(entry);
    } else {
      key = entry;
    }
  }
  return { modifiers, key };
}

function setShortcut(button: LogicalButton, trigger: Trigger, value: string) {
  const keys = value
    .split("+")
    .map((key) => key.trim())
    .filter(Boolean);
  ensureButton(button)[trigger] = {
    type: "shortcut",
    label: keys.join("+"),
    keys,
  };
}

function setShortcutChord(button: LogicalButton, trigger: Trigger, chord: MacroChord) {
  setShortcut(button, trigger, [...chord.modifiers, chord.key].filter(Boolean).join("+"));
}

function appValue(button: LogicalButton, trigger: Trigger): string {
  const action = actionFor(button, trigger);
  return action?.type === "open-app" && typeof action.app === "string" ? action.app : "";
}

function setApp(button: LogicalButton, trigger: Trigger, value: string) {
  ensureButton(button)[trigger] = { type: "open-app", app: value.trim() };
}

function mouseKind(button: LogicalButton, trigger: Trigger): string {
  const action = actionFor(button, trigger);
  return action?.type === "mouse" && typeof action.kind === "string" ? action.kind : "scroll";
}

function mouseDirection(button: LogicalButton, trigger: Trigger): string {
  const action = actionFor(button, trigger);
  return action?.type === "mouse" && typeof action.direction === "string"
    ? action.direction
    : "down";
}

function mouseAmount(button: LogicalButton, trigger: Trigger): number {
  const action = actionFor(button, trigger);
  return action?.type === "mouse" && typeof action.amount === "number" ? action.amount : 3;
}

function setMouseKind(button: LogicalButton, trigger: Trigger, kind: string) {
  ensureButton(button)[trigger] = {
    type: "mouse",
    kind,
    direction: kind === "click" ? null : mouseDirection(button, trigger),
    amount: kind === "click" ? 0 : mouseAmount(button, trigger),
  };
}

function setMouseDirection(button: LogicalButton, trigger: Trigger, direction: string) {
  ensureButton(button)[trigger] = {
    type: "mouse",
    kind: mouseKind(button, trigger),
    direction,
    amount: mouseAmount(button, trigger),
  };
}

function setMouseAmount(button: LogicalButton, trigger: Trigger, value: string) {
  const amount = Math.max(1, Math.min(100, Number(value) || 1));
  ensureButton(button)[trigger] = {
    type: "mouse",
    kind: mouseKind(button, trigger),
    direction: mouseDirection(button, trigger),
    amount,
  };
}

function macroIdFor(button: LogicalButton, trigger: Trigger): string {
  const action = actionFor(button, trigger);
  return action?.type === "run-macro" && typeof action.macro_id === "string" ? action.macro_id : "";
}

function setMacro(button: LogicalButton, trigger: Trigger, macroId: string) {
  ensureButton(button)[trigger] = { type: "run-macro", macro_id: macroId };
}

function useForegroundTarget() {
  if (!props.foregroundApp) return;
  const targets = draft.value.appTargets
    .split(",")
    .map((target) => target.trim())
    .filter(Boolean);
  if (!targets.includes(props.foregroundApp)) targets.push(props.foregroundApp);
  draft.value.appTargets = targets.join(", ");
}

function executableName(path: string): string {
  return path.split(/[\\/]/).pop() ?? "";
}

async function pickRunnable(): Promise<string | null> {
  const selected = await open({
    title: tr("选择程序"),
    multiple: false,
    filters: [{ name: tr("可运行文件"), extensions: RUNNABLE_EXTENSIONS }],
  });
  return !selected || Array.isArray(selected) ? null : selected;
}

async function chooseAppTarget() {
  try {
    const path = await pickRunnable();
    if (!path) return;
    const resolved = await resolveAppTarget(path).catch(() => null);
    const name = resolved || executableName(path);
    if (!name) return;
    const targets = draft.value.appTargets
      .split(",")
      .map((target) => target.trim())
      .filter(Boolean);
    if (!targets.some((target) => target.toLowerCase() === name.toLowerCase())) {
      targets.push(name);
    }
    draft.value.appTargets = targets.join(", ");
  } catch (cause) {
    emit("error", errorMessage(cause));
  }
}

async function chooseAppAction(button: LogicalButton, trigger: Trigger) {
  try {
    const path = await pickRunnable();
    if (!path) return;
    setApp(button, trigger, path);
  } catch (cause) {
    emit("error", errorMessage(cause));
  }
}
</script>

<template>
  <Card class="stack editor-card">
    <SectionHeader :title="editingNew ? '新建方案' : '编辑方案'">
      <template #actions>
        <StatusPill v-if="dirty" :label="trDynamic('未保存')" tone="warning" dot />
      </template>
    </SectionHeader>

    <Field label="名称">
      <TextInput v-model="draft.name" :maxlength="80" />
    </Field>
    <Field label="应用目标">
      <TextInput v-model="draft.appTargets" :placeholder="trDynamic('选择或填写应用')" />
    </Field>
    <div class="field-indent app-target-actions">
      <Button size="sm" :disabled="busy" @click="chooseAppTarget">
        {{ tr("选择程序…") }}
      </Button>
      <Button
        size="sm"
        :disabled="!foregroundApp"
        :title="
          foregroundApp
            ? trDynamic('当前前台：{app}', { app: foregroundApp })
            : trDynamic('未检测到前台应用')
        "
        @click="useForegroundTarget"
      >
        {{ tr("用当前前台") }}
      </Button>
    </div>
    <p class="muted field-indent field-note">
      {{ foregroundApp ? tr("当前前台：{app}", { app: foregroundApp }) : tr("未检测到前台应用") }}
    </p>
    <Checkbox v-model="draft.autoApply">{{ tr("前台匹配时自动应用") }}</Checkbox>
    <Field label="按住连发">
      <SelectInput v-model="holdRepeatModel">
        <option value="">{{ tr("关闭") }}</option>
        <option value="single">{{ tr("单击连发") }}</option>
        <option value="long">{{ tr("长按连发") }}</option>
      </SelectInput>
    </Field>
    <Field label="语音输入工具">
      <SelectInput v-model="draft.voiceInputTool">
        <option value="">{{ tr("继承全局/设备") }}</option>
        <option value="wechat">{{ tr("微信输入法") }}</option>
        <option value="doubao">{{ tr("豆包输入法") }}</option>
        <option value="vokie">{{ tr("Vokie 桌面版") }}</option>
        <option value="other">{{ tr("其他工具") }}</option>
      </SelectInput>
    </Field>

    <div class="binding-section">
      <div class="binding-section-head">
        <h3>{{ tr("按键绑定") }}</h3>
        <span class="muted">
          {{ tr("已修改 {count} 个按键", { count: modifiedCount }) }}
        </span>
      </div>
      <div class="binding-matrix">
        <div class="binding-head" aria-hidden="true">
          <span />
          <span v-for="trigger in TRIGGERS" :key="trigger" class="binding-trigger">
            {{ statusLabel(trigger) }}
          </span>
        </div>
        <div
          v-for="button in PROFILE_BUTTONS"
          :key="button.id"
          class="binding-row"
          :class="{ modified: isRowModified(button.id) }"
        >
          <span class="binding-label">
            <span class="binding-name">{{ trDynamic(button.label) }}</span>
            <span v-if="isRowModified(button.id)" class="binding-dot" aria-hidden="true" />
            <span v-if="isRowModified(button.id)" class="sr-only">{{ tr("已修改") }}</span>
          </span>
          <div v-for="trigger in TRIGGERS" :key="trigger" class="binding-cell">
            <SelectInput
              :model-value="actionType(button.id, trigger)"
              :aria-label="`${trDynamic(button.label)} · ${statusLabel(trigger)}`"
              @update:model-value="setActionType(button.id, trigger, $event)"
            >
              <option value="inherit">{{ tr("继承基线") }}</option>
              <option value="disabled">{{ tr("禁用") }}</option>
              <option value="focus-input">{{ tr("聚焦输入") }}</option>
              <option value="shortcut">{{ tr("快捷键") }}</option>
              <option value="open-app">{{ tr("打开应用") }}</option>
              <option v-if="commonPhrasesAvailable" value="common-phrase">
                {{ tr("常用语") }}
              </option>
              <option
                v-if="!commonPhrasesAvailable && actionType(button.id, trigger) === 'common-phrase'"
                value="common-phrase"
                disabled
              >
                {{ tr("常用语（当前版本不可用）") }}
              </option>
              <option value="mouse">{{ tr("鼠标") }}</option>
              <option value="run-macro">{{ tr("运行宏") }}</option>
              <option value="other" disabled>{{ tr("其他（保留）") }}</option>
            </SelectInput>

            <div v-if="actionType(button.id, trigger) === 'shortcut'" class="cell-stack">
              <ChordInput
                :model-value="shortcutChord(button.id, trigger)"
                @update:model-value="setShortcutChord(button.id, trigger, $event)"
              />
            </div>

            <div v-if="actionType(button.id, trigger) === 'open-app'" class="cell-row">
              <TextInput
                class="cell-input"
                :placeholder="trDynamic('选择或填写应用')"
                :model-value="appValue(button.id, trigger)"
                @change="setApp(button.id, trigger, ($event.target as HTMLInputElement).value)"
              />
              <IconButton
                size="sm"
                :label="trDynamic('选择程序…')"
                @click="chooseAppAction(button.id, trigger)"
              >
                <FolderOpen :size="14" />
              </IconButton>
            </div>

            <SelectInput
              v-if="actionType(button.id, trigger) === 'run-macro'"
              :model-value="macroIdFor(button.id, trigger)"
              :aria-label="trDynamic('选择宏')"
              @update:model-value="setMacro(button.id, trigger, $event)"
            >
              <option value="">{{ tr("选择宏") }}</option>
              <option v-for="macro in latestMacros" :key="macro.macroId" :value="macro.macroId">
                {{ macro.name }}
              </option>
            </SelectInput>

            <div v-if="actionType(button.id, trigger) === 'mouse'" class="mouse-extra">
              <SelectInput
                :model-value="mouseKind(button.id, trigger)"
                :aria-label="trDynamic('鼠标')"
                @update:model-value="setMouseKind(button.id, trigger, $event)"
              >
                <option value="scroll">{{ tr("滚轮") }}</option>
                <option value="move">{{ tr("移动") }}</option>
                <option value="click">{{ tr("左键点击") }}</option>
              </SelectInput>
              <template v-if="mouseKind(button.id, trigger) !== 'click'">
                <SelectInput
                  :model-value="mouseDirection(button.id, trigger)"
                  :aria-label="trDynamic('移动')"
                  @update:model-value="setMouseDirection(button.id, trigger, $event)"
                >
                  <option value="up">{{ tr("上") }}</option>
                  <option value="down">{{ tr("下") }}</option>
                  <option value="left">{{ tr("左") }}</option>
                  <option value="right">{{ tr("右") }}</option>
                </SelectInput>
                <TextInput
                  class="cell-input"
                  type="number"
                  min="1"
                  max="100"
                  :aria-label="trDynamic('鼠标移动量')"
                  :model-value="String(mouseAmount(button.id, trigger))"
                  @change="
                    setMouseAmount(button.id, trigger, ($event.target as HTMLInputElement).value)
                  "
                />
              </template>
            </div>
          </div>
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
          {{ tr("保存方案") }}
        </Button>
      </div>
    </div>
  </Card>
</template>

<style scoped>
.editor-card {
  min-width: 0;
}

.app-target-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.field-indent {
  padding-left: calc(200px + var(--space-3));
}

.field-note {
  margin-top: calc(var(--space-2) * -1);
  font-size: var(--text-xs);
}

.binding-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding-top: var(--space-4);
}

.binding-section-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-2);
}

.binding-section-head h3 {
  font-size: var(--text-base);
  font-weight: 650;
}

.binding-section-head span {
  font-size: var(--text-xs);
}

.binding-matrix {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.binding-head,
.binding-row {
  display: grid;
  grid-template-columns: 84px repeat(3, minmax(0, 1fr));
  gap: var(--space-2);
  min-width: 0;
}

.binding-head {
  align-items: end;
  padding: 0 var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  letter-spacing: 0.06em;
  text-transform: uppercase;
}

.binding-trigger {
  min-width: 0;
}

.binding-row {
  align-items: start;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
}

.binding-row.modified {
  box-shadow: inset 2px 0 0 var(--accent);
}

.binding-label {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  min-width: 0;
  padding-top: 5px;
  font-size: var(--text-sm);
  font-weight: 600;
}

.binding-name {
  min-width: 0;
  overflow-wrap: anywhere;
}

.binding-dot {
  flex: 0 0 auto;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}

.binding-cell {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
}

.binding-cell :deep(.ui-input),
.binding-cell :deep(.ui-select-input) {
  min-height: 28px;
  border-radius: var(--radius-sm);
  padding: 3px 8px;
  font-size: var(--text-xs);
}

.binding-cell :deep(.ui-select-input) {
  padding-right: 24px;
}

.binding-cell :deep(.ui-select-icon) {
  right: 8px;
  width: 13px;
  height: 13px;
}

.cell-stack {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.cell-row {
  display: flex;
  align-items: center;
  gap: var(--space-1);
  min-width: 0;
}

.cell-row .cell-input {
  flex: 1 1 0;
  width: auto;
  min-width: 0;
}

.cell-row :deep(.ui-button) {
  flex: 0 0 auto;
  padding: 0 8px;
}

.cell-row .is-capturing {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--text);
}

.capture-hint {
  color: var(--accent);
  font-size: var(--text-2xs);
}

.mouse-extra {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  min-width: 0;
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

@media (max-width: 1260px) {
  .field-indent {
    padding-left: 0;
  }

  .field-note {
    margin-top: 0;
  }
}
</style>
