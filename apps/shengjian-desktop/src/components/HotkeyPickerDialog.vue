<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { Keyboard, X } from "lucide-vue-next";
import { tr, trDynamic } from "../lib/i18n";
import {
  MAIN_KEY_GROUPS,
  MODIFIER_OPTIONS,
  hotkeyKeyLabel,
  normalizeHotkey,
  sameHotkey,
  splitHotkey,
} from "../lib/hotkey";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";

/**
 * 语音热键点选面板。
 *
 * 不做键盘直接采集：那套要拦系统级按键，输入法会把自己的语音和弦抢先截走
 * （Chatterfly 的 Ctrl+Win 就是典型），现场表现为「按了没反应 / 只录到半个组合」。
 * 点选既能确定地选到任意组合，也不会触发任何程序。
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    /** 当前热键（`KeyboardEvent.code` 列表）；空数组 = 不注入热键。 */
    modelValue: string[];
    /** 当前语音输入工具的默认和弦，用于提示「可能被输入法抢先响应」。 */
    toolDefault?: string[];
    /** 当前语音输入工具显示名。 */
    toolLabel?: string;
  }>(),
  { toolDefault: () => [], toolLabel: "" },
);

const emit = defineEmits<{
  confirm: [keys: string[]];
  close: [];
}>();

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const modifiers = ref<string[]>([]);
const mainKey = ref("");
let previousFocus: HTMLElement | null = null;

/** 点选顺序不影响结果：展示与落盘都用规范顺序（Ctrl → Shift → Alt → Win → 主键）。 */
const selected = computed(() =>
  normalizeHotkey([...modifiers.value, ...(mainKey.value ? [mainKey.value] : [])]),
);
/** 预览逐个按键展示成键帽，与输入框里的展示一致（不再拼成一整串）。 */
const previewParts = computed(() => selected.value.map(hotkeyKeyLabel));
const canConfirm = computed(() => selected.value.length > 0);

/** 选中的组合正好是当前输入法自己的语音和弦：运行时很可能被它抢先响应。 */
const conflictsWithTool = computed(
  () =>
    props.toolDefault.length > 0 &&
    selected.value.length > 0 &&
    sameHotkey(selected.value, props.toolDefault),
);

function resetFromModel() {
  const { modifiers: pickedModifiers, mainKey: pickedMainKey } = splitHotkey(props.modelValue);
  modifiers.value = pickedModifiers;
  mainKey.value = pickedMainKey;
}

function toggleModifier(code: string) {
  modifiers.value = modifiers.value.includes(code)
    ? modifiers.value.filter((item) => item !== code)
    : [...modifiers.value, code];
}

function toggleMainKey(code: string) {
  mainKey.value = mainKey.value === code ? "" : code;
}

function close() {
  emit("close");
}

function confirm() {
  if (!canConfirm.value) return;
  emit("confirm", selected.value);
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    close();
    return;
  }
  if (event.key === "Enter") {
    event.preventDefault();
    confirm();
  }
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      resetFromModel();
      previousFocus = document.activeElement as HTMLElement | null;
      window.addEventListener("keydown", onKeydown, true);
      await nextTick();
      dialogRef.value?.focus();
    } else {
      window.removeEventListener("keydown", onKeydown, true);
      previousFocus?.focus();
      previousFocus = null;
    }
  },
);

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="hotkey-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="hotkey-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="hotkey-picker-title"
        tabindex="-1"
      >
        <header class="hotkey-head">
          <div class="hotkey-heading">
            <h2 id="hotkey-picker-title">
              <Keyboard :size="17" aria-hidden="true" />
              {{ tr("选择热键") }}
            </h2>
            <p>{{ tr("点选修饰键与主键即可组合，不用在键盘上按下。") }}</p>
          </div>
          <IconButton size="sm" :label="tr('关闭')" @click="close">
            <X :size="16" />
          </IconButton>
        </header>

        <div class="hotkey-body">
          <div class="hotkey-group">
            <p class="hotkey-group-label">{{ tr("修饰键") }}</p>
            <div class="hotkey-chips">
              <button
                v-for="option in MODIFIER_OPTIONS"
                :key="option.code"
                type="button"
                class="hotkey-chip"
                :class="{ selected: modifiers.includes(option.code) }"
                :aria-pressed="modifiers.includes(option.code)"
                @click="toggleModifier(option.code)"
              >
                {{ option.label }}
              </button>
            </div>
          </div>

          <div v-for="group in MAIN_KEY_GROUPS" :key="group.label" class="hotkey-group">
            <p class="hotkey-group-label">{{ trDynamic(group.label) }}</p>
            <div class="hotkey-keys">
              <button
                v-for="code in group.keys"
                :key="code"
                type="button"
                class="hotkey-key"
                :class="{ selected: mainKey === code }"
                :aria-pressed="mainKey === code"
                @click="toggleMainKey(code)"
              >
                {{ hotkeyKeyLabel(code) }}
              </button>
            </div>
          </div>
        </div>

        <footer class="hotkey-foot">
          <div class="hotkey-preview" aria-live="polite">
            <span class="hotkey-preview-label">{{ tr("当前组合") }}</span>
            <span v-if="previewParts.length" class="hotkey-preview-keys">
              <kbd v-for="(part, index) in previewParts" :key="`${part}-${index}`">
                {{ part }}
              </kbd>
            </span>
            <span v-else class="hotkey-preview-empty">{{ tr("尚未选择") }}</span>
          </div>
          <div class="hotkey-actions">
            <Button variant="ghost" @click="close">{{ tr("取消") }}</Button>
            <Button variant="primary" :disabled="!canConfirm" @click="confirm">
              {{ tr("确定") }}
            </Button>
          </div>
        </footer>

        <p v-if="conflictsWithTool" class="hotkey-warning" role="status">
          {{
            tr("这是 {tool} 自己的语音和弦，运行时可能被它抢先响应；建议换一个组合。", {
              tool: props.toolLabel || tr("当前输入法"),
            })
          }}
        </p>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.hotkey-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 62%, transparent);
  backdrop-filter: blur(2px);
}

.hotkey-dialog {
  display: flex;
  flex-direction: column;
  width: min(620px, calc(100vw - 48px));
  max-height: min(720px, calc(100vh - 96px));
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-elevated);
  box-shadow: var(--shadow-lg);
  outline: none;
}

.hotkey-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.hotkey-heading h2 {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  color: var(--text);
  font-size: var(--text-lg);
  font-weight: 650;
}

.hotkey-heading p {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.hotkey-body {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
  padding: var(--space-4) var(--space-5);
}

.hotkey-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.hotkey-group-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.hotkey-chips,
.hotkey-keys {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.hotkey-chip,
.hotkey-key {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.hotkey-chip {
  /* 修饰键也用键帽观感（与主键一致）：等宽、底边加重、抬起浅底。 */
  min-width: 68px;
  border-bottom-width: 2px;
  border-bottom-color: var(--border-strong);
  border-radius: 5px;
  background: var(--bg-raised);
  padding: 3px 14px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  font-weight: 600;
  line-height: 20px;
}

.hotkey-key {
  /* 键帽观感：等宽、底边加重、抬起的浅底，与输入框里的键帽一致。 */
  min-width: 40px;
  border-bottom-width: 2px;
  border-bottom-color: var(--border-strong);
  border-radius: 5px;
  background: var(--bg-raised);
  padding: 3px 8px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  line-height: 20px;
}

.hotkey-chip:hover,
.hotkey-key:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

.hotkey-chip.selected,
.hotkey-key.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 650;
}

.hotkey-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-5);
}

.hotkey-preview {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.hotkey-preview-label {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.hotkey-preview-keys {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.hotkey-preview-keys kbd {
  border: 1px solid var(--border);
  border-bottom-color: var(--border-strong);
  border-radius: 4px;
  background: var(--bg-raised);
  padding: 0 6px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 20px;
}

.hotkey-preview-empty {
  color: var(--text-disabled);
  font-size: var(--text-xs);
}

.hotkey-actions {
  display: flex;
  flex: 0 0 auto;
  gap: var(--space-2);
}

.hotkey-warning {
  margin: 0;
  border-top: 1px solid var(--border);
  padding: var(--space-2) var(--space-5);
  color: var(--warning-text);
  font-size: var(--text-xs);
  line-height: 1.5;
}
</style>
