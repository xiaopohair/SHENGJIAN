<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import {
  Ban,
  CircleHelp,
  Crosshair,
  FolderOpen,
  Keyboard,
  MessageSquare,
  Mouse,
  Play,
  RotateCcw,
} from "lucide-vue-next";
import { errorMessage, resolveAppTarget } from "../lib/api";
import { type ProfileActionType, type ProfileDraft, type ProfileTrigger } from "../lib/buttons";
import { tr, trDynamic } from "../lib/i18n";
import type { LogicalAction, LogicalButton, MacroChord, MacroDefinition } from "../lib/types";
import Checkbox from "./ui/Checkbox.vue";
import IconButton from "./ui/IconButton.vue";
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

const emit = defineEmits<{
  error: [message: string];
}>();

const draft = defineModel<ProfileDraft>("draft", { required: true });

type ActionType = ProfileActionType;

const RUNNABLE_EXTENSIONS = [
  "exe",
  "lnk",
  "bat",
  "cmd",
  "com",
  "scr",
  "msi",
  "ps1",
  "vbs",
  "js",
  "url",
];

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

async function chooseApp() {
  try {
    const selected = await open({
      title: tr("选择程序"),
      multiple: false,
      filters: [{ name: tr("可运行文件"), extensions: RUNNABLE_EXTENSIONS }],
    });
    if (!selected || Array.isArray(selected)) return;
    const resolved = await resolveAppTarget(selected).catch(() => null);
    setApp(resolved || selected);
  } catch (cause) {
    emit("error", errorMessage(cause));
  }
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

    <div v-if="actionType === 'shortcut'" class="binding-params">
      <ChordInput :model-value="shortcutChord" @update:model-value="setShortcutChord" />
      <p class="binding-hint">
        {{ tr("点击捕获后直接按组合键；Esc 退出。") }}
      </p>
    </div>

    <div v-else-if="actionType === 'open-app'" class="binding-params">
      <div class="binding-param-row">
        <TextInput
          :model-value="appValue"
          :placeholder="trDynamic('选择或填写应用')"
          :aria-label="trDynamic('应用目标')"
          @change="setApp(($event.target as HTMLInputElement).value)"
        />
        <IconButton size="sm" :label="trDynamic('选择程序…')" @click="chooseApp">
          <FolderOpen :size="14" />
        </IconButton>
      </div>
    </div>

    <div v-else-if="actionType === 'run-macro'" class="binding-params">
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

    <div v-else-if="actionType === 'mouse'" class="binding-params">
      <div class="mouse-grid">
        <SelectInput
          :model-value="mouseKind"
          :aria-label="trDynamic('鼠标')"
          @update:model-value="setMouseKind($event)"
        >
          <option value="scroll">{{ tr("滚轮") }}</option>
          <option value="move">{{ tr("移动") }}</option>
          <option value="click">{{ tr("左键点击") }}</option>
          <option value="right-click">{{ tr("右键点击") }}</option>
        </SelectInput>
        <template v-if="mouseKind !== 'click' && mouseKind !== 'right-click'">
          <SelectInput
            :model-value="mouseDirection"
            :aria-label="trDynamic('移动')"
            @update:model-value="setMouseDirection($event)"
          >
            <option value="up">{{ tr("上") }}</option>
            <option value="down">{{ tr("下") }}</option>
            <option value="left">{{ tr("左") }}</option>
            <option value="right">{{ tr("右") }}</option>
          </SelectInput>
          <div class="mouse-amount">
            <TextInput
              type="number"
              min="1"
              max="100"
              :model-value="String(mouseAmount)"
              :aria-label="trDynamic('鼠标移动量')"
              @change="setMouseAmount(($event.target as HTMLInputElement).value)"
            />
            <span class="mouse-unit">{{ mouseKind === "scroll" ? tr("格") : "px" }}</span>
          </div>
        </template>
      </div>
    </div>

    <p v-else-if="actionType === 'common-phrase'" class="binding-hint">
      {{ tr("使用当前平台可用的常用语。") }}
    </p>
    <p v-else-if="actionType === 'inherit'" class="binding-hint">
      {{ tr("删除该手势的覆盖，回到全局默认。") }}
    </p>
    <p v-else-if="actionType === 'disabled'" class="binding-hint">
      {{ tr("该手势不会触发任何动作，也不会向下传递。") }}
    </p>
    <p v-else class="binding-hint">
      <CircleHelp :size="14" />
      {{ tr("该动作类型会被保留，但当前版本不提供编辑。") }}
    </p>

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

.binding-params {
  margin-top: var(--space-3);
}
.binding-param-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}
.binding-param-row > :first-child {
  flex: 1;
  min-width: 0;
}
.mouse-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-2);
}
.mouse-amount {
  display: flex;
  align-items: center;
  gap: 5px;
  min-width: 0;
}
.mouse-amount > :first-child {
  flex: 1;
  min-width: 0;
}
.mouse-unit {
  flex: 0 0 auto;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  white-space: nowrap;
}
.binding-editor.compact .mouse-grid {
  grid-template-columns: 1fr;
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
  color: var(--text-secondary);
  font-size: var(--text-sm);
  font-weight: 600;
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
