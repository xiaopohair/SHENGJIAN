<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import { open as openDialog, save } from "@tauri-apps/plugin-dialog";
import {
  ArrowLeftRight,
  Check,
  Download,
  FileJson,
  FolderOpen,
  TriangleAlert,
  Upload,
  X,
} from "lucide-vue-next";
import {
  errorMessage,
  exportMacroLibrary,
  previewImportMacroLibrary,
  saveMacroLibrary,
} from "../lib/api";
import { tr } from "../lib/i18n";
import { libraryWithMacros, macroFileStems, mergeMacroLibraries } from "../lib/macroTransfer";
import type { MacroDefinition, MacroLibraryDocument } from "../lib/types";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";
import Notice from "./ui/Notice.vue";
import TransferProgress from "./ui/TransferProgress.vue";

/**
 * 宏库导入/导出面板（单文件或多文件）。
 *
 * 视觉与「按键方案 → 导入与导出」弹窗（`ProfileTransferDialog`）保持同一套：
 * 头部徽标 + 标题说明、方向分段控件 + 右侧提示、正文列表 / 忙碌层 / 结果面板、
 * 页脚目的地 + 汇总 + 操作按钮。
 *
 * 单文件 = 一个 JSON = 整个（或选中的）宏库，一次读写（不定态）；
 * 多文件 = 每个宏一个 JSON，按条读写（真进度）；导入按 `macroId` + `version` 合并。
 */
const props = defineProps<{
  open: boolean;
  document: MacroLibraryDocument | null;
}>();

const emit = defineEmits<{
  close: [];
  /** 导入完成后的库文档，由页面负责刷新界面。 */
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
const appliedCount = shallowRef(0);
const mergedCounts = ref({ added: 0, updated: 0, skipped: 0 });
const resultPath = shallowRef("");

const macros = computed(() => props.document?.definitions ?? []);
const pickedIds = ref<string[]>([]);
const picked = computed<MacroDefinition[]>(() =>
  macros.value.filter((macro) => pickedIds.value.includes(macro.macroId)),
);
const canSubmit = computed(() => direction.value === "import" || picked.value.length > 0);

let previousFocus: HTMLElement | null = null;

function resetState() {
  stage.value = "form";
  busyLabel.value = "";
  progress.value = null;
  failure.value = null;
  resultPath.value = "";
  appliedCount.value = 0;
  mergedCounts.value = { added: 0, updated: 0, skipped: 0 };
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

function toggleAll() {
  pickedIds.value =
    pickedIds.value.length === macros.value.length
      ? []
      : macros.value.map((macro) => macro.macroId);
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
      appliedCount.value = picked.value.length;
      resultPath.value = path;
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
    appliedCount.value = picked.value.length;
    resultPath.value = directory;
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
      appliedCount.value = saved.definitions.length;
      resultPath.value = path;
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
    const counts = { added: 0, updated: 0, skipped: 0 };
    for (const [index, path] of list.entries()) {
      const imported = await previewImportMacroLibrary(path);
      const step = mergeMacroLibraries(merged, imported);
      merged = step.document;
      counts.added += step.added;
      counts.updated += step.updated;
      counts.skipped += step.skipped;
      if (progress.value) progress.value = { done: index + 1, total: list.length };
    }
    const saved = await saveMacroLibrary(merged);
    mergedCounts.value = counts;
    appliedCount.value = counts.added + counts.updated + counts.skipped;
    emit("applied", saved);
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

function submit() {
  if (stage.value === "busy" || !canSubmit.value) return;
  failure.value = null;
  if (direction.value === "export") void runExport();
  else void runImport();
}

function setDirection(next: Direction) {
  if (direction.value === next) return;
  direction.value = next;
  stage.value = "form";
  failure.value = null;
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

const resultTitle = computed(() => {
  if (direction.value === "export") return tr("已导出 {count} 个宏", { count: appliedCount.value });
  return tr("已导入 {count} 个宏", { count: appliedCount.value });
});

const resultCopy = computed(() => {
  if (direction.value === "export") {
    return format.value === "single"
      ? tr("已写出 1 个文件")
      : tr("已写出 {count} 个文件", { count: appliedCount.value });
  }
  if (format.value === "single") return tr("整库已替换");
  return tr("新增 {added} · 更新 {updated} · 跳过 {skipped}", mergedCounts.value);
});

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
          <span class="head-badge" aria-hidden="true"><ArrowLeftRight :size="17" /></span>
          <div class="head-copy">
            <h2 id="macro-transfer-title">{{ tr("导入与导出宏") }}</h2>
            <p>
              {{
                direction === "import"
                  ? tr("把别的电脑上的宏带过来，先看清会覆盖还是新增。")
                  : tr("单文件是整个宏库；多文件是每个宏一个文件，可以挑着导出。")
              }}
            </p>
          </div>
          <IconButton size="sm" :label="tr('关闭')" @click="close">
            <X :size="16" />
          </IconButton>
        </header>

        <div v-if="stage === 'form' || stage === 'busy'" class="transfer-switch">
          <div class="transfer-seg" role="tablist" :aria-label="tr('导入或导出')">
            <button
              type="button"
              role="tab"
              :aria-pressed="direction === 'export'"
              @click="setDirection('export')"
            >
              <Download :size="13" aria-hidden="true" />
              {{ tr("导出") }}
            </button>
            <button
              type="button"
              role="tab"
              :aria-pressed="direction === 'import'"
              @click="setDirection('import')"
            >
              <Upload :size="13" aria-hidden="true" />
              {{ tr("导入") }}
            </button>
          </div>
          <span class="switch-hint">
            {{
              direction === "export" ? tr("导出只读，不改动本机宏库") : tr("导入前先看会怎么合并")
            }}
          </span>
        </div>

        <div class="transfer-body">
          <template v-if="stage === 'done' || stage === 'error'">
            <div class="transfer-result" :class="stage === 'error' ? 'is-error' : 'is-done'">
              <span class="result-badge" aria-hidden="true">
                <TriangleAlert v-if="stage === 'error'" :size="24" :stroke-width="1.9" />
                <Check v-else :size="24" :stroke-width="2.2" />
              </span>
              <template v-if="stage === 'error'">
                <h3 class="result-title">{{ tr("操作失败，请重试") }}</h3>
                <p class="result-copy">{{ failure }}</p>
              </template>
              <template v-else>
                <h3 class="result-title">{{ resultTitle }}</h3>
                <p class="result-copy">{{ resultCopy }}</p>
                <p v-if="resultPath" class="result-path" :title="resultPath">{{ resultPath }}</p>
              </template>
              <div class="result-actions">
                <Button size="sm" variant="primary" @click="close">{{ tr("知道了") }}</Button>
              </div>
            </div>
          </template>

          <template v-else>
            <div class="transfer-split">
              <div class="macro-list-pane">
                <template v-if="direction === 'export'">
                  <div class="pane-head">
                    <p class="pane-label">{{ tr("要导出的宏") }}</p>
                    <div class="pane-head-actions">
                      <span class="pane-count">{{ picked.length }} / {{ macros.length }}</span>
                      <button type="button" class="link-button" @click="toggleAll">
                        {{ pickedIds.length === macros.length ? tr("全不选") : tr("全选") }}
                      </button>
                    </div>
                  </div>
                  <ul class="macro-rows">
                    <li
                      v-for="macro in macros"
                      :key="macro.macroId"
                      class="macro-row"
                      :class="{ selected: pickedIds.includes(macro.macroId) }"
                      tabindex="0"
                      role="checkbox"
                      :aria-checked="pickedIds.includes(macro.macroId)"
                      @click="togglePick(macro.macroId)"
                      @keydown.enter.prevent="togglePick(macro.macroId)"
                      @keydown.space.prevent="togglePick(macro.macroId)"
                    >
                      <span class="row-check" aria-hidden="true" />
                      <span class="row-main">
                        <span class="row-name">{{ macro.name }}</span>
                      </span>
                      <span class="row-meta">
                        {{ tr("{count} 步", { count: macro.steps.length }) }} · v{{ macro.version }}
                      </span>
                    </li>
                  </ul>
                </template>

                <template v-else>
                  <div class="pane-head">
                    <p class="pane-label">{{ tr("本机宏库") }}</p>
                    <span class="pane-count">{{ macros.length }} {{ tr("个宏") }}</span>
                  </div>
                  <div class="import-hint">
                    <Notice v-if="format === 'single'" kind="warning">
                      {{ tr("单文件导入会整库替换当前宏库。") }}
                    </Notice>
                    <Notice v-else kind="info">
                      {{
                        tr("多文件导入会按宏合并：同 ID 同版本跳过，同 ID 新版本覆盖，新 ID 追加。")
                      }}
                    </Notice>
                  </div>
                </template>
              </div>

              <div class="option-pane">
                <div class="pane-head">
                  <p class="pane-label">{{ tr("文件个数") }}</p>
                </div>
                <div class="format-seg" role="radiogroup" :aria-label="tr('文件个数')">
                  <button
                    type="button"
                    role="radio"
                    :aria-checked="format === 'single'"
                    class="format-option"
                    :class="{ selected: format === 'single' }"
                    @click="format = 'single'"
                  >
                    <FileJson :size="15" aria-hidden="true" />
                    <span class="format-name">{{ tr("单文件") }}</span>
                    <span class="format-hint">{{ tr("整个宏库打包成一个 JSON") }}</span>
                  </button>
                  <button
                    type="button"
                    role="radio"
                    :aria-checked="format === 'multi'"
                    class="format-option"
                    :class="{ selected: format === 'multi' }"
                    @click="format = 'multi'"
                  >
                    <FolderOpen :size="15" aria-hidden="true" />
                    <span class="format-name">{{ tr("多文件") }}</span>
                    <span class="format-hint">{{ tr("每个宏一个 JSON，按条推进") }}</span>
                  </button>
                </div>
                <p class="option-note">
                  {{
                    format === "single"
                      ? tr("一个 JSON，适合整库备份与迁移。")
                      : tr("按宏拆分，适合只带走或只合并某几个宏。")
                  }}
                </p>
              </div>
            </div>
          </template>

          <div v-if="stage === 'busy'" class="transfer-busy">
            <TransferProgress
              :label="busyLabel"
              :done="progress?.done ?? null"
              :total="progress?.total ?? null"
            />
          </div>
        </div>

        <footer v-if="stage === 'form'" class="transfer-foot">
          <p class="dest-field is-static">
            <FolderOpen :size="13" aria-hidden="true" />
            <span>
              <template v-if="direction === 'export'">
                {{ format === "single" ? tr("导出为一个文件") : tr("导出到一个文件夹") }}
              </template>
              <template v-else>
                {{ format === "single" ? tr("从一个文件导入") : tr("从多个文件导入") }}
              </template>
            </span>
          </p>
          <p class="foot-summary">
            <template v-if="direction === 'export'">
              <b>{{ picked.length }}</b>
              {{ tr("个宏") }} · {{ format === "single" ? tr("单文件") : tr("多文件") }}
            </template>
            <template v-else>{{ format === "single" ? tr("单文件") : tr("多文件") }}</template>
          </p>
          <div class="foot-actions">
            <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
            <Button size="sm" variant="primary" :disabled="!canSubmit" @click="submit">
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
  padding: var(--space-4);
  background: var(--overlay);
  backdrop-filter: blur(4px);
}

.transfer-dialog {
  display: flex;
  flex-direction: column;
  width: min(920px, calc(100vw - 48px));
  max-height: calc(100vh - 40px);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  outline: none;
}

.transfer-head {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-4) var(--space-3) var(--space-5);
}

.head-badge {
  display: grid;
  flex: 0 0 32px;
  place-items: center;
  width: 32px;
  height: 32px;
  border: 1px solid color-mix(in srgb, var(--accent) 26%, transparent);
  border-radius: var(--radius-md);
  background: var(--accent-soft);
  color: var(--accent);
}

.head-copy {
  min-width: 0;
  flex: 1 1 auto;
}

.head-copy h2 {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.head-copy p {
  margin: 3px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.transfer-switch {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-5) var(--space-3);
}

.transfer-seg {
  display: inline-flex;
  gap: 2px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 3px;
}

.transfer-seg button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  height: 26px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  padding: 0 16px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-xs);
  font-weight: 600;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.transfer-seg button:hover {
  color: var(--text-secondary);
}

.transfer-seg button[aria-pressed="true"] {
  background: var(--bg-elevated);
  box-shadow:
    0 1px 2px color-mix(in srgb, var(--shadow) 25%, transparent),
    0 0 0 1px var(--border);
  color: var(--text);
}

.switch-hint {
  margin-left: auto;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.transfer-body {
  position: relative;
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  min-height: 400px;
  border-top: 1px solid var(--border);
  overflow: hidden;
}

.transfer-split {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 300px;
  flex: 1 1 auto;
  min-height: 0;
}

.macro-list-pane {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-height: 0;
  padding: var(--space-3) 0 0;
}

.option-pane {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-height: 0;
  border-left: 1px solid var(--border);
  background: color-mix(in srgb, var(--bg-subtle) 55%, transparent);
  padding: var(--space-3) var(--space-4) var(--space-4);
}

.option-note {
  margin: var(--space-2) 0 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.6;
}

.import-hint {
  padding: 0 var(--space-4);
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: 0 var(--space-4);
}

.pane-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 600;
  letter-spacing: 0.06em;
}

.pane-head-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.pane-count {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

.link-button {
  border: 0;
  border-radius: var(--radius-xs);
  background: transparent;
  padding: 2px 5px;
  color: var(--accent);
  font: inherit;
  font-size: var(--text-2xs);
  font-weight: 600;
  cursor: pointer;
}

.link-button:hover {
  background: var(--accent-soft);
}

.format-seg {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: var(--space-2);
}

.format-option {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr);
  grid-template-rows: auto auto;
  gap: 2px 8px;
  align-items: center;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg);
  padding: 9px 11px;
  color: var(--text-muted);
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.format-option svg {
  grid-row: span 2;
  color: var(--text-muted);
}

.format-name {
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 600;
}

.format-hint {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.format-option:hover {
  border-color: var(--border-strong);
}

.format-option.selected {
  border-color: color-mix(in srgb, var(--accent) 45%, var(--border));
  background: color-mix(in srgb, var(--accent) 8%, transparent);
}

.format-option.selected svg,
.format-option.selected .format-hint {
  color: var(--accent);
}

.macro-rows {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: 1px;
  min-height: 0;
  margin: 0;
  padding: 0 var(--space-2) var(--space-2);
  overflow-y: auto;
  list-style: none;
  scrollbar-width: thin;
}

.macro-row {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  padding: 8px 9px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--duration-fast) ease;
}

.macro-row:hover {
  background: var(--bg-hover);
}

.macro-row.selected {
  background: color-mix(in srgb, var(--accent) 9%, transparent);
}

.macro-row.selected:hover {
  background: color-mix(in srgb, var(--accent) 14%, transparent);
}

.macro-row:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.row-check {
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border: 1px solid var(--border-control);
  border-radius: 4.5px;
  background: var(--bg);
  color: transparent;
  font-size: 10px;
}

.row-check::after {
  content: "";
  width: 8px;
  height: 4px;
  border-bottom: 2px solid currentColor;
  border-left: 2px solid currentColor;
  transform: rotate(-45deg) translateY(-1px);
}

.selected .row-check {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--text-on-accent);
}

.row-main {
  min-width: 0;
}

.row-name {
  display: block;
  overflow: hidden;
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-meta {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}

.transfer-busy {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  background: color-mix(in srgb, var(--bg-raised) 92%, transparent);
  backdrop-filter: blur(2px);
}

.transfer-result {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  padding: var(--space-6) var(--space-5);
  text-align: center;
}

.result-badge {
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: var(--accent-soft);
  color: var(--accent);
}

.transfer-result.is-error .result-badge {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  color: var(--danger);
}

.result-title {
  margin: 0;
  color: var(--text);
  font-size: var(--text-md);
  font-weight: 650;
}

.result-copy {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.6;
}

.result-path {
  margin: 0;
  max-width: 100%;
  overflow: hidden;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.transfer-foot {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-4);
}

.dest-field {
  display: flex;
  flex: 0 0 260px;
  align-items: center;
  gap: 6px;
  min-width: 0;
  height: 32px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg);
  padding: 0 9px;
  color: var(--text-muted);
}

.dest-field.is-static {
  margin: 0;
  font-size: var(--text-2xs);
}

.foot-summary {
  min-width: 0;
  margin: 0;
  overflow: hidden;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.foot-summary b {
  color: var(--text-secondary);
  font-weight: 600;
}

.foot-actions {
  display: flex;
  flex: 0 0 auto;
  gap: var(--space-2);
  margin-left: auto;
}
</style>
