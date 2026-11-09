<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";
import { Copy, Minus, Square, X } from "lucide-vue-next";
import BrandMark from "./BrandMark.vue";
import { trDynamic } from "../lib/i18n";

const appWindow = getCurrentWindow();
const maximized = ref(false);
let unlisten: UnlistenFn | undefined;

async function syncMaximized() {
  maximized.value = await appWindow.isMaximized().catch(() => false);
}

async function minimize() {
  await appWindow.minimize().catch(() => undefined);
}

async function toggleMaximize() {
  await appWindow.toggleMaximize().catch(() => undefined);
  await syncMaximized();
}

async function close() {
  await appWindow.close().catch(() => undefined);
}

function onDoubleClick(event: MouseEvent) {
  if ((event.target as HTMLElement | null)?.closest(".titlebar-actions")) return;
  void toggleMaximize();
}

onMounted(async () => {
  await syncMaximized();
  try {
    unlisten = await appWindow.onResized(() => {
      void syncMaximized();
    });
  } catch {
    // The window still works if resize events are unavailable.
  }
});

onUnmounted(() => {
  unlisten?.();
});
</script>

<template>
  <header
    class="titlebar"
    data-tauri-drag-region
    :aria-label="trDynamic('应用标题栏')"
    @dblclick="onDoubleClick"
  >
    <div class="titlebar-brand" data-tauri-drag-region>
      <BrandMark :height="12" />
      <span class="titlebar-name">{{ trDynamic("声键") }}</span>
    </div>
    <div class="titlebar-drag" data-tauri-drag-region />
    <div class="titlebar-actions" :aria-label="trDynamic('窗口控制')" role="group">
      <button
        type="button"
        class="titlebar-button"
        :aria-label="trDynamic('最小化')"
        :title="trDynamic('最小化')"
        @click="minimize"
      >
        <Minus :size="16" />
      </button>
      <button
        type="button"
        class="titlebar-button"
        :aria-label="trDynamic(maximized ? '还原' : '最大化')"
        :title="trDynamic(maximized ? '还原' : '最大化')"
        @click="toggleMaximize"
      >
        <component :is="maximized ? Copy : Square" :size="13" />
      </button>
      <button
        type="button"
        class="titlebar-button titlebar-close"
        :aria-label="trDynamic('关闭')"
        :title="trDynamic('关闭')"
        @click="close"
      >
        <X :size="16" />
      </button>
    </div>
  </header>
</template>

<style scoped>
.titlebar {
  position: relative;
  z-index: 200;
  display: flex;
  flex: 0 0 var(--titlebar-height);
  align-items: stretch;
  height: var(--titlebar-height);
  border-bottom: 1px solid transparent;
  background: var(--bg-sidebar);
  user-select: none;
  -webkit-user-select: none;
}

.titlebar-drag {
  flex: 1;
  min-width: 24px;
}

.titlebar-brand {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 6px;
  padding-inline: 12px 8px;
  color: var(--text-secondary);
}

/* Let the whole brand block start a window drag instead of its children. */
.titlebar-brand > * {
  pointer-events: none;
}

.titlebar-name {
  font-size: var(--text-xs);
  font-weight: 600;
  letter-spacing: 0.03em;
  white-space: nowrap;
}

.titlebar-actions {
  display: flex;
  align-items: stretch;
}

.titlebar-button {
  display: grid;
  place-items: center;
  width: 46px;
  border: 0;
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.titlebar-button:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.titlebar-button:focus-visible {
  position: relative;
  z-index: 1;
  outline-offset: -3px;
}

.titlebar-close:hover {
  background: var(--window-close);
  color: var(--window-close-text);
}
</style>
