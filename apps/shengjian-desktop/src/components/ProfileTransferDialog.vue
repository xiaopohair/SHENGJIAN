<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { ArrowLeftRight, Download, FileJson, FolderOpen, Upload, X } from "lucide-vue-next";
import { tr } from "../lib/i18n";
import {
  errorMessage,
  exportProfileAsPreset,
  exportProfiles,
  previewImportProfiles,
  saveButtonProfiles,
} from "../lib/api";
import {
  buildImportRows,
  composeImportDocument,
  dateStamp,
  estimateDocumentBytes,
  formatKilobytes,
  importCounts,
  importedTotal,
  presetIdsFor,
  resolveImportRows,
  transferSummary,
  type ConflictPolicy,
  type ImportAction,
  type TransferFormat,
  type TransferResultState,
} from "../lib/profileTransfer";
import type { ButtonProfilesDocument } from "../lib/types";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";
import TransferExportPane from "./transfer/TransferExportPane.vue";
import TransferImportPane from "./transfer/TransferImportPane.vue";
import TransferResult from "./transfer/TransferResult.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    document: ButtonProfilesDocument | null;
    /** 当前遥控器型号；推荐方案 id 与「只导入兼容方案」都用它。 */
    model?: string | null;
    busy?: boolean;
  }>(),
  { model: null, busy: false },
);

const emit = defineEmits<{ close: []; applied: [] }>();

type Stage = "form" | "busy" | "done" | "error";
type Direction = "export" | "import";

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const direction = ref<Direction>("export");
const stage = ref<Stage>("form");
const busyLabel = ref("");
/** 只有真正能按条计数的工序才给进度；单次调用一律用不定态，不编百分比。 */
const busyProgress = ref<{ done: number; total: number } | null>(null);
const result = shallowRef<TransferResultState | null>(null);

const selectedIds = ref<string[]>([]);
const format = ref<TransferFormat>("document");
const fileName = ref("");

const importFile = shallowRef<{
  path: string;
  name: string;
  document: ButtonProfilesDocument;
} | null>(null);
const policy = ref<ConflictPolicy>("overwrite");
const overrides = ref<Record<string, ImportAction>>({});
const onlyCompatible = ref(true);
/** 导入前的文档快照：撤销时原样写回去。 */
const previousDocument = shallowRef<ButtonProfilesDocument | null>(null);

let previousFocus: HTMLElement | null = null;

const profiles = computed(() => props.document?.profiles ?? []);

const selectedProfiles = computed(() =>
  profiles.value.filter((profile) => selectedIds.value.includes(profile.id)),
);

const summary = computed(() => transferSummary(selectedProfiles.value));
const sizeLabel = computed(() =>
  formatKilobytes(estimateDocumentBytes({ profiles: selectedProfiles.value })),
);
const canExport = computed(() => selectedProfiles.value.length > 0 && !props.busy);
const busyPercent = computed(() => {
  const progress = busyProgress.value;
  if (!progress || progress.total === 0) return 0;
  return Math.round((progress.done / progress.total) * 100);
});

/** 进入忙碌态：默认不定态，需要计数的工序自己填 busyProgress。 */
function beginBusy(label: string) {
  busyLabel.value = label;
  busyProgress.value = null;
  stage.value = "busy";
}

const importRows = computed(() =>
  importFile.value
    ? buildImportRows(importFile.value.document.profiles, profiles.value, props.model)
    : [],
);

const resolvedRows = computed(() =>
  resolveImportRows(importRows.value, {
    policy: policy.value,
    overrides: overrides.value,
    onlyCompatible: onlyCompatible.value,
  }),
);

const counts = computed(() => importCounts(resolvedRows.value));
const pendingCount = computed(() => importedTotal(counts.value));

const importFileInfo = computed(() => {
  if (!importFile.value) return null;
  const models = new Set(
    importFile.value.document.profiles.map((profile) => profile.deviceModel ?? tr("通用")),
  );
  return {
    name: importFile.value.name,
    meta: tr("{count} 个方案 · {models}", {
      count: importFile.value.document.profiles.length,
      models: [...models].join(" / "),
    }),
  };
});

const importBreakdown = computed(() => {
  const value = counts.value;
  const parts: string[] = [];
  if (value.add) parts.push(tr("新增 {count}", { count: value.add }));
  if (value.overwrite) parts.push(tr("覆盖 {count}", { count: value.overwrite }));
  if (value.keep) parts.push(tr("保留 {count}", { count: value.keep }));
  if (value.skip) parts.push(tr("跳过 {count}", { count: value.skip }));
  return parts.join(" · ");
});

function resetState() {
  direction.value = "export";
  stage.value = "form";
  result.value = null;
  busyLabel.value = "";
  busyProgress.value = null;
  format.value = "document";
  selectedIds.value = profiles.value.map((profile) => profile.id);
  fileName.value = tr("按键方案-{date}", { date: dateStamp() });
  importFile.value = null;
  policy.value = "overwrite";
  overrides.value = {};
  onlyCompatible.value = true;
  previousDocument.value = null;
}

function setDirection(next: Direction) {
  if (stage.value !== "form") return;
  direction.value = next;
}

function toggleSelected(id: string) {
  const profile = profiles.value.find((item) => item.id === id);
  if (!profile) return;
  if (format.value === "preset" && profile.isDefault) return;
  selectedIds.value = selectedIds.value.includes(id)
    ? selectedIds.value.filter((value) => value !== id)
    : [...selectedIds.value, id];
}

function toggleAll() {
  const selectable = profiles.value.filter(
    (profile) => !(format.value === "preset" && profile.isDefault),
  );
  selectedIds.value =
    selectedIds.value.length === selectable.length ? [] : selectable.map((profile) => profile.id);
}

function changeFormat(next: TransferFormat) {
  format.value = next;
  // 推荐方案是下发给别人的，默认方案属于本机型基线，不能带走。
  if (next === "preset") {
    const defaults = new Set(
      profiles.value.filter((profile) => profile.isDefault).map((profile) => profile.id),
    );
    selectedIds.value = selectedIds.value.filter((id) => !defaults.has(id));
  }
}

function failWith(cause: unknown) {
  stage.value = "error";
  result.value = { kind: "error", message: errorMessage(cause) };
}

async function runExport() {
  if (!props.document || !canExport.value) return;
  if (format.value === "preset") {
    await runPresetExport();
    return;
  }

  const picked = selectedProfiles.value;
  try {
    const target = await saveDialog({
      defaultPath: `${fileName.value.trim() || tr("按键方案")}.json`,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!target) return;

    beginBusy(tr("正在导出…"));
    const snapshot = { ...props.document };
    await exportProfiles(target, {
      ...snapshot,
      profiles: picked,
      activeManualProfileId: picked.some((profile) => profile.id === snapshot.activeManualProfileId)
        ? snapshot.activeManualProfileId
        : null,
    });
    result.value = {
      kind: "export",
      count: picked.length,
      binds: summary.value.binds,
      macros: summary.value.macros,
      size: sizeLabel.value,
      path: target,
    };
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

async function runPresetExport() {
  const picked = selectedProfiles.value;
  if (picked.length === 0) return;
  try {
    const directory = await openDialog({ directory: true, multiple: false });
    if (!directory || Array.isArray(directory)) return;

    beginBusy(tr("正在写入推荐方案…"));
    // 这里是真正按文件推进的工序，所以进度条报的是实际写出的条数。
    busyProgress.value = { done: 0, total: picked.length };
    const presetIds = presetIdsFor(picked, props.model);
    for (const profile of picked) {
      const presetId = presetIds.get(profile.id);
      if (!presetId) continue;
      await exportProfileAsPreset({
        path: `${directory}\\${presetId}.json`,
        profile,
        presetId,
        compatibleModels: profile.deviceModel ? [profile.deviceModel] : [],
      });
      busyProgress.value = {
        done: (busyProgress.value?.done ?? 0) + 1,
        total: picked.length,
      };
    }
    result.value = {
      kind: "export",
      count: picked.length,
      binds: summary.value.binds,
      macros: summary.value.macros,
      size: tr("{count} 个文件", { count: picked.length }),
      path: directory,
    };
    stage.value = "done";
  } catch (cause) {
    failWith(cause);
  }
}

async function chooseImportFile() {
  try {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;

    beginBusy(tr("正在读取文件…"));
    const document = await previewImportProfiles(path);
    importFile.value = {
      path,
      name: path.split(/[\\/]/).pop() ?? path,
      document,
    };
    overrides.value = {};
    stage.value = "form";
    direction.value = "import";
  } catch (cause) {
    failWith(cause);
  }
}

async function applyImport() {
  if (!props.document || !importFile.value || pendingCount.value === 0) return;
  const before = props.document;
  const next = composeImportDocument(before, resolvedRows.value);
  const snapshotCounts = { ...counts.value };

  try {
    beginBusy(tr("正在导入…"));
    await saveButtonProfiles(next);
    previousDocument.value = before;
    result.value = { kind: "import", counts: snapshotCounts };
    stage.value = "done";
    emit("applied");
  } catch (cause) {
    failWith(cause);
  }
}

async function undoImport() {
  if (!previousDocument.value) return;
  const restore = previousDocument.value;
  try {
    beginBusy(tr("正在撤销…"));
    await saveButtonProfiles(restore);
    previousDocument.value = null;
    result.value = null;
    stage.value = "form";
    emit("applied");
  } catch (cause) {
    failWith(cause);
  }
}

function close() {
  emit("close");
}

function runPrimary() {
  if (stage.value !== "form" || props.busy) return;
  if (direction.value === "export") {
    void runExport();
    return;
  }
  if (importFile.value) void applyImport();
  else void chooseImportFile();
}

function onWindowKeydown(event: KeyboardEvent) {
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
  runPrimary();
}

watch(
  () => props.open,
  async (isOpen) => {
    if (!isOpen) {
      window.removeEventListener("keydown", onWindowKeydown, true);
      previousFocus?.focus();
      previousFocus = null;
      return;
    }
    previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    resetState();
    window.addEventListener("keydown", onWindowKeydown, true);
    await nextTick();
    dialogRef.value
      ?.querySelector<HTMLElement>(".transfer-seg button[aria-pressed='true']")
      ?.focus();
  },
  { immediate: true },
);

onUnmounted(() => {
  window.removeEventListener("keydown", onWindowKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="transfer-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="transfer-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="transfer-title"
      >
        <header class="transfer-head">
          <span class="head-badge" aria-hidden="true"><ArrowLeftRight :size="17" /></span>
          <div class="head-copy">
            <h2 id="transfer-title">{{ tr("导入与导出") }}</h2>
            <p>
              {{
                direction === "import"
                  ? tr("从别的电脑带过来：先看清内容、冲突和依赖，再决定放哪儿。")
                  : tr("在电脑之间搬运按键方案，或者把方案整理成推荐方案。")
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
              direction === "export" ? tr("导出只读，不改动本机配置") : tr("导入前先显示冲突与依赖")
            }}
          </span>
        </div>

        <div class="transfer-body">
          <template v-if="stage === 'done' || stage === 'error'">
            <TransferResult
              v-if="result"
              :state="result"
              :can-undo="Boolean(previousDocument)"
              @finish="close"
              @undo="undoImport"
              @retry="chooseImportFile"
            />
          </template>

          <template v-else-if="direction === 'export'">
            <TransferExportPane
              :profiles="profiles"
              :selected-ids="selectedIds"
              :format="format"
              @toggle="toggleSelected"
              @toggle-all="toggleAll"
              @change-format="changeFormat"
            />
          </template>

          <template v-else>
            <TransferImportPane
              :file="importFileInfo"
              :rows="resolvedRows"
              :policy="policy"
              :only-compatible="onlyCompatible"
              :model="model"
              @pick-file="chooseImportFile"
              @clear-file="importFile = null"
              @change-policy="policy = $event"
              @override="(id, action) => (overrides = { ...overrides, [id]: action })"
              @toggle-compatible="onlyCompatible = !onlyCompatible"
            />
          </template>

          <div v-if="stage === 'busy'" class="transfer-busy">
            <span v-if="!busyProgress" class="busy-spinner" aria-hidden="true" />
            <span class="busy-label">{{ busyLabel }}</span>
            <span
              class="busy-bar"
              role="progressbar"
              aria-valuemin="0"
              aria-valuemax="100"
              :aria-valuenow="busyProgress ? busyPercent : undefined"
            >
              <i
                :class="{ indeterminate: !busyProgress }"
                :style="busyProgress ? { width: `${busyPercent}%` } : undefined"
              />
            </span>
            <span v-if="busyProgress" class="busy-count">
              {{ busyProgress.done }} / {{ busyProgress.total }}
            </span>
          </div>
        </div>

        <footer v-if="stage === 'form'" class="transfer-foot">
          <template v-if="direction === 'export'">
            <label v-if="format === 'document'" class="dest-field">
              <FileJson :size="13" aria-hidden="true" />
              <input
                v-model="fileName"
                class="dest-input"
                type="text"
                spellcheck="false"
                :aria-label="tr('文件名')"
              />
              <span class="dest-ext">.json</span>
            </label>
            <p v-else class="dest-field is-static">
              <FolderOpen :size="13" aria-hidden="true" />
              <span>{{ tr("导出时选择文件夹") }}</span>
            </p>

            <p class="foot-summary">
              <template v-if="summary.count">
                <b>{{ summary.count }}</b> {{ tr("个方案") }} · {{ summary.binds }}
                {{ tr("键位") }}
                <template v-if="summary.macros">
                  · {{ tr("含 {count} 个宏", { count: summary.macros }) }}
                </template>
                · {{ tr("约 {size}", { size: sizeLabel }) }}
              </template>
              <template v-else>{{ tr("未选择方案") }}</template>
            </p>

            <div class="foot-actions">
              <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
              <Button size="sm" variant="primary" :disabled="!canExport" @click="runExport">
                {{
                  format === "preset"
                    ? tr("选择文件夹并导出")
                    : tr("导出 {count} 个方案", { count: selectedProfiles.length })
                }}
              </Button>
            </div>
          </template>

          <template v-else>
            <p class="foot-summary">
              <template v-if="pendingCount">
                {{ tr("将导入 {count} 个方案", { count: pendingCount }) }} · {{ importBreakdown }}
              </template>
              <template v-else>{{ tr("没有可导入的方案") }}</template>
            </p>
            <div class="foot-actions">
              <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
              <Button
                v-if="!importFile"
                size="sm"
                variant="primary"
                :disabled="busy"
                @click="chooseImportFile"
              >
                {{ tr("选择文件…") }}
              </Button>
              <Button
                v-else
                size="sm"
                variant="primary"
                :disabled="pendingCount === 0 || busy"
                @click="applyImport"
              >
                {{ tr("导入 {count} 个方案", { count: pendingCount }) }}
              </Button>
            </div>
          </template>
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

.transfer-busy {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: var(--space-3);
  background: color-mix(in srgb, var(--bg-raised) 92%, transparent);
  backdrop-filter: blur(2px);
}

.busy-spinner {
  width: 22px;
  height: 22px;
  border: 2px solid var(--border-strong);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: transfer-spin 0.8s linear infinite;
}

@keyframes transfer-spin {
  to {
    transform: rotate(360deg);
  }
}

.busy-label {
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.busy-bar {
  position: relative;
  width: 280px;
  height: 3px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--bg-hover);
}

.busy-bar i {
  position: absolute;
  inset: 0 auto 0 0;
  width: 0;
  border-radius: 2px;
  background: var(--accent);
  transition: width 160ms var(--ease-out, ease);
}

.busy-bar i.indeterminate {
  width: 40%;
  transition: none;
  animation: transfer-indeterminate 1.1s ease-in-out infinite;
}

.busy-count {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

@keyframes transfer-indeterminate {
  0% {
    left: -40%;
  }
  100% {
    left: 100%;
  }
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
  flex: 0 0 320px;
  align-items: center;
  gap: 6px;
  max-width: 320px;
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

.dest-input {
  min-width: 60px;
  flex: 1 1 auto;
  border: 0;
  background: transparent;
  padding: 0;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 11.5px;
  outline: none;
}

.dest-ext {
  flex: 0 0 auto;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 11px;
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

@media (prefers-reduced-motion: reduce) {
  .busy-spinner,
  .busy-bar i {
    animation: none;
  }
}
</style>
