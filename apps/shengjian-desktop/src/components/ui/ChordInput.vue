<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef, watch } from "vue";
import { Keyboard, X } from "lucide-vue-next";
import { beginShortcutCapture, endShortcutCapture } from "../../lib/api";
import {
  listenShortcutCapture,
  macroKeyFromDomCode,
  modifierForDomCode,
} from "../../lib/captureBridge";
import { tr, trDynamic } from "../../lib/i18n";
import { chordParts } from "../../lib/macroEditor";
import { chordFromKeyboardEvent } from "../../lib/shortcutCapture";
import type { MacroChord } from "../../lib/types";
import Button from "./Button.vue";
import IconButton from "./IconButton.vue";

const model = defineModel<MacroChord>({ required: true });

const capturing = shallowRef(false);
/**
 * 是否走平台钩子层录入通道。
 *
 * DOM 的 keydown 拿不到完整组合：输入法会把自己的和弦「吞掉 + 重放」，被吞掉的
 * 那半个只以注入副本出现在系统钩子层。钩子通道可用时以它为准（true）；
 * 不可用（非 Windows / 钩子未就绪）时退回 DOM 事件。
 */
const hookCapture = shallowRef(false);
const heldModifiers = new Set<string>();
let unlistenHook: (() => void) | undefined;

const parts = computed(() => chordParts(model.value));
const hasChord = computed(
  () => model.value.key.trim().length > 0 || model.value.modifiers.length > 0,
);

function clear() {
  stop();
  model.value = { modifiers: [], key: "" };
}

function finish(chord: MacroChord) {
  model.value = chord;
  stop();
}

/** 一条钩子层边沿：修饰键先攒着，主键到齐即成组合。 */
function applyEdge(code: string, isDown: boolean) {
  if (!capturing.value) return;
  if (isDown) {
    if (code === "Escape") {
      // Esc 放弃本次录入，保留原组合。
      stop();
      return;
    }
    const modifier = modifierForDomCode(code);
    if (modifier) {
      heldModifiers.add(modifier);
      return;
    }
    const key = macroKeyFromDomCode(code);
    if (!key) return;
    const order = ["control", "alt", "shift", "win"];
    finish({
      modifiers: order.filter((name) => heldModifiers.has(name)),
      key,
    });
    return;
  }
  const modifier = modifierForDomCode(code);
  if (modifier) heldModifiers.delete(modifier);
}

function onKeydown(event: KeyboardEvent) {
  if (!capturing.value || hookCapture.value) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.key === "Escape") {
    stop();
    return;
  }
  const chord = chordFromKeyboardEvent(event);
  if (!chord) return;
  finish(chord);
}

async function stop() {
  capturing.value = false;
  hookCapture.value = false;
  heldModifiers.clear();
  window.removeEventListener("keydown", onKeydown, true);
  unlistenHook?.();
  unlistenHook = undefined;
  await endShortcutCapture().catch(() => undefined);
}

async function start() {
  capturing.value = true;
  heldModifiers.clear();
  window.addEventListener("keydown", onKeydown, true);
  try {
    // 先订阅再开吞键：反过来的话最早几条边沿会丢。
    unlistenHook = await listenShortcutCapture(applyEdge);
    hookCapture.value = await beginShortcutCapture();
  } catch {
    unlistenHook?.();
    unlistenHook = undefined;
    hookCapture.value = false;
  }
}

function toggle() {
  if (capturing.value) void stop();
  else void start();
}

// 钩子通道生效时由边沿驱动；DOM 监听只在退回模式下使用。
watch(hookCapture, (enabled) => {
  if (!capturing.value) return;
  if (enabled) window.removeEventListener("keydown", onKeydown, true);
  else window.addEventListener("keydown", onKeydown, true);
});

onBeforeUnmount(() => {
  void stop();
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
