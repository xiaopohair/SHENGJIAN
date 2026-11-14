<script setup lang="ts">
import { computed, shallowRef } from "vue";
import { Keyboard, X } from "lucide-vue-next";
import ChordPickerDialog from "../ChordPickerDialog.vue";
import { tr, trDynamic } from "../../lib/i18n";
import { chordParts } from "../../lib/macroEditor";
import type { MacroChord } from "../../lib/types";
import Button from "./Button.vue";
import IconButton from "./IconButton.vue";

/**
 * 组合键输入（宏动作、按键动作共用）。
 *
 * 只做点选：点「选择」弹出面板，用鼠标组合修饰键与主键。不做键盘直接采集——
 * 那条路要拦系统级按键，输入法会把自己的和弦抢先截走（Chatterfly 的 Ctrl+Win
 * 是典型），现场表现为「按了没反应 / 只录到半个组合」。
 */
const model = defineModel<MacroChord>({ required: true });

const pickerOpen = shallowRef(false);
const parts = computed(() => chordParts(model.value));
const hasChord = computed(
  () => model.value.key.trim().length > 0 || model.value.modifiers.length > 0,
);

function clear() {
  model.value = { modifiers: [], key: "" };
}

function applyChord(chord: MacroChord) {
  model.value = chord;
  pickerOpen.value = false;
}
</script>

<template>
  <div class="chord-input">
    <div class="chord-value">
      <span v-if="parts.length" class="chord-keys">
        <kbd v-for="part in parts" :key="part">{{ part }}</kbd>
      </span>
      <span v-else class="chord-empty">{{ tr("未设置快捷键") }}</span>
    </div>
    <Button size="sm" @click="pickerOpen = true">
      <Keyboard :size="14" />
      {{ tr("选择") }}
    </Button>
    <IconButton v-if="hasChord" size="sm" :label="trDynamic('清除快捷键')" @click="clear">
      <X :size="14" />
    </IconButton>
  </div>

  <ChordPickerDialog
    :open="pickerOpen"
    :model-value="model"
    @confirm="applyChord"
    @close="pickerOpen = false"
  />
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
</style>
