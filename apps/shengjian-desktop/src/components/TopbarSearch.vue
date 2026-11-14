<script setup lang="ts">
import { computed, nextTick, ref, shallowRef, useTemplateRef, watch } from "vue";
import { Search } from "lucide-vue-next";
import type { PaletteCommand } from "../lib/palette";
import { tr, trDynamic } from "../lib/i18n";

const props = defineProps<{
  commands: PaletteCommand[];
}>();

const emit = defineEmits<{
  run: [command: PaletteCommand];
}>();

const expanded = shallowRef(false);
const query = shallowRef("");
const activeIndex = shallowRef(0);
const rootRef = useTemplateRef<HTMLElement>("rootRef");
const inputRef = useTemplateRef<HTMLInputElement>("inputRef");
const toggleRef = useTemplateRef<HTMLButtonElement>("toggleRef");
const optionRefs = ref<HTMLElement[]>([]);

const filtered = computed(() => {
  const value = query.value.trim().toLowerCase();
  if (!value) return props.commands;
  return props.commands.filter((command) =>
    [command.label, command.group, command.keywords ?? ""].some(
      (text) => text.toLowerCase().includes(value) || trDynamic(text).toLowerCase().includes(value),
    ),
  );
});

const activeCommand = computed(() => filtered.value[activeIndex.value]);
const activeDescendant = computed(() =>
  activeCommand.value ? `topbar-option-${activeCommand.value.id}` : undefined,
);

watch(query, () => {
  activeIndex.value = 0;
});

watch(filtered, () => {
  if (activeIndex.value >= filtered.value.length) activeIndex.value = 0;
});

function open() {
  expanded.value = true;
  query.value = "";
  activeIndex.value = 0;
  void nextTick(() => inputRef.value?.focus());
}

function close(restoreFocus = false) {
  expanded.value = false;
  query.value = "";
  activeIndex.value = 0;
  if (restoreFocus) {
    void nextTick(() => toggleRef.value?.focus());
  }
}

function run(command: PaletteCommand) {
  emit("run", command);
  close(true);
}

function move(delta: number) {
  const count = filtered.value.length;
  if (count === 0) return;
  activeIndex.value = (activeIndex.value + delta + count) % count;
  void nextTick(() => {
    optionRefs.value[activeIndex.value]?.scrollIntoView?.({ block: "nearest" });
  });
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close(true);
  } else if (event.key === "ArrowDown") {
    event.preventDefault();
    move(1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    move(-1);
  } else if (event.key === "Enter") {
    event.preventDefault();
    if (activeCommand.value) run(activeCommand.value);
  }
}

function onFocusOut(event: FocusEvent) {
  const next = event.relatedTarget as Node | null;
  if (next && rootRef.value?.contains(next)) return;
  window.setTimeout(() => {
    if (!rootRef.value?.contains(document.activeElement)) close();
  }, 0);
}

defineExpose({ open });
</script>

<template>
  <div ref="rootRef" class="topbar-search" :class="{ expanded }" @focusout="onFocusOut">
    <button
      v-if="!expanded"
      ref="toggleRef"
      type="button"
      class="topbar-search-toggle"
      :aria-label="trDynamic('搜索页面和命令')"
      :title="trDynamic('搜索页面和命令')"
      aria-keyshortcuts="Control+K Meta+K"
      :aria-expanded="false"
      @click="open"
    >
      <Search :size="14" aria-hidden="true" />
      <span class="topbar-search-hint">{{ tr("搜索页面和命令") }}</span>
      <kbd class="topbar-search-kbd">Ctrl K</kbd>
    </button>

    <template v-else>
      <div class="topbar-search-field">
        <Search :size="14" class="topbar-search-icon" aria-hidden="true" />
        <input
          ref="inputRef"
          v-model="query"
          type="text"
          role="combobox"
          aria-expanded="true"
          aria-autocomplete="list"
          aria-haspopup="listbox"
          aria-controls="topbar-search-results"
          :aria-activedescendant="activeDescendant"
          :aria-label="trDynamic('搜索页面和命令')"
          :placeholder="trDynamic('搜索命令或页面…')"
          @keydown="onKeydown"
        />
        <kbd>Esc</kbd>
      </div>

      <div
        id="topbar-search-results"
        class="topbar-search-results"
        role="listbox"
        :aria-label="trDynamic('命令面板')"
      >
        <template v-for="(command, index) in filtered" :key="command.id">
          <p
            v-if="index === 0 || filtered[index - 1].group !== command.group"
            class="topbar-search-group"
            role="presentation"
          >
            {{ trDynamic(command.group) }}
          </p>
          <button
            :id="`topbar-option-${command.id}`"
            :ref="
              (element) => {
                if (element) optionRefs[index] = element as HTMLElement;
              }
            "
            type="button"
            role="option"
            tabindex="-1"
            class="topbar-search-option"
            :class="{ active: index === activeIndex }"
            :aria-selected="index === activeIndex"
            @mouseenter="activeIndex = index"
            @mousedown.prevent="run(command)"
          >
            <component :is="command.icon" v-if="command.icon" :size="15" />
            <span class="topbar-search-option-label">{{ trDynamic(command.label) }}</span>
          </button>
        </template>
        <p v-if="filtered.length === 0" class="topbar-search-empty">
          {{ tr("没有匹配的命令") }}
        </p>
      </div>
    </template>
  </div>
</template>

<style scoped>
.topbar-search {
  position: relative;
  flex: 0 0 auto;
  width: 200px;
  height: 32px;
}

.topbar-search.expanded {
  z-index: 90;
  width: 320px;
}

.topbar-search-toggle {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  width: 100%;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 0 8px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-xs);
  text-align: left;
  cursor: text;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.topbar-search-toggle:hover {
  border-color: var(--border-strong);
  background: var(--bg-subtle);
  color: var(--text-secondary);
}

.topbar-search-hint {
  flex: 1 1 auto;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.topbar-search-kbd {
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-raised);
  padding: 0 5px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 10px;
  line-height: 16px;
}

.topbar-search-toggle:focus-visible {
  border-color: var(--accent);
}

.topbar-search-field {
  position: absolute;
  top: 50%;
  right: 0;
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 320px;
  height: 34px;
  border: 1px solid var(--accent);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  box-shadow: var(--shadow-md);
  padding: 0 10px;
  transform: translateY(-50%);
}

.topbar-search-icon {
  color: var(--text-muted);
}

.topbar-search-field input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: none;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
}

.topbar-search-field input::placeholder {
  color: var(--text-muted);
}

.topbar-search-field kbd {
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  padding: 1px 5px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
}

.topbar-search-results {
  position: absolute;
  top: calc(100% + 8px);
  right: 0;
  width: 360px;
  max-height: min(52vh, 380px);
  overflow-y: auto;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  padding: var(--space-2);
}

.topbar-search-group {
  padding: var(--space-2) 8px 4px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  letter-spacing: 0.06em;
}

.topbar-search-option {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  padding: 8px;
  color: var(--text-secondary);
  font: inherit;
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
}

.topbar-search-option.active {
  background: var(--accent-soft);
  color: var(--text);
}

.topbar-search-option-label {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.topbar-search-empty {
  padding: var(--space-5) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-sm);
  text-align: center;
}

@media (max-width: 1260px) {
  .topbar-search-field {
    width: 280px;
  }

  .topbar-search-results {
    width: 320px;
  }
}
</style>
