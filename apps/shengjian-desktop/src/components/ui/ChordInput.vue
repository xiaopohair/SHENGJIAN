<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from "vue";
import { Keyboard, X } from "lucide-vue-next";
import { tr, trDynamic } from "../../lib/i18n";
import { chordParts } from "../../lib/macroEditor";
import { chordFromKeyboardEvent } from "../../lib/shortcutCapture";
import type { MacroChord } from "../../lib/types";
import Button from "./Button.vue";
import IconButton from "./IconButton.vue";

const props = defineProps<{
  modelValue: MacroChord;
}>();

const emit = defineEmits<{
  "update:modelValue": [value: MacroChord];
}>();

const capturing = ref(false);
const parts = computed(() => chordParts(props.modelValue));
const hasChord = computed(
  () => props.modelValue.key.trim().length > 0 || props.modelValue.modifiers.length > 0,
);

function clear() {
  stop();
  emit("update:modelValue", { modifiers: [], key: "" });
}

function onKeydown(event: KeyboardEvent) {
  if (!capturing.value) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    // Escape leaves the previous chord untouched.
    stop();
    return;
  }
  const chord = chordFromKeyboardEvent(event);
  if (!chord) return;
  emit("update:modelValue", chord);
  stop();
}

function stop() {
  capturing.value = false;
}

function toggle() {
  capturing.value = !capturing.value;
}

watch(capturing, (active) => {
  if (active) {
    window.addEventListener("keydown", onKeydown, true);
  } else {
    window.removeEventListener("keydown", onKeydown, true);
  }
});

onBeforeUnmount(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <div class="chord-input" :class="{ capturing }">
    <div class="chord-value">
      <span v-if="parts.length" class="chord-keys">
        <kbd v-for="part in parts" :key="part">{{ part }}</kbd>
      </span>
      <span v-else class="chord-empty">{{ tr("未设置快捷键") }}</span>
    </div>
    <Button size="sm" :class="{ capturing }" @click="toggle">
      <Keyboard :size="14" />
      {{ capturing ? tr("按下组合键") : tr("捕获") }}
    </Button>
    <IconButton v-if="hasChord" size="sm" :label="trDynamic('清除快捷键')" @click="clear">
      <X :size="14" />
    </IconButton>
  </div>
  <p v-if="capturing" class="chord-hint" role="status" aria-live="polite">
    {{ tr("按下组合键，Esc 取消") }}
  </p>
</template>

<style scoped>
.chord-input {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.chord-value {
  flex: 1;
  min-width: 0;
}
.chord-keys {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.chord-keys kbd {
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
.chord-empty {
  color: var(--text-muted);
  font-size: 12px;
}
.chord-hint {
  margin: 4px 0 0;
  color: var(--accent-ink);
  font-size: 12px;
}
</style>
