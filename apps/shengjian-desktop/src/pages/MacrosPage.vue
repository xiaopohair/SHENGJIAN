<script setup lang="ts">
import {
  computed,
  nextTick,
  onMounted,
  onUnmounted,
  ref,
  shallowRef,
  useTemplateRef,
  watch,
} from "vue";
import {
  ArrowLeftRight,
  Command,
  Copy,
  MoreVertical,
  Play,
  Plus,
  SlidersHorizontal,
  Square,
  Trash2,
} from "lucide-vue-next";
import { errorMessage, saveMacroLibrary } from "../lib/api";
import { createMacroStep, type MacroDraft } from "../lib/macros";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { confirmAction } from "../lib/confirm";
import { useDraftGuard } from "../composables/useDraftGuard";
import { useMacroLibrary, useMacroRunStates } from "../composables/useMacroLibrary";
import { useMacroDraftTestRun } from "../composables/useMacroDraftTestRun";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import IconButton from "../components/ui/IconButton.vue";
import Card from "../components/ui/Card.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import Notice from "../components/ui/Notice.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import MacroEditor from "../components/MacroEditor.vue";
import MacroTransferDialog from "../components/MacroTransferDialog.vue";
import MacroSettingsDrawer from "../components/macro-editor/MacroSettingsDrawer.vue";
import type { MacroChord, MacroDefinition, MacroLibraryDocument, MacroStep } from "../lib/types";

const editor = ref<MacroDraft | null>(null);
const editingNew = shallowRef(false);
const busy = shallowRef(false);
const error = shallowRef<string | null>(null);
const saveError = shallowRef<string | null>(null);
const editorHost = useTemplateRef<HTMLElement>("editorHost");
const macroTabs = useTemplateRef<HTMLElement>("macroTabs");
const baselineJson = shallowRef("");
const transferOpen = shallowRef(false);
const moreMenuOpen = shallowRef(false);
const moreMenuRef = useTemplateRef<HTMLElement>("moreMenuRef");

function toggleMoreMenu() {
  moreMenuOpen.value = !moreMenuOpen.value;
}

/** 点菜单外或按 Esc 收起菜单。 */
function onDocumentClick(event: MouseEvent) {
  if (!moreMenuOpen.value) return;
  const target = event.target as Node | null;
  if (target && moreMenuRef.value?.contains(target)) return;
  moreMenuOpen.value = false;
}

onMounted(() => document.addEventListener("click", onDocumentClick));
onUnmounted(() => document.removeEventListener("click", onDocumentClick));

/** 菜单里的动作：先收起菜单再执行，避免菜单挡住后续界面。 */
function runFromMoreMenu(action: () => void | Promise<void>) {
  moreMenuOpen.value = false;
  void action();
}

function openTransferFromMenu() {
  moreMenuOpen.value = false;
  void openTransfer();
}

const reportError = (cause: unknown) => {
  const message = macroErrorMessage(cause);
  // Errors raised from the editor toolbar stay next to the buttons that caused
  // them, so the reason is visible without scrolling to the page top.
  if (editor.value) {
    saveError.value = message;
    return;
  }
  error.value = message;
};

const {
  macroDocument,
  loading,
  latestMacros,
  nextVersion,
  load: loadLibrary,
} = useMacroLibrary(reportError);
const { runStates, refreshRunStates } = useMacroRunStates(latestMacros);
const {
  draftRunStatus,
  draftTesting,
  testDraft: startDraftTest,
  stopDraft: stopDraftRun,
  resetDraftRun,
  closeDraftRun,
} = useMacroDraftTestRun(editor, reportError);

const { dirty, confirmDiscard } = useDraftGuard({
  draft: editor,
  baseline: baselineJson,
  discardMessage: "当前宏的修改尚未保存，继续将丢失这些修改。",
});

const selectedDefinition = computed(() => {
  const macroId = editor.value?.macroId;
  if (!macroId) return null;
  return latestMacros.value.find((definition) => definition.macroId === macroId) ?? null;
});

const otherMacros = computed(() =>
  latestMacros.value.filter((definition) => definition.macroId !== editor.value?.macroId),
);

const macroTabRows = computed(() =>
  latestMacros.value.map((definition) => ({
    definition,
    status: statusText(definition.macroId),
    tone: runTone(definition.macroId),
  })),
);

function statusText(macroId: string): string {
  const status = runStates.value[macroId];
  if (!status) return "";
  if (status.running) return tr("运行中");
  if (!status.report) return "";
  return `${statusLabel(status.report.result)} · ${status.report.elapsedMs}ms`;
}

function runTone(
  macroId: string,
): "neutral" | "success" | "warning" | "danger" | "info" | "accent" {
  const status = runStates.value[macroId];
  if (!status) return "neutral";
  if (status.running) return "accent";
  const result = status.report?.result;
  if (result === "completed" || result === "done") return "success";
  if (result === "cancelled") return "neutral";
  if (!result) return "neutral";
  return "danger";
}

function syncBaseline() {
  baselineJson.value = editor.value ? JSON.stringify(editor.value) : "";
}

/** 工具栏的「试运行」是否可用：草稿必须通过校验（与保存同一套规则）。 */
const canTestDraft = computed(() => Boolean(editor.value) && !validateDraft());

function closeEditor() {
  closeDraftRun();
  editor.value = null;
  editingNew.value = false;
  baselineJson.value = "";
  saveError.value = null;
}

/**
 * 新建宏：**直接落盘成草稿**，宏库与标签栏里马上就能看到这一条。
 *
 * 首步用「切到前台应用」—— 它是唯一不需要必填参数的动作，所以刚建出来的草稿
 * 能通过后端的宏校验；之后用户改了什么却没保存，离开时再提示。
 */
async function openNew() {
  if (!(await confirmDiscard())) return;
  if (!macroDocument.value) return;
  resetDraftRun();
  saveError.value = null;
  const draft: MacroDraft = {
    macroId: `macro-${crypto.randomUUID()}`,
    name: tr("新宏"),
    summary: "",
    version: "1.0.0",
    steps: [createMacroStep("focus_frontmost")],
  };
  busy.value = true;
  try {
    const definition: MacroDefinition = {
      schemaVersion: "0.1",
      macroId: draft.macroId,
      version: draft.version,
      name: draft.name,
      summary: null,
      steps: draft.steps,
    };
    macroDocument.value = await saveMacroLibrary({
      ...macroDocument.value,
      definitions: [...macroDocument.value.definitions, definition],
    });
  } catch (cause) {
    reportError(cause);
    return;
  } finally {
    busy.value = false;
  }
  editingNew.value = true;
  editor.value = draft;
  syncBaseline();
}

async function openEdit(definition: MacroDefinition) {
  if (!(await confirmDiscard())) return;
  resetDraftRun();
  saveError.value = null;
  editingNew.value = false;
  editor.value = {
    macroId: definition.macroId,
    name: definition.name,
    summary: definition.summary ?? "",
    version: nextVersion(definition.macroId),
    steps: JSON.parse(JSON.stringify(definition.steps)) as MacroStep[],
  };
  syncBaseline();
}

async function duplicateMacro(definition: MacroDefinition) {
  if (!(await confirmDiscard())) return;
  resetDraftRun();
  editingNew.value = true;
  editor.value = {
    macroId: `macro-${crypto.randomUUID()}`,
    name: tr("{name} 副本", { name: definition.name }).slice(0, 20),
    summary: definition.summary ?? "",
    version: "1.0.0",
    steps: JSON.parse(JSON.stringify(definition.steps)) as MacroStep[],
  };
  syncBaseline();
}

function duplicateCurrent() {
  if (selectedDefinition.value) void duplicateMacro(selectedDefinition.value);
}

function deleteCurrent() {
  if (selectedDefinition.value) void removeMacro(selectedDefinition.value);
}

async function testDraft() {
  if (!editor.value || draftTesting.value) return;
  const validation = validateDraft();
  if (validation) {
    saveError.value = validation;
    return;
  }
  const definition: MacroDefinition = {
    schemaVersion: "0.1",
    macroId: `macro-draft-${crypto.randomUUID()}`,
    version: editor.value.version,
    name: editor.value.name.trim(),
    summary: editor.value.summary.trim() || null,
    steps: editor.value.steps,
  };
  saveError.value = null;
  error.value = null;
  await startDraftTest(definition);
}

async function stopDraft() {
  await stopDraftRun();
}

/**
 * A finished draft run that did not succeed explains itself next to the
 * buttons: which step failed and why, so the reason is visible without hunting
 * through the trace.
 */
watch(
  () => draftRunStatus.value,
  (status) => {
    if (!status || status.running) return;
    const report = status.report;
    if (!report) return;
    if (report.result === "completed" || report.result === "completed_with_unknown_result") {
      return;
    }
    const failed = report.steps.find(
      (step) => step.outcome !== "done" && step.outcome !== "unknown",
    );
    const reason = statusLabel(report.result);
    saveError.value = failed
      ? tr("第 {index} 步未完成：{reason}", {
          index: failed.index + 1,
          reason: statusLabel(failed.outcome),
        })
      : tr("试运行未成功：{reason}", { reason });
  },
);

function onMacroWheel(event: WheelEvent) {
  const element = event.currentTarget as HTMLElement;
  if (element.scrollWidth <= element.clientWidth) return;
  if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
  element.scrollLeft += event.deltaY;
  event.preventDefault();
}

watch(
  () => editor.value?.macroId,
  async () => {
    await nextTick();
    macroTabs.value
      ?.querySelector<HTMLElement>(".macro-tab.selected")
      ?.scrollIntoView({ block: "nearest", inline: "nearest", behavior: "smooth" });
  },
);

async function editLatest(definition: MacroDefinition) {
  if (editor.value?.macroId === definition.macroId && !editingNew.value) return;
  await openEdit(definition);
}

function chord(step: MacroStep): MacroChord {
  return step.parameters.chord ?? { modifiers: [], key: "" };
}

const MACRO_ISSUE_LABELS: Record<string, string> = {
  unsupported_schema_version: "不支持的宏 schema 版本",
  invalid_macro_id: "宏 ID 无效",
  invalid_version: "版本号无效",
  name_empty: "名称不能为空",
  name_too_long: "名称不能超过 20 个字符",
  summary_too_long: "说明不能超过 300 个字符",
  too_few_steps: "至少需要一个步骤",
  too_many_steps: "步骤不能超过 20 个",
  too_many_definitions: "宏数量超过上限",
  invalid_step_id: "步骤 ID 无效",
  duplicate_step_id: "步骤 ID 重复",
  step_timeout_out_of_range: "步骤超时超出范围（100–60000ms）",
  invalid_url: "网址必须是 http/https",
  invalid_chord: "快捷键无效",
  invalid_wait_state: "等待状态无效",
  nested_self: "宏不能调用自身",
  nested_cycle: "宏调用存在循环",
  duplicate_definition: "存在重复的宏版本",
  immutable_version_conflict: "宏版本冲突",
};

function macroIssueLabel(code: string): string {
  if (code in MACRO_ISSUE_LABELS) return trDynamic(MACRO_ISSUE_LABELS[code]);
  if (code.startsWith("missing_parameter.")) {
    return tr("缺少参数：{name}", { name: code.split(".")[1] ?? code });
  }
  if (code.startsWith("unexpected_parameter.")) {
    return tr("多余参数：{name}", { name: code.split(".")[1] ?? code });
  }
  return code;
}

function macroErrorMessage(cause: unknown): string {
  const raw = errorMessage(cause);
  const prefix = "macro validation failed: ";
  if (raw.startsWith(prefix)) {
    const labels = raw
      .slice(prefix.length)
      .split(",")
      .map((code) => macroIssueLabel(code.trim()));
    return tr("宏校验失败：{labels}", { labels: labels.join(tr("；")) });
  }
  return MACRO_ISSUE_LABELS[raw] ?? raw;
}

function validateDraft(): string | null {
  if (!editor.value) return tr("没有正在编辑的宏");
  const name = editor.value.name.trim();
  if (!name) return tr("宏名称不能为空");
  if (name.length > 20) return tr("宏名称不能超过 20 个字符");
  if (editor.value.summary.length > 300) return tr("宏说明不能超过 300 个字符");
  if (editor.value.steps.length === 0) return tr("至少需要一个步骤");
  if (editor.value.steps.length > 20) return tr("步骤不能超过 20 个");
  for (const [index, step] of editor.value.steps.entries()) {
    const position = tr("步骤 #{index}", { index: index + 1 });
    const maxTimeout = step.action === "run_macro" ? 60000 : 10000;
    if (
      step.timeoutMs !== null &&
      step.timeoutMs !== undefined &&
      (step.timeoutMs < 100 || step.timeoutMs > maxTimeout)
    ) {
      return tr("{position}：超时超出范围（100–{max}ms）", { position, max: maxTimeout });
    }
    switch (step.action) {
      case "send_shortcut":
      case "system_action":
        if (!chord(step).key.trim()) return tr("{position}：缺少快捷键", { position });
        break;
      case "open_application":
      case "focus_input":
        if (!(step.parameters.target ?? "").trim())
          return tr("{position}：缺少应用目标", { position });
        break;
      case "wait_for_application":
        if (!(step.parameters.target ?? "").trim())
          return tr("{position}：缺少应用目标", { position });
        break;
      case "open_url":
        if (!/^https?:\/\/\S+/i.test(step.parameters.url ?? "")) {
          return tr("{position}：网址必须是 http/https", { position });
        }
        break;
      case "run_macro":
        if (!(step.parameters.nestedMacroId ?? "").trim()) {
          return tr("{position}：未选择目标宏", { position });
        }
        break;
      default:
        break;
    }
  }
  return null;
}

async function saveEditor() {
  if (!editor.value || !macroDocument.value) return;
  const validation = validateDraft();
  if (validation) {
    saveError.value = validation;
    return;
  }
  busy.value = true;
  saveError.value = null;
  try {
    const definition: MacroDefinition = {
      schemaVersion: "0.1",
      macroId: editor.value.macroId,
      version: editingNew.value ? "1.0.0" : nextVersion(editor.value.macroId),
      name: editor.value.name.trim(),
      summary: editor.value.summary.trim() || null,
      steps: editor.value.steps,
    };
    // Saving replaces every stored version of this macro: the library keeps one
    // current definition per macro, and macros.json.bak covers recovery.
    const definitions = [
      ...macroDocument.value.definitions.filter((item) => item.macroId !== definition.macroId),
      definition,
    ];
    macroDocument.value = await saveMacroLibrary({
      ...macroDocument.value,
      definitions,
    });
    // Keep the editor open on the saved macro so further edits continue in place.
    editor.value = {
      macroId: definition.macroId,
      name: definition.name,
      summary: definition.summary ?? "",
      version: definition.version,
      steps: definition.steps,
    };
    editingNew.value = false;
    syncBaseline();
    await refreshRunStates();
  } catch (cause) {
    saveError.value = macroErrorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function removeMacro(definition: MacroDefinition) {
  if (!macroDocument.value) return;
  const isLast = latestMacros.value.length <= 1;
  const confirmed = await confirmAction({
    title: tr("删除宏“{name}”？", { name: definition.name }),
    message: isLast
      ? tr("这是最后一个宏，删除后宏库将为空。此操作不可恢复。")
      : tr("此操作不可恢复。"),
    confirmLabel: tr("删除"),
    danger: true,
  });
  if (!confirmed) return;
  busy.value = true;
  error.value = null;
  try {
    const definitions = macroDocument.value.definitions.filter(
      (item) => item.macroId !== definition.macroId,
    );
    macroDocument.value = await saveMacroLibrary({
      ...macroDocument.value,
      definitions,
    });
    if (editor.value?.macroId === definition.macroId) closeEditor();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    busy.value = false;
  }
}

/**
 * 打开导入/导出面板（单文件或多文件都在同一个面板里选）。
 *
 * 面板里读写文件、算进度；落盘与刷新仍由本页负责，避免两处各写一遍。
 */
async function openTransfer() {
  if (!macroDocument.value) return;
  if (!(await confirmDiscard())) return;
  error.value = null;
  transferOpen.value = true;
}

/** 导入完成：面板已经把新库保存到磁盘，这里只同步内存与运行状态。 */
async function applyTransfer(document: MacroLibraryDocument) {
  macroDocument.value = document;
  closeEditor();
  await refreshRunStates();
}

watch(editor, async (value) => {
  if (!value) return;
  await nextTick();
  if (window.matchMedia("(max-width: 1120px)").matches) {
    const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    editorHost.value?.scrollIntoView({
      behavior: reduceMotion ? "auto" : "smooth",
      block: "start",
    });
  }
});

const macroSettingsOpen = shallowRef(false);

/** Two-way bridges so the drawer edits the draft in place. */
const editorNameModel = computed({
  get: () => editor.value?.name ?? "",
  set: (value: string) => {
    if (editor.value) editor.value.name = value;
  },
});

const editorSummaryModel = computed({
  get: () => editor.value?.summary ?? "",
  set: (value: string) => {
    if (editor.value) editor.value.summary = value;
  },
});

onMounted(async () => {
  await loadLibrary();
  await refreshRunStates();
  // Land on the first macro so the library is not an unexplained row of tabs.
  if (!editor.value && latestMacros.value.length > 0) {
    await openEdit(latestMacros.value[0]);
  }
});
</script>

<template>
  <section class="page page-macros">
    <PageHeader
      eyebrow="Macros"
      title="宏"
      description="创建可复用的动作序列，绑定到按键或手动测试。"
    />

    <Notice v-if="error" kind="error">{{ error }}</Notice>

    <div class="macro-toolbar">
      <div class="macro-tabs-wrap">
        <div
          v-if="latestMacros.length || editingNew"
          ref="macroTabs"
          class="macro-tabs"
          @wheel="onMacroWheel"
        >
          <button v-if="editingNew" type="button" class="macro-tab selected new-tab">
            <span class="macro-tab-index new">+</span>
            <span class="macro-tab-name">{{ editor?.name || tr("新宏") }}</span>
          </button>
          <button
            v-for="(row, index) in macroTabRows"
            :key="row.definition.macroId"
            type="button"
            class="macro-tab"
            :class="{ selected: editor?.macroId === row.definition.macroId }"
            :title="`${row.definition.name} · ${tr('{count} 个步骤', { count: row.definition.steps.length })}`"
            @click="editLatest(row.definition)"
          >
            <span class="macro-tab-index">{{ index + 1 }}</span>
            <span class="macro-tab-name" :title="row.definition.name">
              {{ row.definition.name }}
            </span>
            <span v-if="row.status" class="macro-tab-status" :class="row.tone">
              {{ row.status }}
            </span>
          </button>
        </div>
      </div>
      <div class="macro-toolbar-actions">
        <StatusPill v-if="editor && dirty" :label="trDynamic('未保存')" tone="warning" dot />
        <template v-if="editor">
          <Button
            v-if="draftRunStatus?.running"
            size="sm"
            variant="danger"
            :disabled="busy"
            @click="stopDraft"
          >
            <Square :size="13" />
            {{ tr("停止") }}
          </Button>
          <Button
            v-else
            size="sm"
            variant="secondary"
            :disabled="busy || !canTestDraft"
            @click="testDraft"
          >
            <Play :size="14" />
            {{ draftTesting ? tr("启动中…") : tr("试运行") }}
          </Button>
          <Button size="sm" variant="primary" :loading="busy" @click="saveEditor">
            {{ tr("保存") }}
          </Button>
        </template>
        <Button size="sm" :disabled="busy || !macroDocument" @click="openNew">
          <Plus :size="14" />
          {{ tr("添加") }}
        </Button>
        <template v-if="editor">
          <IconButton
            size="sm"
            :label="tr('属性')"
            :disabled="busy"
            @click="macroSettingsOpen = true"
          >
            <SlidersHorizontal :size="14" />
          </IconButton>
        </template>
        <div ref="moreMenuRef" class="macro-more">
          <IconButton
            size="sm"
            :label="tr('更多操作')"
            :aria-expanded="moreMenuOpen"
            aria-haspopup="menu"
            @click="toggleMoreMenu"
          >
            <MoreVertical :size="15" />
          </IconButton>
          <div v-if="moreMenuOpen" class="macro-more-menu" role="menu">
            <button
              type="button"
              class="macro-more-item"
              role="menuitem"
              :disabled="busy || !macroDocument"
              @click="openTransferFromMenu"
            >
              <ArrowLeftRight :size="14" />
              {{ tr("导入 / 导出") }}
            </button>
            <template v-if="editor && !editingNew">
              <div class="macro-more-divider" role="separator" />
              <button
                type="button"
                class="macro-more-item"
                role="menuitem"
                :disabled="busy"
                @click="runFromMoreMenu(duplicateCurrent)"
              >
                <Copy :size="14" />
                {{ tr("复制宏") }}
              </button>
              <button
                type="button"
                class="macro-more-item is-danger"
                role="menuitem"
                :disabled="busy"
                @click="runFromMoreMenu(deleteCurrent)"
              >
                <Trash2 :size="14" />
                {{ tr("删除宏") }}
              </button>
            </template>
          </div>
        </div>
      </div>
    </div>

    <LoadingState v-if="loading" label="正在加载…" description="正在读取本地宏库…" />

    <EmptyState
      v-else-if="macroDocument && latestMacros.length === 0 && !editor"
      title="还没有宏"
      description="新建一个宏，或从导入宏库开始。"
    >
      <template #icon><Command :size="20" /></template>
      <template #actions>
        <Button variant="primary" @click="openNew">{{ tr("新建宏") }}</Button>
      </template>
    </EmptyState>

    <div
      v-if="editor || (!loading && latestMacros.length > 0)"
      ref="editorHost"
      class="editor-host"
    >
      <div v-if="editor" class="macro-editor-shell">
        <p v-if="saveError" class="macro-save-error" role="alert">{{ saveError }}</p>
        <MacroEditor
          v-model:draft="editor"
          :other-macros="otherMacros"
          :run-status="draftRunStatus"
        />
      </div>

      <Card v-else class="editor-placeholder">
        <EmptyState title="选择一个宏开始编辑" description="从顶部宏库选择，或新建一个宏。">
          <template #icon><Command :size="20" /></template>
          <template #actions>
            <Button variant="primary" @click="openNew">{{ tr("新建宏") }}</Button>
          </template>
        </EmptyState>
      </Card>
    </div>

    <MacroSettingsDrawer
      v-if="editor"
      v-model:name="editorNameModel"
      v-model:summary="editorSummaryModel"
      :open="macroSettingsOpen"
      @close="macroSettingsOpen = false"
    />
  </section>

  <MacroTransferDialog
    :open="transferOpen"
    :document="macroDocument"
    @applied="applyTransfer"
    @close="transferOpen = false"
  />
</template>

<style scoped>
.page-macros {
  gap: var(--space-3);
}

.macro-toolbar {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding-bottom: var(--space-2);
  min-width: 0;
}

.macro-tabs-wrap {
  flex: 1 1 auto;
  min-width: 0;
}

.macro-tabs {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 3px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-subtle);
  scrollbar-width: none;
}
.macro-tabs::-webkit-scrollbar {
  display: none;
}

.macro-tab {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
  max-width: 260px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  padding: 7px 14px;
  color: var(--text-secondary);
  font-size: 12px;
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}
.macro-tab:hover {
  border-color: var(--border-strong);
  background: var(--bg-raised);
  color: var(--text);
}
.macro-tab.selected {
  border-color: var(--accent);
  background: var(--bg-raised);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
}
.macro-tab-index {
  display: grid;
  flex: 0 0 18px;
  place-items: center;
  width: 18px;
  height: 18px;
  border-radius: 5px;
  background: var(--bg-subtle);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 10px;
  line-height: 1;
}
.macro-tab.selected .macro-tab-index {
  background: var(--accent);
  color: var(--text-on-accent);
}
.macro-tab.new-tab .macro-tab-index {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}
.macro-tab-name {
  overflow: hidden;
  min-width: 0;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.macro-tab.selected .macro-tab-name {
  color: var(--accent-ink);
  font-weight: 600;
}
.macro-tab-status {
  max-width: 100px;
  overflow: hidden;
  flex: 0 0 auto;
  color: var(--text-muted);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.macro-tab-status.success {
  color: var(--success-text);
}
.macro-tab-status.danger {
  color: var(--danger-text);
}
.macro-tab-status.warning {
  color: var(--warning-text);
}
.macro-tab-status.accent,
.macro-tab-status.info {
  color: var(--info-text);
}

.macro-toolbar-actions {
  display: flex;
  flex: 0 0 auto;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}
.editor-host {
  min-width: 0;
  scroll-margin-top: var(--space-4);
}

.macro-editor-shell {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.macro-save-error {
  margin: 0;
  border: 1px solid color-mix(in srgb, var(--danger) 35%, transparent);
  border-radius: var(--radius-md);
  background: color-mix(in srgb, var(--danger) 8%, transparent);
  padding: var(--space-2) var(--space-3);
  color: var(--danger);
  font-size: var(--text-xs);
}

.macro-more {
  position: relative;
  display: flex;
}

.macro-more-menu {
  position: absolute;
  top: calc(100% + var(--space-2));
  right: 0;
  z-index: 40;
  min-width: 168px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  box-shadow: var(--shadow-md);
  padding: var(--space-1);
}

.macro-more-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  padding: var(--space-2) var(--space-3);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
}

.macro-more-item:hover:not(:disabled),
.macro-more-item:focus-visible {
  background: var(--accent-soft);
  color: var(--accent);
}

.macro-more-item:disabled {
  opacity: 0.5;
  cursor: default;
}

.macro-more-item.is-danger:hover:not(:disabled),
.macro-more-item.is-danger:focus-visible {
  background: color-mix(in srgb, var(--danger) 10%, transparent);
  color: var(--danger);
}

.macro-more-divider {
  height: 1px;
  margin: var(--space-1) 0;
  background: var(--border);
}

.macro-more-item svg {
  color: currentColor;
}
</style>
