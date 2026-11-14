<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import { open as openDialog, save } from "@tauri-apps/plugin-dialog";
import { ArrowDownToLine, ArrowUpFromLine, FileJson, FolderOpen, X } from "lucide-vue-next";
import {
  errorMessage,
  exportMacroLibrary,
  previewImportMacroLibrary,
  saveMacroLibrary,
} from "../lib/api";
import { tr } from "../lib/i18n";
import { macroFileStems, libraryWithMacros, mergeMacroLibraries } from "../lib/macroTransfer";
import type { MacroDefinition, MacroLibraryDocument } from "../lib/types";
import Button from "./ui/Button.vue";
import Checkbox from "./ui/Checkbox.vue";
import IconButton from "./ui/IconButton.vue";
import Notice from "./ui/Notice.vue";
import TransferProgress from "./ui/TransferProgress.vue";

/**
 * 宏库导入/导出面板（单文件或多文件）。
 *
 * 单文件：一个 JSON = 整个（或选中的）宏库，一次读写（进度为不定态）。
 * 多文件：每个宏一个 JSON，导出按条写、导入按条读，因此有真实进度条；
 * 导入时按 `macroId` + `version` 合并，冲突策略见 `lib/macroTransfer.ts`。
 */
const props = defineProps<{
  open: boolean;
  document: MacroLibraryDocument | null;
}>();

const emit = defineEmits<{
  close: [];
  /** 导入完成后的库文档，由页面负责落盘与刷新。 */
  applied: [document: MacroLibraryDocument];
}>();

type Direction = "export" | "import";
type Format = "single" | "multi";
type Stage = "form" | "busy" | "done" | "error";

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const direction = ref<Direction>("export");
const format = ref<Format>("single");
const stage = ref<Stage>("form");
const busyLabel = ref("");
const progress = ref<{ done: number; total: number } | null>(null);
const failure = shallowRef<string | null>(null);
const result = shallowRef<string | null>(null);
const pickedIds = ref<string[]>([]);
const replaceAll = ref(false);

const macros = computed(() => props.document?.definitions ?? []);
const picked = computed<MacroDefinition[]>(() =>
  macros.value.filter((macro) => pickedIds.value.includes(macro.macroId)),
);
const canSubmit = computed(() => {
  if (direction.value === "import") return true;
  return picked.value.length > 0;
});

let previousFocus: HTMLElement | null = null;

function resetState() {
  stage.value = "form";
  busyLabel.value = "";
  progress.value = null;
  failure.value = null;
  result.value = null;
  replaceAll.value = false;
  pickedIds.value = macros.value.map((macro) => macro.macroId);
}

function beginBusy(label: string, total: number | null = null) {
  busyLabel.value = label;
  progress.value = total === null ? null : { done: 0, total };
  stage.value = "busy";
}

function failWith(cause: unknown) {
  failure.value = errorMessage(cause);
  stage.value = "error";
}

function togglePick(macroId: string) {
  pickedIds.value = pickedIds.value.includes(macroId)
    ? pickedIds.value.filter((id) => id !== macroId)
    : [...pickedIds.value, macroId];
}

function pickAll() {
  pickedIds.value = macros.value.map((macro) => macro.macroId);
}

function pickNone() {
  pickedIds.value = [];
}

async function runExport() {
  const document = props.document;
  if (!document || picked.value.length === 0) return;
  try {
    if (format.value === "single") {
      const path = await save({
        defaultPath: `${tr("宏库")}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      beginBusy(tr("正在导出…"));
      await exportMacroLibrary(path, libraryWithMacros(document, picked.value));
      result.value = tr("已导出 {count} 个宏到单个文件。", { count: picked.value.length });
      stage.value = "done";
      return;
    }

    const directory = await openDialog({ directory: true, multiple: false });
    if (!directory || Array.isArray(directory)) return;
    beginBusy(tr("正在导出…"), picked.value.length);
    const stems = macroFileStems(picked.value);
    for (const [index, macro] of picked.value.entries()) {
      await exportMacroLibrary(
        `${directory}\\${stems[index]}.json`,
        libraryWithMacros(document, [macro]),
      );
      if (progress.value) progress.value = { done: index + 1, total: picked.value.length };
    }
    result.value = tr("已导出 {count} 个宏，每个宏一个文件。", { count: picked.value.length });
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

async function runImport() {
  const current = props.document;
  if (!current) return;
  try {
    if (format.value === "single") {
      const path = await openDialog({
        multiple: false,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path || Array.isArray(path)) return;
      beginBusy(tr("正在导入…"));
      const imported = await previewImportMacroLibrary(path);
      const saved = await saveMacroLibrary(imported);
      result.value = tr("已导入 {count} 个宏（整库替换）。", { count: saved.definitions.length });
      emit("applied", saved);
      stage.value = "done";
      return;
    }

    const paths = await openDialog({
      multiple: true,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!paths) return;
    const list = Array.isArray(paths) ? paths : [paths];
    if (list.length === 0) return;

    beginBusy(tr("正在导入…"), list.length);
    let merged = current;
    let added = 0;
    let updated = 0;
    let skipped = 0;
    for (const [index, path] of list.entries()) {
      const imported = await previewImportMacroLibrary(path);
      const step = mergeMacroLibraries(merged, imported);
      merged = step.document;
      added += step.added;
      updated += step.updated;
      skipped += step.skipped;
      if (progress.value) progress.value = { done: index + 1, total: list.length };
    }
    const saved = await saveMacroLibrary(merged);
    emit("applied", saved);
    result.value = tr("已导入：新增 {added} 个，更新 {updated} 个，跳过 {skipped} 个。", {
      added,
      updated,
      skipped,
    });
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

function submit() {
  if (stage.value === "busy") return;
  stage.value = "form";
  failure.value = null;
  result.value = null;
  if (direction.value === "export") void runExport();
  else void runImport();
}

function close() {
  emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    close();
  }
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      resetState();
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

watch(direction, () => {
  stage.value = "form";
  failure.value = null;
  result.value = null;
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="transfer-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="transfer-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="macro-transfer-title"
        tabindex="-1"
      >
        <header class="transfer-head">
          <div class="transfer-heading">
            <h2 id="macro-transfer-title">
              <FileJson :size="17" aria-hidden="true" />
              {{ tr("导入 / 导出宏") }}
            </h2>
            <p>{{ tr("单文件是整个宏库；多文件是每个宏一个文件，可以挑着导入导出。") }}</p>
          </div>
          <IconButton size="sm" :label="tr('关闭')" @click="close">
            <X :size="16" />
          </IconButton>
        </header>

        <div class="transfer-body">
          <div class="transfer-tabs" role="tablist">
            <button
              type="button"
              role="tab"
              class="transfer-tab"
              :class="{ selected: direction === 'export' }"
              :aria-selected="direction === 'export'"
              @click="direction = 'export'"
            >
              <ArrowDownToLine :size="14" />
              {{ tr("导出") }}
            </button>
            <button
              type="button"
              role="tab"
              class="transfer-tab"
              :class="{ selected: direction === 'import' }"
              :aria-selected="direction === 'import'"
              @click="direction = 'import'"
            >
              <ArrowUpFromLine :size="14" />
              {{ tr("导入") }}
            </button>
          </div>

          <template v-if="stage === 'form'">
            <div class="transfer-field">
              <p class="transfer-field-label">{{ tr("文件个数") }}</p>
              <div class="transfer-formats">
                <button
                  type="button"
                  class="transfer-format"
                  :class="{ selected: format === 'single' }"
                  :aria-pressed="format === 'single'"
                  @click="format = 'single'"
                >
                  {{ tr("单文件") }}
                  <span>{{ tr("整个宏库打包成一个 JSON") }}</span>
                </button>
                <button
                  type="button"
                  class="transfer-format"
                  :class="{ selected: format === 'multi' }"
                  :aria-pressed="format === 'multi'"
                  @click="format = 'multi'"
                >
                  {{ tr("多文件") }}
                  <span>{{ tr("每个宏一个 JSON，按条推进") }}</span>
                </button>
              </div>
            </div>

            <div v-if="direction === 'export'" class="transfer-field">
              <div class="transfer-field-head">
                <p class="transfer-field-label">
                  {{ tr("要导出的宏") }} · {{ tr("{count} 个", { count: picked.length }) }}
                </p>
                <div class="transfer-field-actions">
                  <Button size="sm" variant="ghost" @click="pickAll">{{ tr("全选") }}</Button>
                  <Button size="sm" variant="ghost" @click="pickNone">{{ tr("全不选") }}</Button>
                </div>
              </div>
              <ul class="transfer-list">
                <li v-for="macro in macros" :key="macro.macroId" class="transfer-row">
                  <Checkbox
                    :model-value="pickedIds.includes(macro.macroId)"
                    @update:model-value="togglePick(macro.macroId)"
                  >
                    <span class="transfer-row-name">{{ macro.name }}</span>
                  </Checkbox>
                  <span class="transfer-row-meta">
                    {{ tr("{count} 步", { count: macro.steps.length }) }} · v{{ macro.version }}
                  </span>
                </li>
              </ul>
            </div>

            <Notice v-else-if="format === 'single'" kind="warning">
              {{ tr("单文件导入会整库替换当前宏库。") }}
            </Notice>
            <Notice v-else kind="info">
              {{ tr("多文件导入会按宏合并：同 ID 同版本跳过，同 ID 新版本覆盖，新 ID 追加。") }}
            </Notice>
          </template>

          <TransferProgress
            v-else-if="stage === 'busy'"
            :label="busyLabel"
            :done="progress?.done ?? null"
            :total="progress?.total ?? null"
          />

          <Notice v-else-if="stage === 'done'" kind="success">{{ result }}</Notice>
          <Notice v-else-if="stage === 'error'" kind="error">{{ failure }}</Notice>
        </div>

        <footer class="transfer-foot">
          <div class="transfer-hint">
            <FolderOpen :size="13" aria-hidden="true" />
            <span v-if="direction === 'export'">
              {{ format === "single" ? tr("导出为一个文件") : tr("导出到一个文件夹") }}
            </span>
            <span v-else>
              {{ format === "single" ? tr("从一个文件导入") : tr("从多个文件导入") }}
            </span>
          </div>
          <div class="transfer-actions">
            <Button variant="ghost" @click="close">
              {{ stage === "done" ? tr("关闭") : tr("取消") }}
            </Button>
            <Button
              v-if="stage !== 'done'"
              variant="primary"
              :loading="stage === 'busy'"
              :disabled="!canSubmit"
              @click="submit"
            >
              {{ direction === "export" ? tr("开始导出") : tr("开始导入") }}
            </Button>
          </div>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.transfer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  background: color-mix(in srgb, var(--bg) 62%, transparent);
  backdrop-filter: blur(2px);
}

.transfer-dialog {
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

.transfer-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.transfer-heading h2 {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  color: var(--text);
  font-size: var(--text-lg);
  font-weight: 650;
}

.transfer-heading p {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.transfer-body {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
  padding: var(--space-4) var(--space-5);
}

.transfer-tabs {
  display: inline-flex;
  gap: 4px;
  align-self: flex-start;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  padding: 3px;
}

.transfer-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 0;
  border-radius: var(--radius-xs);
  background: transparent;
  padding: 5px 12px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
}

.transfer-tab.selected {
  background: var(--bg-elevated);
  color: var(--text);
  font-weight: 600;
  box-shadow: var(--shadow-sm);
}

.transfer-field {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.transfer-field-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.transfer-field-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.transfer-field-actions {
  display: flex;
  gap: 4px;
}

.transfer-formats {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: var(--space-2);
}

.transfer-format {
  display: flex;
  flex-direction: column;
  gap: 4px;
  align-items: flex-start;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  padding: var(--space-3);
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  font-weight: 600;
  text-align: left;
  cursor: pointer;
}

.transfer-format span {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 400;
}

.transfer-format.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

.transfer-format.selected span {
  color: var(--accent);
}

.transfer-list {
  display: flex;
  flex-direction: column;
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  max-height: 240px;
  overflow-y: auto;
}

.transfer-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: 6px var(--space-3);
}

.transfer-row + .transfer-row {
  border-top: 1px solid var(--border);
}

.transfer-row-name {
  color: var(--text);
  font-size: var(--text-sm);
}

.transfer-row-meta {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
  white-space: nowrap;
}

.transfer-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-5);
}

.transfer-hint {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.transfer-actions {
  display: flex;
  flex: 0 0 auto;
  gap: var(--space-2);
}
</style>
