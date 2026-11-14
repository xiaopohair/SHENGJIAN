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
import {
  buildMacroImportRows,
  collectIncomingMacros,
  composeMacroImportDocument,
  libraryWithMacros,
  localOnlyMacros,
  macroFileStems,
  macroImportCounts,
  resolveMacroImportRows,
  type MacroImportCounts,
  type MacroImportMode,
  type MacroImportPolicy,
} from "../lib/macroTransfer";
import type { MacroDefinition, MacroLibraryDocument } from "../lib/types";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";
import TransferProgress from "./ui/TransferProgress.vue";

/**
 * 宏库导入/导出面板，和「按键方案 → 导入与导出」保持同一套骨架与手势。
 *
 * 导出：挑宏 → 单文件 / 多文件 → 写盘（多文件按条报进度）。
 * 导入：先选文件 → 预检逐条对比（新增 / 更新 / 相同）→ 选合并或整库替换、
 * 同 ID 新版本覆盖还是保留 → 落地 → 可撤销。
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

interface ImportFile {
  path: string;
  name: string;
  document: MacroLibraryDocument;
}

type TransferResult =
  | { kind: "export"; count: number; path: string; single: boolean }
  | {
      kind: "import";
      count: number;
      counts: MacroImportCounts;
      /** 整库替换时被删掉的本机宏名字，结果页要逐条列出来。 */
      dropped: string[];
      mode: MacroImportMode;
      path: string;
    };

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const direction = ref<Direction>("export");
const format = ref<Format>("single");
const stage = ref<Stage>("form");
const busyLabel = ref("");
const progress = ref<{ done: number; total: number } | null>(null);
const failure = shallowRef<string | null>(null);
const result = shallowRef<TransferResult | null>(null);

const pickedIds = ref<string[]>([]);
const importFiles = shallowRef<ImportFile[]>([]);
const importMode = ref<MacroImportMode>("merge");
const importPolicy = ref<MacroImportPolicy>("overwrite");
const rowOverrides = ref<Record<string, MacroImportPolicy>>({});
/** 导入前的库快照：撤销时原样写回去。 */
const previousDocument = shallowRef<MacroLibraryDocument | null>(null);

const macros = computed(() => props.document?.definitions ?? []);
const picked = computed<MacroDefinition[]>(() =>
  macros.value.filter((macro) => pickedIds.value.includes(macro.macroId)),
);

const incoming = computed(() =>
  collectIncomingMacros(importFiles.value.map((file) => file.document)),
);
const incomingFormatVersion = computed(() => {
  const versions = importFiles.value.map((file) => file.document.formatVersion);
  return versions.length ? Math.max(...versions) : 1;
});
const importRows = computed(() => buildMacroImportRows(incoming.value, macros.value));
const resolvedRows = computed(() =>
  resolveMacroImportRows(importRows.value, importPolicy.value, rowOverrides.value),
);
const counts = computed(() => macroImportCounts(resolvedRows.value));
const droppedMacros = computed(() =>
  importMode.value === "replace" ? localOnlyMacros(macros.value, incoming.value) : [],
);

/** 单文件才提供「整库替换」；多文件是逐条合并，没有替换语义。 */
const effectiveMode = computed<MacroImportMode>(() =>
  format.value === "single" ? importMode.value : "merge",
);
const pendingCount = computed(() =>
  effectiveMode.value === "replace"
    ? importRows.value.length
    : counts.value.added + counts.value.updated,
);

const canSubmit = computed(() => {
  if (stage.value === "busy") return false;
  if (direction.value === "export") return picked.value.length > 0;
  return importFiles.value.length > 0;
});

let previousFocus: HTMLElement | null = null;

function resetState() {
  direction.value = "export";
  format.value = "single";
  stage.value = "form";
  busyLabel.value = "";
  progress.value = null;
  failure.value = null;
  result.value = null;
  pickedIds.value = macros.value.map((macro) => macro.macroId);
  importFiles.value = [];
  importMode.value = "merge";
  importPolicy.value = "overwrite";
  rowOverrides.value = {};
  previousDocument.value = null;
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

function fileNameOf(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function setRowPolicy(macroId: string, policy: MacroImportPolicy) {
  rowOverrides.value = { ...rowOverrides.value, [macroId]: policy };
}

async function runExport() {
  const document = props.document;
  if (!document || picked.value.length === 0) return;

  if (format.value === "single") {
    try {
      const path = await save({
        defaultPath: `${tr("宏库")}.json`,
        filters: [{ name: "JSON", extensions: ["json"] }],
      });
      if (!path) return;
      beginBusy(tr("正在导出…"));
      await exportMacroLibrary(path, libraryWithMacros(document, picked.value));
      result.value = { kind: "export", count: picked.value.length, path, single: true };
      stage.value = "done";
    } catch (cause) {
      failWith(cause);
    }
    return;
  }

  try {
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
    result.value = { kind: "export", count: picked.value.length, path: directory, single: false };
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

async function chooseImportFiles() {
  try {
    const paths = await openDialog({
      multiple: format.value === "multi",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!paths) return;
    const list = Array.isArray(paths) ? paths : [paths];
    if (list.length === 0) return;

    beginBusy(tr("正在读取文件…"), list.length);
    const files: ImportFile[] = [];
    for (const [index, path] of list.entries()) {
      files.push({ path, name: fileNameOf(path), document: await previewImportMacroLibrary(path) });
      if (progress.value) progress.value = { done: index + 1, total: list.length };
    }
    importFiles.value = files;
    rowOverrides.value = {};
    stage.value = "form";
  } catch (cause) {
    failWith(cause);
  }
}

async function applyImport() {
  const current = props.document;
  if (!current || importFiles.value.length === 0) return;
  const mode = effectiveMode.value;
  const next = composeMacroImportDocument(
    current,
    resolvedRows.value,
    mode,
    incomingFormatVersion.value,
  );

  try {
    beginBusy(tr("正在导入…"));
    const saved = await saveMacroLibrary(next);
    previousDocument.value = current;
    result.value = {
      kind: "import",
      count:
        mode === "replace" ? importRows.value.length : counts.value.added + counts.value.updated,
      counts: { ...counts.value },
      dropped: droppedMacros.value.map((macro) => macro.name),
      mode,
      path: importFiles.value.map((file) => file.name).join("、"),
    };
    emit("applied", saved);
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

async function undoImport() {
  if (!previousDocument.value) return;
  const restore = previousDocument.value;
  try {
    beginBusy(tr("正在撤销…"));
    const saved = await saveMacroLibrary(restore);
    previousDocument.value = null;
    result.value = null;
    emit("applied", saved);
    stage.value = "form";
  } catch (cause) {
    failWith(cause);
  }
}

function submit() {
  if (!canSubmit.value) return;
  failure.value = null;
  if (direction.value === "export") void runExport();
  else if (importFiles.value.length) void applyImport();
  else void chooseImportFiles();
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
    return;
  }
  if (event.key !== "Enter") return;
  const target = event.target as HTMLElement | null;
  if (target && (target.tagName === "INPUT" || target.tagName === "TEXTAREA")) return;
  if (stage.value !== "form") return;
  event.preventDefault();
  submit();
}

const resultTitle = computed(() => {
  const value = result.value;
  if (!value) return "";
  return value.kind === "export"
    ? tr("已导出 {count} 个宏", { count: value.count })
    : tr("已导入 {count} 个宏", { count: value.count });
});

const resultCopy = computed(() => {
  const value = result.value;
  if (!value) return "";
  if (value.kind === "export") {
    return value.single
      ? tr("已写出 1 个文件")
      : tr("已写出 {count} 个文件", { count: value.count });
  }
  const parts: string[] = [];
  if (value.mode === "replace") {
    parts.push(tr("整库已替换"));
    if (value.dropped.length) parts.push(tr("删除 {count} 个", { count: value.dropped.length }));
  } else {
    if (value.counts.added) parts.push(tr("新增 {count}", { count: value.counts.added }));
    if (value.counts.updated) parts.push(tr("更新 {count}", { count: value.counts.updated }));
    if (value.counts.kept) parts.push(tr("保留 {count}", { count: value.counts.kept }));
    if (value.counts.same) parts.push(tr("相同 {count}", { count: value.counts.same }));
  }
  return parts.join(" · ");
});

/** 结果页里要列出来的「被删掉的本机宏」。 */
const droppedNames = computed(() => {
  const value = result.value;
  return value && value.kind === "import" ? value.dropped : [];
});

const importSummary = computed(() => {
  const parts: string[] = [];
  if (counts.value.added) parts.push(tr("新增 {count}", { count: counts.value.added }));
  if (counts.value.updated) parts.push(tr("更新 {count}", { count: counts.value.updated }));
  if (counts.value.kept) parts.push(tr("保留 {count}", { count: counts.value.kept }));
  if (counts.value.same) parts.push(tr("相同 {count}", { count: counts.value.same }));
  return parts.join(" · ");
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
  { immediate: true },
);

watch(format, (value) => {
  if (value !== "single") importMode.value = "merge";
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
                <div class="result-actions">
                  <Button size="sm" variant="primary" @click="close">{{ tr("知道了") }}</Button>
                </div>
              </template>
              <template v-else>
                <h3 class="result-title">{{ resultTitle }}</h3>
                <p class="result-copy">{{ resultCopy }}</p>
                <p v-if="result?.path" class="result-path" :title="result.path">
                  {{ result.path }}
                </p>
                <div v-if="droppedNames.length" class="result-dropped">
                  <p class="result-dropped-label">{{ tr("被删掉的本机宏") }}</p>
                  <ul class="dropped-list">
                    <li v-for="name in droppedNames" :key="name">{{ name }}</li>
                  </ul>
                </div>
                <div class="result-actions">
                  <Button v-if="previousDocument" size="sm" @click="undoImport">
                    {{ tr("撤销导入") }}
                  </Button>
                  <Button size="sm" variant="primary" @click="close">{{ tr("知道了") }}</Button>
                </div>
              </template>
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
                  <ul
                    class="macro-rows"
                    role="listbox"
                    aria-multiselectable="true"
                    :aria-label="tr('选择要导出的宏')"
                  >
                    <li
                      v-for="macro in macros"
                      :key="macro.macroId"
                      class="macro-row"
                      :class="{ selected: pickedIds.includes(macro.macroId) }"
                      tabindex="0"
                      role="option"
                      :aria-selected="pickedIds.includes(macro.macroId)"
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
                  <div v-if="!importFiles.length" class="pane-head">
                    <p class="pane-label">{{ tr("导入") }}</p>
                  </div>

                  <button
                    v-if="!importFiles.length"
                    type="button"
                    class="drop-zone"
                    @click="chooseImportFiles"
                  >
                    <span class="drop-icon"><Upload :size="20" aria-hidden="true" /></span>
                    <strong>
                      {{
                        format === "single" ? tr("选择一个宏库文件") : tr("选择宏文件（可多选）")
                      }}
                    </strong>
                    <span class="drop-hint">{{ tr("支持 .json；单个宏文件也是合法的库。") }}</span>
                  </button>

                  <template v-else>
                    <div v-for="file in importFiles" :key="file.path" class="file-card">
                      <span class="file-icon"><FileJson :size="16" aria-hidden="true" /></span>
                      <span class="file-copy">
                        <span class="file-name" :title="file.path">{{ file.name }}</span>
                        <span class="file-meta">
                          {{ tr("{count} 个宏", { count: file.document.definitions.length }) }}
                        </span>
                      </span>
                    </div>

                    <div class="pane-head">
                      <p class="pane-label">{{ tr("将导入") }}</p>
                      <span class="pane-count">{{ importRows.length }} {{ tr("个宏") }}</span>
                    </div>

                    <ul class="macro-rows">
                      <li
                        v-for="row in resolvedRows"
                        :key="row.macro.macroId"
                        class="macro-row import-row"
                      >
                        <span class="status-chip" :class="`status-${row.status}`">
                          {{
                            row.status === "new"
                              ? tr("新增")
                              : row.status === "update"
                                ? tr("更新")
                                : tr("相同")
                          }}
                        </span>
                        <span class="row-main">
                          <span class="row-name">{{ row.macro.name }}</span>
                          <span class="row-meta">
                            {{ tr("{count} 步", { count: row.macro.steps.length }) }} · v{{
                              row.macro.version
                            }}
                            <template
                              v-if="row.localVersion && row.localVersion !== row.macro.version"
                            >
                              （{{ tr("本机 v{version}", { version: row.localVersion }) }}）
                            </template>
                          </span>
                        </span>
                        <span class="row-side">
                          <span
                            v-if="row.status === 'update' && effectiveMode === 'merge'"
                            class="mini-segment"
                            role="group"
                          >
                            <button
                              type="button"
                              :aria-pressed="row.action === 'overwrite'"
                              @click="setRowPolicy(row.macro.macroId, 'overwrite')"
                            >
                              {{ tr("覆盖") }}
                            </button>
                            <button
                              type="button"
                              :aria-pressed="row.action === 'skip'"
                              @click="setRowPolicy(row.macro.macroId, 'keep')"
                            >
                              {{ tr("保留本机") }}
                            </button>
                          </span>
                        </span>
                      </li>
                    </ul>
                  </template>
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

                <template v-if="direction === 'import'">
                  <template v-if="importFiles.length">
                    <p class="pane-label is-spaced">{{ tr("导入方式") }}</p>
                    <div v-if="format === 'single'" class="option-seg" role="radiogroup">
                      <button
                        type="button"
                        role="radio"
                        :aria-pressed="importMode === 'merge'"
                        @click="importMode = 'merge'"
                      >
                        {{ tr("合并进本机") }}
                      </button>
                      <button
                        type="button"
                        role="radio"
                        :aria-pressed="importMode === 'replace'"
                        @click="importMode = 'replace'"
                      >
                        {{ tr("替换整库") }}
                      </button>
                    </div>
                    <p v-if="format === 'single'" class="option-note">
                      {{ importMode === "merge" ? tr("保留本机独有的宏") : tr("库变成文件内容") }}
                    </p>
                    <p v-else class="option-note">
                      {{ tr("多文件导入按宏合并，不会删除本机宏。") }}
                    </p>

                    <p
                      v-if="effectiveMode === 'replace' && droppedMacros.length"
                      class="option-warning"
                    >
                      <TriangleAlert :size="13" aria-hidden="true" />
                      <span>
                        {{ tr("会删除 {count} 个本机独有的宏。", { count: droppedMacros.length }) }}
                      </span>
                    </p>

                    <p class="pane-label is-spaced">{{ tr("遇到版本不同的同名宏") }}</p>
                    <div class="option-seg" role="radiogroup">
                      <button
                        type="button"
                        role="radio"
                        :aria-pressed="importPolicy === 'overwrite'"
                        @click="importPolicy = 'overwrite'"
                      >
                        {{ tr("用文件里的版本") }}
                      </button>
                      <button
                        type="button"
                        role="radio"
                        :aria-pressed="importPolicy === 'keep'"
                        @click="importPolicy = 'keep'"
                      >
                        {{ tr("保留本机版本") }}
                      </button>
                    </div>
                    <p class="option-note">
                      {{
                        importPolicy === "overwrite"
                          ? tr("同 ID 覆盖本机")
                          : tr("同 ID 时跳过文件里的")
                      }}
                    </p>
                  </template>
                  <p v-else class="option-note">
                    {{ tr("先选文件，读到内容后再决定怎么合并。") }}
                  </p>
                </template>

                <p v-else class="option-note">
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
            <template v-else-if="importFiles.length">
              {{ tr("将导入 {count} 个宏", { count: pendingCount }) }}
              <template v-if="importSummary"> · {{ importSummary }}</template>
            </template>
            <template v-else>{{ tr("还没有选择文件") }}</template>
          </p>
          <div class="foot-actions">
            <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
            <Button
              v-if="direction === 'import' && !importFiles.length"
              size="sm"
              variant="primary"
              @click="chooseImportFiles"
            >
              {{ tr("选择文件…") }}
            </Button>
            <Button v-else size="sm" variant="primary" :disabled="!canSubmit" @click="submit">
              {{
                direction === "export"
                  ? tr("开始导出")
                  : tr("导入 {count} 个宏", { count: pendingCount })
              }}
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
  min-height: 452px;
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
  overflow-y: auto;
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

.option-warning {
  display: flex;
  gap: var(--space-2);
  margin: var(--space-2) 0 0;
  border: 1px solid color-mix(in srgb, var(--warning) 30%, transparent);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--warning) 8%, transparent);
  padding: 7px 9px;
  color: var(--warning-text);
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.option-warning svg {
  flex: 0 0 auto;
  margin-top: 1px;
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

.pane-label.is-spaced {
  margin-top: var(--space-3);
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

.drop-zone {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: var(--space-2) var(--space-4) var(--space-3);
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-subtle);
  padding: var(--space-5);
  color: var(--text-muted);
  font: inherit;
  text-align: center;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.drop-zone:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.drop-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  color: var(--text-secondary);
}

.drop-zone strong {
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 650;
}

.drop-hint {
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.file-card {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr);
  gap: 10px;
  align-items: center;
  margin: 0 var(--space-4);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: 7px 9px;
}

.file-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--accent);
}

.file-copy {
  min-width: 0;
}

.file-name {
  display: block;
  overflow: hidden;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-meta {
  display: block;
  margin-top: 1px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
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
  background: var(--bg-raised);
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

.option-seg {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 3px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 3px;
}

.option-seg button {
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  padding: 6px 8px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-2xs);
  font-weight: 600;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.option-seg button:hover {
  color: var(--text-secondary);
}

.option-seg button[aria-pressed="true"] {
  background: var(--bg-elevated);
  box-shadow:
    0 1px 2px color-mix(in srgb, var(--shadow) 22%, transparent),
    0 0 0 1px var(--border);
  color: var(--text);
}

.macro-rows {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-1);
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
  border: 1px solid var(--border);
}

.macro-row:hover {
  background: var(--bg-hover);
}

.macro-row.selected {
  background: color-mix(in srgb, var(--accent) 9%, transparent);
  border-color: var(--accent-soft-strong);
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

.row-side {
  display: flex;
  align-items: center;
  gap: 6px;
}

.row-note {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  white-space: nowrap;
}

.status-chip {
  border: 1px solid transparent;
  border-radius: var(--radius-xs);
  padding: 1px 6px;
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
}

.status-new {
  border-color: color-mix(in srgb, var(--accent) 32%, transparent);
  background: var(--accent-soft);
  color: var(--accent);
}

.status-update {
  border-color: color-mix(in srgb, var(--info) 30%, transparent);
  background: color-mix(in srgb, var(--info) 12%, transparent);
  color: var(--info-text);
}

.status-same {
  border-color: var(--border);
  background: var(--bg-hover);
  color: var(--text-muted);
}

.mini-segment {
  display: inline-flex;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-raised);
}

.mini-segment button {
  border: 0;
  border-right: 1px solid var(--border);
  background: transparent;
  padding: 3px 7px;
  color: var(--text-muted);
  font: inherit;
  font-size: 10.5px;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.mini-segment button:last-child {
  border-right: 0;
}

.mini-segment button:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.mini-segment button[aria-pressed="true"] {
  background: var(--bg-elevated);
  color: var(--text);
  font-weight: 600;
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
  margin: var(--space-2) 0 0;
  color: var(--text);
  font-size: var(--text-lg);
  font-weight: 650;
}

.result-copy {
  margin: 0;
  max-width: 460px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.6;
}

.result-path {
  margin: 8px 0 0;
  max-width: 100%;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  padding: 5px 10px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-dropped {
  display: flex;
  flex-direction: column;
  gap: 5px;
  margin-top: 10px;
  max-width: 460px;
}

.result-dropped-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.dropped-list {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
  justify-content: center;
  margin: 0;
  padding: 0;
  max-height: 96px;
  overflow-y: auto;
  list-style: none;
}

.dropped-list li {
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  padding: 1px 7px;
  color: var(--text-secondary);
  font-size: var(--text-2xs);
}

.result-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: var(--space-4);
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
  max-width: 260px;
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
