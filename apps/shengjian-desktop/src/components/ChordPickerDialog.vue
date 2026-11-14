<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { Keyboard, X } from "lucide-vue-next";
import { CHORD_KEY_GROUPS, CHORD_MODIFIER_OPTIONS } from "../lib/chordOptions";
import { tr, trDynamic } from "../lib/i18n";
import { chordParts } from "../lib/macroEditor";
import type { MacroChord } from "../lib/types";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";

/**
 * 组合键点选面板（宏/动作快捷键、语音热键共用同一套交互思路）。
 *
 * 不做键盘直接采集：那条路要拦系统级按键，输入法会把自己的和弦抢先截走
 * （Chatterfly 的 Ctrl+Win 是典型），现场表现为「按了没反应 / 只录到半个组合」。
 * 点选既能选到任意组合，也不会触发任何程序。
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    /** 当前组合；`key` 为空表示还没设。 */
    modelValue: MacroChord;
  }>(),
  { modelValue: () => ({ modifiers: [], key: "" }) },
);

const emit = defineEmits<{
  confirm: [chord: MacroChord];
  close: [];
}>();

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const modifiers = ref<string[]>([]);
const mainKey = ref("");
let previousFocus: HTMLElement | null = null;

const selected = computed<MacroChord>(() => ({
  modifiers: CHORD_MODIFIER_OPTIONS.filter((option) => modifiers.value.includes(option.code)).map(
    (option) => option.code,
  ),
  key: mainKey.value,
}));
const preview = computed(() => chordParts(selected.value).join(" + "));
const canConfirm = computed(() => mainKey.value.trim().length > 0);

function resetFromModel() {
  modifiers.value = [...props.modelValue.modifiers];
  mainKey.value = props.modelValue.key;
}

function toggleModifier(code: string) {
  modifiers.value = modifiers.value.includes(code)
    ? modifiers.value.filter((item) => item !== code)
    : [...modifiers.value, code];
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
  if (event.key === "Enter" && canConfirm.value) {
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
    <div v-if="props.open" class="chord-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="chord-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="chord-picker-title"
        tabindex="-1"
      >
        <header class="chord-head">
          <div class="chord-heading">
            <h2 id="chord-picker-title">
              <Keyboard :size="17" aria-hidden="true" />
              {{ tr("选择组合键") }}
            </h2>
            <p>{{ tr("点选修饰键与主键即可组合，不用在键盘上按下。") }}</p>
          </div>
          <IconButton size="sm" :label="tr('关闭')" @click="close">
            <X :size="16" />
          </IconButton>
        </header>

        <div class="chord-body">
          <div class="chord-group">
            <p class="chord-group-label">{{ tr("修饰键") }}</p>
            <div class="chord-chips">
              <button
                v-for="option in CHORD_MODIFIER_OPTIONS"
                :key="option.code"
                type="button"
                class="chord-chip"
                :class="{ selected: modifiers.includes(option.code) }"
                :aria-pressed="modifiers.includes(option.code)"
                @click="toggleModifier(option.code)"
              >
                {{ option.label }}
              </button>
            </div>
          </div>

          <div v-for="group in CHORD_KEY_GROUPS" :key="group.label" class="chord-group">
            <p class="chord-group-label">{{ trDynamic(group.label) }}</p>
            <div class="chord-keys">
              <button
                v-for="option in group.keys"
                :key="option.key"
                type="button"
                class="chord-key"
                :class="{ selected: mainKey === option.key }"
                :aria-pressed="mainKey === option.key"
                @click="mainKey = mainKey === option.key ? '' : option.key"
              >
                {{ trDynamic(option.label) }}
              </button>
            </div>
          </div>
        </div>

        <footer class="chord-foot">
          <div class="chord-preview" aria-live="polite">
            <span class="chord-preview-label">{{ tr("当前组合") }}</span>
            <kbd class="chord-preview-value" :class="{ empty: !preview }">
              {{ preview || tr("尚未选择") }}
            </kbd>
          </div>
          <div class="chord-actions">
            <Button variant="ghost" @click="close">{{ tr("取消") }}</Button>
            <Button variant="primary" :disabled="!canConfirm" @click="confirm">
              {{ tr("确定") }}
            </Button>
          </div>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.chord-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 62%, transparent);
  backdrop-filter: blur(2px);
}

.chord-dialog {
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

.chord-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.chord-heading h2 {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  color: var(--text);
  font-size: var(--text-lg);
  font-weight: 650;
}

.chord-heading p {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.chord-body {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
  padding: var(--space-4) var(--space-5);
}

.chord-group {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.chord-group-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.chord-chips,
.chord-keys {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
}

.chord-chip,
.chord-key {
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

.chord-chip {
  min-width: 68px;
  padding: 6px 14px;
  font-weight: 600;
}

.chord-key {
  min-width: 40px;
  padding: 6px 10px;
}

.chord-chip:hover,
.chord-key:hover {
  background: var(--bg-hover);
  border-color: var(--border-strong);
}

.chord-chip.selected,
.chord-key.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 650;
}

.chord-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-5);
}

.chord-preview {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.chord-preview-label {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.chord-preview-value {
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  padding: 3px 8px;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  white-space: nowrap;
}

.chord-preview-value.empty {
  color: var(--text-disabled);
}

.chord-actions {
  display: flex;
  flex: 0 0 auto;
  gap: var(--space-2);
}
</style>
