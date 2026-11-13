<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import {
  Ban,
  CircleAlert,
  CircleCheck,
  CircleHelp,
  Crosshair,
  FolderOpen,
  Keyboard,
  MessageSquare,
  Mouse,
  Play,
  RotateCcw,
} from "lucide-vue-next";
import { type ProfileActionType, type ProfileDraft, type ProfileTrigger } from "../lib/buttons";
import { tr, trDynamic } from "../lib/i18n";
import type { LogicalAction, LogicalButton, MacroChord, MacroDefinition } from "../lib/types";
import { cachedAppTarget, useAppPicker } from "../composables/useAppPicker";
import { issueReasonKey } from "../lib/appTargets";
import AppPickerDialog from "./app-picker/AppPickerDialog.vue";
import AppTargetField from "./app-picker/AppTargetField.vue";
import Checkbox from "./ui/Checkbox.vue";
import SelectInput from "./ui/SelectInput.vue";
import TextInput from "./ui/TextInput.vue";
import ChordInput from "./ui/ChordInput.vue";

const MODIFIER_IDS = new Set(["control", "alt", "shift", "win"]);

const props = defineProps<{
  buttonId: LogicalButton;
  trigger: ProfileTrigger;
  compact?: boolean;
  busy: boolean;
  commonPhrasesAvailable: boolean;
  latestMacros: MacroDefinition[];
}>();

// Kept for compatibility with ProfileWorkbench's error listener; the picker now
// reports its own failures through the shared toast channel.
defineEmits<{
  error: [message: string];
}>();

const draft = defineModel<ProfileDraft>("draft", { required: true });

type ActionType = ProfileActionType;

const {
  open: appPickerOpen,
  openPicker: openAppPicker,
  closePicker: closeAppPicker,
} = useAppPicker();

const ACTIONS = computed(() =>
  [
    {
      type: "shortcut" as const,
      label: "发送快捷键",
      detail: "键盘组合",
      icon: Keyboard,
      tone: "teal",
    },
    {
      type: "focus-input" as const,
      label: "聚焦输入框",
      detail: "把光标送回输入框",
      icon: Crosshair,
      tone: "teal",
    },
    {
      type: "run-macro" as const,
      label: "运行宏",
      detail: "执行一段动作",
      icon: Play,
      tone: "violet",
    },
    {
      type: "open-app" as const,
      label: "打开应用",
      detail: "启动或切换应用",
      icon: FolderOpen,
      tone: "amber",
    },
    {
      type: "mouse" as const,
      label: "鼠标",
      detail: "滚轮/移动/左右键",
      icon: Mouse,
      tone: "neutral",
    },
    {
      type: "common-phrase" as const,
      label: "常用语",
      detail: "插入常用文本",
      icon: MessageSquare,
      tone: "neutral",
      hidden: !props.commonPhrasesAvailable && actionType.value !== "common-phrase",
    },
    {
      type: "disabled" as const,
      label: "禁用",
      detail: "阻止默认行为",
      icon: Ban,
      tone: "danger",
    },
    {
      type: "inherit" as const,
      label: "继承默认",
      detail: "使用基线绑定",
      icon: RotateCcw,
      tone: "muted",
    },
  ].filter((action) => !action.hidden),
);

const holdRepeatSetting = computed<"inherit" | "off" | "single" | "long">({
  get: () => draft.value.holdRepeatByButton?.[props.buttonId] ?? "inherit",
  set: (value) => {
    draft.value.holdRepeatByButton ??= {};
    if (value === "inherit") {
      delete draft.value.holdRepeatByButton[props.buttonId];
    } else {
      draft.value.holdRepeatByButton[props.buttonId] = value;
    }
  },
});

const currentActionAllowsRepeat = computed(() => {
  const action = draft.value.bindings[props.buttonId]?.[props.trigger];
  if (action?.type === "shortcut" && Array.isArray(action.keys)) {
    const keys = action.keys.map((key) => key.trim().toLowerCase()).filter(Boolean);
    return (
      keys.length === 1 &&
      !keys[0].includes("+") &&
      !["control", "ctrl", "alt", "shift", "win", "meta"].includes(keys[0])
    );
  }
  if (action?.type === "mouse") {
    return action.kind === "scroll" || action.kind === "move";
  }
  return false;
});

const showHoldRepeat = computed(
  () => props.trigger !== "double" && currentActionAllowsRepeat.value,
);

watch(
  [() => props.trigger, currentActionAllowsRepeat],
  () => {
    if (props.trigger === "double") return;
    if (!currentActionAllowsRepeat.value && holdRepeatSetting.value === props.trigger) {
      holdRepeatSetting.value = "off";
    }
  },
  { immediate: true },
);

const singleRepeat = computed({
  get: () => holdRepeatSetting.value === "single",
  set: (value: boolean) => {
    if (value) {
      holdRepeatSetting.value = "single";
    } else if (holdRepeatSetting.value === "single") {
      holdRepeatSetting.value = "off";
    }
  },
});

const longRepeat = computed({
  get: () => holdRepeatSetting.value === "long",
  set: (value: boolean) => {
    if (value) {
      holdRepeatSetting.value = "long";
    } else if (holdRepeatSetting.value === "long") {
      holdRepeatSetting.value = "off";
    }
  },
});

const currentAction = computed<LogicalAction | undefined>(
  () => draft.value.bindings[props.buttonId]?.[props.trigger],
);

const actionType = computed<ActionType>(() => {
  const action = currentAction.value;
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
});

const shortcutChord = computed<MacroChord>(() => {
  const action = currentAction.value;
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
});

const appValue = computed(() => {
  const action = currentAction.value;
  return action?.type === "open-app" && typeof action.app === "string" ? action.app : "";
});

const macroId = computed(() => {
  const action = currentAction.value;
  return action?.type === "run-macro" && typeof action.macro_id === "string" ? action.macro_id : "";
});

const mouseKind = computed(() => {
  const action = currentAction.value;
  return action?.type === "mouse" && typeof action.kind === "string" ? action.kind : "scroll";
});

type MouseDirection = "up" | "down" | "left" | "right";
const lastMouseDirection = ref<MouseDirection>("down");
const lastMouseAmount = shallowRef(3);

const mouseDirection = computed(() => {
  const action = currentAction.value;
  return action?.type === "mouse" && typeof action.direction === "string"
    ? action.direction
    : lastMouseDirection.value;
});

const mouseAmount = computed(() => {
  const action = currentAction.value;
  const amount = action?.type === "mouse" && typeof action.amount === "number" ? action.amount : 0;
  return amount > 0 ? amount : lastMouseAmount.value;
});

/** 有参数的动作：横线 + 「动作参数」+ 字段 + 说明（与宏编辑器同款）。 */
const hasParamFields = computed(() =>
  ["shortcut", "open-app", "run-macro", "mouse"].includes(actionType.value),
);

/** 缺必填内容时显示红色说明行。 */
const paramIssue = computed(() => {
  switch (actionType.value) {
    case "shortcut":
      return shortcutChord.value.key ? "" : tr("还没有录制快捷键。");
    case "open-app": {
      if (!appValue.value.trim()) return tr("还没有选择应用。");
      const info = cachedAppTarget(appValue.value);
      if (info && !info.launchable) {
        return tr("应用不可用：{reason}", { reason: tr(issueReasonKey(info.issue)) });
      }
      return info?.legacy ? tr("旧式目标，建议重新选择") : "";
    }
    case "run-macro": {
      if (!macroId.value.trim()) return tr("还没有选择宏。");
      return props.latestMacros.some((macro) => macro.macroId === macroId.value)
        ? ""
        : tr("所选宏已不存在，请重新选择。");
    }
    default:
      return "";
  }
});

/** 有参数动作的说明行。 */
const paramNote = computed(() => {
  switch (actionType.value) {
    case "shortcut":
      return tr("点击捕获后直接按组合键；Esc 退出。");
    case "open-app":
      return tr("点「选择应用」更换应用。");
    case "run-macro":
      return tr("执行该宏的最新保存版本。");
    case "mouse":
      return tr("滚轮与移动支持按住连发；点击只触发一次。");
    default:
      return "";
  }
});

/** 无参数动作的说明行。 */
const actionNote = computed(() => {
  switch (actionType.value) {
    case "focus-input":
      return tr("把光标送回当前应用的输入框。");
    case "common-phrase":
      return tr("使用当前平台可用的常用语。");
    case "inherit":
      return tr("删除该手势的覆盖，回到默认方案。");
    case "disabled":
      return tr("该手势不会触发任何动作，也不会向下传递。");
    case "other":
      return tr("该动作类型会被保留，但当前版本不提供编辑。");
    default:
      return "";
  }
});

watch(
  currentAction,
  (action) => {
    if (action?.type !== "mouse") return;
    if (typeof action.amount === "number" && action.amount > 0) {
      lastMouseAmount.value = action.amount;
    }
    if (
      (action.kind === "scroll" || action.kind === "move") &&
      (action.direction === "up" ||
        action.direction === "down" ||
        action.direction === "left" ||
        action.direction === "right")
    ) {
      lastMouseDirection.value = action.direction;
    }
  },
  { deep: true, immediate: true },
);

function bindingsForButton(): Partial<Record<ProfileTrigger, LogicalAction>> {
  draft.value.bindings[props.buttonId] ??= {};
  return draft.value.bindings[props.buttonId];
}

function setAction(type: string) {
  const bindings = bindingsForButton();
  if (type === "inherit") {
    delete bindings[props.trigger];
    return;
  }
  if (type === "other") return;
  if (type === "disabled") {
    bindings[props.trigger] = { type: "disabled" };
  } else if (type === "focus-input") {
    bindings[props.trigger] = { type: "focus-input" };
  } else if (type === "shortcut") {
    bindings[props.trigger] = { type: "shortcut", label: "", keys: [] };
  } else if (type === "open-app") {
    bindings[props.trigger] = { type: "open-app", app: "" };
  } else if (type === "common-phrase") {
    bindings[props.trigger] = { type: "common-phrase" };
  } else if (type === "mouse") {
    bindings[props.trigger] = {
      type: "mouse",
      kind: "scroll",
      direction: lastMouseDirection.value,
      amount: lastMouseAmount.value,
    };
  } else if (type === "run-macro") {
    bindings[props.trigger] = {
      type: "run-macro",
      macro_id: props.latestMacros[0]?.macroId ?? "",
    };
  }
}

function setShortcut(value: string) {
  const keys = value
    .split("+")
    .map((key) => key.trim())
    .filter(Boolean);
  bindingsForButton()[props.trigger] = {
    type: "shortcut",
    label: keys.join("+"),
    keys,
  };
}

function setShortcutChord(chord: MacroChord) {
  setShortcut([...chord.modifiers, chord.key].filter(Boolean).join("+"));
}

function setApp(value: string) {
  bindingsForButton()[props.trigger] = { type: "open-app", app: value.trim() };
}

/** 同一个方案里其它 open-app 目标，作为选择器的「最近使用」。 */
const recentAppTargets = computed<string[]>(() => {
  const current = appValue.value.trim();
  const targets: string[] = [];
  const seen = new Set<string>();
  for (const triggers of Object.values(draft.value.bindings ?? {})) {
    for (const action of Object.values(triggers ?? {})) {
      if (action?.type !== "open-app" || typeof action.app !== "string") continue;
      const target = action.app.trim();
      if (!target || target === current) continue;
      const key = target.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      targets.push(target);
      if (targets.length >= 8) return targets;
    }
  }
  return targets;
});

function applyPickedApp(target: string) {
  setApp(target);
  closeAppPicker();
}

function setMacro(value: string) {
  bindingsForButton()[props.trigger] = { type: "run-macro", macro_id: value };
}

function setMouseKind(value: string) {
  const isClick = value === "click" || value === "right-click";
  bindingsForButton()[props.trigger] = {
    type: "mouse",
    kind: value,
    direction: isClick ? null : lastMouseDirection.value,
    amount: isClick ? 0 : lastMouseAmount.value,
  };
}

function setMouseDirection(value: string) {
  if (value === "up" || value === "down" || value === "left" || value === "right") {
    lastMouseDirection.value = value;
  }
  bindingsForButton()[props.trigger] = {
    type: "mouse",
    kind: mouseKind.value,
    direction: value,
    amount: mouseAmount.value,
  };
}

function setMouseAmount(value: string) {
  const amount = Math.max(1, Math.min(100, Number(value) || 1));
  lastMouseAmount.value = amount;
  bindingsForButton()[props.trigger] = {
    type: "mouse",
    kind: mouseKind.value,
    direction: mouseDirection.value,
    amount,
  };
}
</script>

<template>
  <div class="binding-editor" :class="{ compact }">
    <div class="binding-actions" :class="{ compact }">
      <button
        v-for="action in ACTIONS"
        :key="action.type"
        type="button"
        class="binding-option"
        :class="[action.tone, { active: actionType === action.type }]"
        :disabled="busy"
        @click="setAction(action.type)"
      >
        <span class="binding-option-icon">
          <component :is="action.icon" :size="15" />
        </span>
        <span class="binding-option-copy">
          <strong>{{ trDynamic(action.label) }}</strong>
          <small>{{ trDynamic(action.detail) }}</small>
        </span>
      </button>
    </div>

    <div v-if="hasParamFields" class="action-config">
      <h4 class="config-head">{{ tr("动作参数") }}</h4>

      <div v-if="actionType === 'shortcut'" class="config-field">
        <label>{{ tr("快捷键") }}</label>
        <ChordInput :model-value="shortcutChord" @update:model-value="setShortcutChord" />
      </div>

      <div v-else-if="actionType === 'open-app'" class="config-field">
        <label>{{ tr("应用") }}</label>
        <AppTargetField :target="appValue" :disabled="busy" @choose="openAppPicker" />
      </div>

      <div v-else-if="actionType === 'run-macro'" class="config-field">
        <label>{{ tr("宏") }}</label>
        <SelectInput
          :model-value="macroId"
          :aria-label="trDynamic('选择宏')"
          @update:model-value="setMacro($event)"
        >
          <option value="">{{ tr("选择宏") }}</option>
          <option v-for="macro in latestMacros" :key="macro.macroId" :value="macro.macroId">
            {{ macro.name }}
          </option>
        </SelectInput>
      </div>

      <div v-else-if="actionType === 'mouse'" class="config-row">
        <div class="config-field mouse-kind">
          <label>{{ tr("鼠标动作") }}</label>
          <SelectInput
            :model-value="mouseKind"
            :aria-label="trDynamic('鼠标动作')"
            @update:model-value="setMouseKind($event)"
          >
            <option value="scroll">{{ tr("滚轮") }}</option>
            <option value="move">{{ tr("移动") }}</option>
            <option value="click">{{ tr("左键点击") }}</option>
            <option value="right-click">{{ tr("右键点击") }}</option>
          </SelectInput>
        </div>
        <template v-if="mouseKind !== 'click' && mouseKind !== 'right-click'">
          <div class="config-field mouse-direction">
            <label>{{ tr("方向") }}</label>
            <SelectInput
              :model-value="mouseDirection"
              :aria-label="trDynamic('方向')"
              @update:model-value="setMouseDirection($event)"
            >
              <option value="up">{{ tr("上") }}</option>
              <option value="down">{{ tr("下") }}</option>
              <option value="left">{{ tr("左") }}</option>
              <option value="right">{{ tr("右") }}</option>
            </SelectInput>
          </div>
          <div class="config-field mouse-amount">
            <label>{{ tr("数量") }}</label>
            <div class="mouse-amount-row">
              <TextInput
                type="number"
                min="1"
                max="100"
                :model-value="String(mouseAmount)"
                :aria-label="trDynamic('数量')"
                @change="setMouseAmount(($event.target as HTMLInputElement).value)"
              />
              <span class="mouse-unit">{{ mouseKind === "scroll" ? tr("格") : "px" }}</span>
            </div>
          </div>
        </template>
      </div>

      <p v-if="paramIssue" class="config-issue" role="alert">
        <CircleAlert :size="13" />
        {{ paramIssue }}
      </p>
      <p v-else class="config-note">
        <CircleCheck :size="13" />
        {{ paramNote }}
      </p>
    </div>

    <div v-else class="action-config action-config-note">
      <h4 class="config-head">{{ tr("动作参数") }}</h4>
      <p class="config-note">
        <CircleHelp :size="13" />
        {{ actionNote }}
      </p>
    </div>

    <div v-if="showHoldRepeat" class="hold-repeat-section">
      <div class="hold-repeat-head">
        <span class="hold-repeat-label">{{ tr("按住连发") }}</span>
      </div>
      <div class="hold-repeat-options">
        <Checkbox v-if="trigger === 'single'" v-model="singleRepeat">
          {{ tr("按住不放重复触发") }}
        </Checkbox>
        <Checkbox v-else v-model="longRepeat">
          {{ tr("按住不放重复触发") }}
        </Checkbox>
      </div>
      <p class="binding-hint hold-repeat-hint">
        {{
          trigger === "single"
            ? tr("按住约 350ms 后，持续重复“单击”动作。")
            : tr("按住约 550ms 触发一次长按动作，然后继续重复这个动作。")
        }}
      </p>
    </div>

    <AppPickerDialog
      :open="appPickerOpen"
      mode="launch"
      :current-target="appValue"
      :recent-targets="recentAppTargets"
      @select="applyPickedApp"
      @close="closeAppPicker"
    />
  </div>
</template>

<style scoped>
.binding-editor {
  min-width: 0;
}

.binding-actions {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: var(--space-2);
}

.binding-actions.compact {
  grid-template-columns: 1fr;
  gap: 5px;
}

.binding-option {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: 10px;
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.binding-option:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--bg-hover);
}
.binding-option.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.binding-option:disabled {
  cursor: default;
  opacity: 0.6;
}

.binding-actions.compact .binding-option {
  align-items: center;
  padding: 6px 8px;
}

.binding-option-icon {
  display: grid;
  flex: 0 0 26px;
  place-items: center;
  width: 26px;
  height: 26px;
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  color: var(--text-muted);
}

.binding-option.active .binding-option-icon {
  background: var(--accent-soft-strong);
  color: var(--accent);
}
.binding-option.violet .binding-option-icon {
  color: var(--violet-text, var(--accent));
}
.binding-option.amber .binding-option-icon {
  color: var(--warning-text);
}
.binding-option.danger .binding-option-icon {
  color: var(--danger-text);
}
.binding-option-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.binding-option-copy strong {
  font-size: var(--text-xs);
  font-weight: 650;
}
.binding-option-copy small {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.35;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.binding-actions.compact .binding-option-copy small {
  display: none;
}

.action-config {
  margin-top: var(--space-4);
  border-top: 1px solid var(--border);
  padding-top: var(--space-3);
}
.action-config-note {
  margin-top: var(--space-3);
  padding-top: var(--space-2);
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
.config-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
  margin-bottom: 8px;
}
.config-field label {
  color: var(--text-muted);
  font-size: 12px;
}
.mouse-kind {
  flex: 1 1 160px;
}
.mouse-direction {
  flex: 0 0 110px;
}
.mouse-amount {
  flex: 0 0 130px;
}

.mouse-amount-row {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
}
.mouse-amount-row > :first-child {
  flex: 1;
  min-width: 0;
}
.mouse-unit {
  flex: 0 0 auto;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  white-space: nowrap;
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
.binding-editor.compact .config-row {
  flex-direction: column;
  align-items: stretch;
  gap: 0;
}
.binding-editor.compact .mouse-kind,
.binding-editor.compact .mouse-direction,
.binding-editor.compact .mouse-amount {
  flex: 1 1 auto;
}

.binding-hint {
  display: flex;
  align-items: flex-start;
  gap: 6px;
  margin: var(--space-2) 0 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.hold-repeat-section {
  margin-top: var(--space-4);
  border-top: 1px solid var(--border);
  padding-top: var(--space-3);
}

.hold-repeat-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-2);
}

.hold-repeat-label {
  color: var(--text-muted);
  font-size: 12px;
  font-weight: 400;
}

.hold-repeat-options {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-4);
  margin-top: var(--space-2);
}

.hold-repeat-hint {
  margin-top: 5px;
}

.is-capturing {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

@media (max-width: 1400px) {
  .binding-actions {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
</style>
