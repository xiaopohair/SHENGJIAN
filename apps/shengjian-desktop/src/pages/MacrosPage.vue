<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Command, Copy, FileDown, FileUp, Info, Pencil, Play, Plus, Trash2 } from "lucide-vue-next";
import {
  errorMessage,
  exportMacroLibrary,
  getMacroLibrary,
  getMacroRunStatus,
  importMacroLibrary,
  saveMacroLibrary,
  testMacro,
  cancelMacro,
} from "../lib/api";
import {
  compareMacroVersions,
  createMacroStep,
  latestMacroDefinitions,
  MACRO_ACTIONS as ACTIONS,
  nextMacroVersion,
  type MacroDraft,
} from "../lib/macros";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { confirmAction } from "../lib/confirm";
import { useDraftGuard } from "../composables/useDraftGuard";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import IconButton from "../components/ui/IconButton.vue";
import Notice from "../components/ui/Notice.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import TextInput from "../components/ui/TextInput.vue";
import MacroEditor from "../components/MacroEditor.vue";
import type {
  MacroChord,
  MacroDefinition,
  MacroLibraryDocument,
  MacroRunStatus,
  MacroStep,
} from "../lib/types";

const macroDocument = ref<MacroLibraryDocument | null>(null);
const editor = ref<MacroDraft | null>(null);
const editingNew = ref(false);
const busy = ref(false);
const loading = ref(true);
const testingId = ref<string | null>(null);
const testResult = ref<string | null>(null);
const expandedId = ref<string | null>(null);
const copiedId = ref<string | null>(null);
const editorHost = ref<HTMLElement | null>(null);
const baselineJson = ref("");
const runStates = ref<Record<string, MacroRunStatus>>({});
const error = ref<string | null>(null);
const macroSearch = ref("");
let runTimer: number | undefined;
let copyTimer: number | undefined;

const { dirty, confirmDiscard } = useDraftGuard({
  draft: editor,
  baseline: baselineJson,
  discardMessage: "当前宏的修改尚未保存，继续将丢失这些修改。",
});

const MODIFIER_LABELS: Record<string, string> = {
  control: "Ctrl",
  alt: "Alt",
  shift: "Shift",
  win: "Win",
};

const KEY_LABELS: Record<string, string> = {
  space: "Space",
  enter: "Enter",
  tab: "Tab",
  backspace: "Backspace",
  delete: "Delete",
  insert: "Insert",
  home: "Home",
  end: "End",
  pageup: "PageUp",
  pagedown: "PageDown",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
};

function compareVersions(a: string, b: string): number {
  return compareMacroVersions(a, b);
}

const versionsByMacro = computed(() => {
  const versions = new Map<string, MacroDefinition[]>();
  for (const definition of macroDocument.value?.definitions ?? []) {
    const list = versions.get(definition.macroId) ?? [];
    list.push(definition);
    versions.set(definition.macroId, list);
  }
  for (const list of versions.values()) {
    list.sort((left, right) => compareVersions(right.version, left.version));
  }
  return versions;
});

function versionsFor(macroId: string): MacroDefinition[] {
  return versionsByMacro.value.get(macroId) ?? [];
}

function nextVersion(macroId: string): string {
  return nextMacroVersion(macroDocument.value?.definitions ?? [], macroId);
}

const latestMacros = computed(() => latestMacroDefinitions(macroDocument.value?.definitions ?? []));

const otherMacros = computed(() =>
  latestMacros.value.filter((definition) => definition.macroId !== editor.value?.macroId),
);

const visibleMacros = computed(() => {
  const query = macroSearch.value.trim().toLowerCase();
  if (!query) return latestMacros.value;
  return latestMacros.value.filter((definition) => {
    const haystack = [
      definition.name,
      definition.summary ?? "",
      ...definition.steps.map((step) => stepSummary(step)),
    ]
      .join(" ")
      .toLowerCase();
    return haystack.includes(query);
  });
});

function keyLabel(key: string): string {
  if (!key) return "";
  const mapped = KEY_LABELS[key];
  if (mapped) return mapped;
  return key.length === 1 ? key.toUpperCase() : key;
}

function chordLabel(value: MacroChord): string {
  const parts = value.modifiers
    .map((modifier) => MODIFIER_LABELS[modifier] ?? modifier)
    .filter(Boolean);
  const key = keyLabel(value.key);
  if (key) parts.push(key);
  return parts.join(" + ");
}

function waitStateLabel(value: string | null | undefined): string {
  return value === "visible_window" ? tr("窗口可见") : tr("进程运行");
}

function versionCount(macroId: string): number {
  return versionsFor(macroId).length;
}

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

function runReportFor(macroId: string) {
  return runStates.value[macroId]?.report ?? null;
}

async function refreshRunStates() {
  const entries = await Promise.all(
    latestMacros.value.map(async (definition) => {
      try {
        const status = await getMacroRunStatus(definition.macroId);
        return [definition.macroId, status] as const;
      } catch {
        return null;
      }
    }),
  );
  const next: Record<string, MacroRunStatus> = {};
  for (const entry of entries) {
    if (entry) next[entry[0]] = entry[1];
  }
  runStates.value = next;
}

async function load() {
  try {
    macroDocument.value = await getMacroLibrary();
    await refreshRunStates();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    loading.value = false;
  }
}

function toggleDetails(macroId: string) {
  expandedId.value = expandedId.value === macroId ? null : macroId;
}

function syncBaseline() {
  baselineJson.value = editor.value ? JSON.stringify(editor.value) : "";
}

function closeEditor() {
  editor.value = null;
  editingNew.value = false;
  baselineJson.value = "";
}

async function openNew() {
  if (!(await confirmDiscard())) return;
  editingNew.value = true;
  editor.value = {
    macroId: `macro-${crypto.randomUUID()}`,
    name: tr("新宏"),
    summary: "",
    version: "1.0.0",
    steps: [createMacroStep()],
  };
  syncBaseline();
}

async function openEdit(definition: MacroDefinition) {
  if (!(await confirmDiscard())) return;
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

async function editLatest(definition: MacroDefinition) {
  if (editor.value?.macroId === definition.macroId && !editingNew.value) return;
  await openEdit(definition);
}

async function cancelEdit() {
  if (!(await confirmDiscard())) return;
  closeEditor();
}

function chord(step: MacroStep): MacroChord {
  return step.parameters.chord ?? { modifiers: [], key: "" };
}

function actionLabel(actionId: string): string {
  return trDynamic(ACTIONS.find((action) => action.id === actionId)?.label ?? actionId);
}

function stepSummary(step: MacroStep): string {
  switch (step.action) {
    case "send_shortcut":
    case "system_action":
      return chordLabel(chord(step));
    case "open_application":
    case "focus_input":
      return step.parameters.target ?? "";
    case "open_url":
      return step.parameters.url ?? "";
    case "wait_for_application":
      return `${step.parameters.target ?? ""} / ${waitStateLabel(step.parameters.state)}`;
    case "run_macro":
      return step.parameters.nestedMacroName ?? "";
    default:
      return tr("前台输入框");
  }
}

function stepsSummary(steps: MacroStep[]): string {
  return steps.map((step) => stepSummary(step)).join(" → ");
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
      return tr("{position}超时超出范围（100–{max}ms）", { position, max: maxTimeout });
    }
    switch (step.action) {
      case "send_shortcut":
      case "system_action":
        if (!chord(step).key.trim()) return tr("{position}缺少快捷键", { position });
        break;
      case "open_application":
      case "focus_input":
        if (!(step.parameters.target ?? "").trim())
          return tr("{position}缺少应用目标", { position });
        break;
      case "wait_for_application":
        if (!(step.parameters.target ?? "").trim())
          return tr("{position}缺少应用目标", { position });
        break;
      case "open_url":
        if (!/^https?:\/\/\S+/i.test(step.parameters.url ?? "")) {
          return tr("{position}网址必须是 http/https", { position });
        }
        break;
      case "run_macro":
        if (!(step.parameters.nestedMacroId ?? "").trim()) {
          return tr("{position}未选择目标宏", { position });
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
    error.value = validation;
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    const definition: MacroDefinition = {
      schemaVersion: "0.1",
      macroId: editor.value.macroId,
      version: editingNew.value ? "1.0.0" : nextVersion(editor.value.macroId),
      name: editor.value.name.trim(),
      summary: editor.value.summary.trim() || null,
      steps: editor.value.steps,
    };
    const definitions = [...macroDocument.value.definitions, definition];
    macroDocument.value = await saveMacroLibrary({
      ...macroDocument.value,
      definitions,
    });
    closeEditor();
    await refreshRunStates();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function removeMacro(definition: MacroDefinition) {
  if (!macroDocument.value) return;
  const confirmed = await confirmAction({
    title: tr("删除宏“{name}”？", { name: definition.name }),
    message: tr("此操作不可恢复。"),
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
    if (expandedId.value === definition.macroId) expandedId.value = null;
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function removeVersion(definition: MacroDefinition) {
  if (!macroDocument.value) return;
  const versions = versionsFor(definition.macroId);
  if (versions.length <= 1) {
    error.value = tr("至少保留一个宏版本");
    return;
  }
  const confirmed = await confirmAction({
    title: tr("删除宏“{name}”的 v{version}？", {
      name: definition.name,
      version: definition.version,
    }),
    message: tr("此操作不可恢复。"),
    confirmLabel: tr("删除"),
    danger: true,
  });
  if (!confirmed) return;
  busy.value = true;
  error.value = null;
  try {
    const definitions = macroDocument.value.definitions.filter(
      (item) => !(item.macroId === definition.macroId && item.version === definition.version),
    );
    macroDocument.value = await saveMacroLibrary({
      ...macroDocument.value,
      definitions,
    });
    await refreshRunStates();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function runMacro(definition: MacroDefinition) {
  testingId.value = definition.macroId;
  testResult.value = null;
  error.value = null;
  try {
    await testMacro(definition.macroId);
    testResult.value = tr("宏“{name}”已启动", { name: definition.name });
    await refreshRunStates();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    testingId.value = null;
  }
}

async function stopMacro(definition: MacroDefinition) {
  error.value = null;
  try {
    const cancelled = await cancelMacro(definition.macroId);
    testResult.value = cancelled
      ? tr("已请求取消“{name}”", { name: definition.name })
      : tr("宏没有在运行");
    await refreshRunStates();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  }
}

async function exportFile() {
  if (!macroDocument.value) return;
  error.value = null;
  try {
    const path = await save({
      defaultPath: "macros.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await exportMacroLibrary(path, macroDocument.value);
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  }
}

async function importFile() {
  if (!(await confirmDiscard())) return;
  error.value = null;
  busy.value = true;
  try {
    const path = await open({
      multiple: false,
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path || Array.isArray(path)) return;
    macroDocument.value = await importMacroLibrary(path);
    closeEditor();
    expandedId.value = null;
    testResult.value = null;
    await refreshRunStates();
  } catch (cause) {
    error.value = macroErrorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function copyId(macroId: string) {
  try {
    await navigator.clipboard.writeText(macroId);
    copiedId.value = macroId;
    if (copyTimer !== undefined) window.clearTimeout(copyTimer);
    copyTimer = window.setTimeout(() => {
      copiedId.value = null;
    }, 1500);
  } catch {
    error.value = tr("无法复制 ID，请手动选择文本。");
  }
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

onMounted(async () => {
  await load();
  runTimer = window.setInterval(() => {
    if (window.document.visibilityState === "visible") void refreshRunStates();
  }, 3000);
});

onUnmounted(() => {
  if (runTimer !== undefined) window.clearInterval(runTimer);
  if (copyTimer !== undefined) window.clearTimeout(copyTimer);
});
</script>

<template>
  <section class="page">
    <PageHeader
      eyebrow="Macros"
      title="宏"
      description="创建可复用的动作序列，绑定到按键或手动测试。"
    />

    <Notice v-if="error" kind="error">{{ error }}</Notice>
    <Notice v-if="testResult" kind="info">{{ testResult }}</Notice>

    <div class="split-layout">
      <Card class="stack stack-sm library-card">
        <SectionHeader
          title="宏库"
          :hint="
            trDynamic('{count} 个宏 · {versions} 个版本', {
              count: latestMacros.length,
              versions: macroDocument?.definitions.length ?? 0,
            })
          "
        />
        <div class="library-tools">
          <TextInput
            v-model="macroSearch"
            type="search"
            :placeholder="trDynamic('搜索宏 / 动作')"
            :aria-label="trDynamic('搜索宏')"
          />
          <div class="toolbar-actions">
            <Button size="sm" :disabled="busy" @click="importFile">
              <FileUp :size="14" />
              {{ tr("导入") }}
            </Button>
            <Button size="sm" :disabled="busy || !macroDocument" @click="exportFile">
              <FileDown :size="14" />
              {{ tr("导出") }}
            </Button>
            <Button size="sm" variant="primary" :disabled="busy" @click="openNew">
              <Plus :size="14" />
              {{ tr("新建") }}
            </Button>
          </div>
        </div>

        <LoadingState v-if="loading" label="正在加载…" description="正在读取本地宏库…" />

        <EmptyState
          v-else-if="macroDocument && latestMacros.length === 0"
          title="还没有宏"
          description="新建一个宏，或从导入宏库开始。"
        >
          <template #icon><Command :size="20" /></template>
          <template #actions>
            <Button variant="primary" @click="openNew">{{ tr("新建宏") }}</Button>
          </template>
        </EmptyState>
        <Notice v-else-if="visibleMacros.length === 0" kind="info">
          {{ tr("没有匹配“{query}”的宏。", { query: macroSearch }) }}
        </Notice>

        <ul v-else class="macro-list">
          <li
            v-for="definition in visibleMacros"
            :key="definition.macroId"
            class="macro-item"
            :class="{ editing: editor?.macroId === definition.macroId }"
          >
            <div class="macro-row">
              <span class="macro-icon"><Command :size="16" /></span>
              <div class="macro-main">
                <span class="macro-name-row">
                  <span class="macro-name text-truncate" :title="definition.name">
                    {{ definition.name }}
                  </span>
                  <StatusPill :label="`v${definition.version}`" tone="neutral" />
                  <StatusPill
                    v-if="statusText(definition.macroId)"
                    :label="statusText(definition.macroId)"
                    :tone="runTone(definition.macroId)"
                    dot
                  />
                  <StatusPill
                    v-if="editor?.macroId === definition.macroId"
                    :label="trDynamic('编辑中')"
                    tone="accent"
                    dot
                  />
                </span>
                <span class="macro-steps text-truncate" :title="stepsSummary(definition.steps)">
                  {{ stepsSummary(definition.steps) }}
                </span>
                <span class="macro-meta text-truncate">
                  {{ tr("{count} 个步骤", { count: definition.steps.length }) }}
                  · {{ tr("{count} 个版本", { count: versionCount(definition.macroId) }) }}
                  <template v-if="definition.summary"> · {{ definition.summary }}</template>
                </span>
              </div>
              <div class="macro-actions">
                <Button
                  v-if="runStates[definition.macroId]?.running"
                  size="sm"
                  @click="stopMacro(definition)"
                >
                  {{ tr("停止") }}
                </Button>
                <Button
                  v-else
                  size="sm"
                  variant="primary"
                  :disabled="testingId !== null"
                  @click="runMacro(definition)"
                >
                  <Play :size="14" />
                  {{
                    trDynamic(
                      testingId === definition.macroId
                        ? "启动中"
                        : statusText(definition.macroId)
                          ? "重试"
                          : "测试",
                    )
                  }}
                </Button>
                <IconButton
                  size="sm"
                  :label="trDynamic('复制宏')"
                  :disabled="busy"
                  @click="duplicateMacro(definition)"
                >
                  <Copy :size="14" />
                </IconButton>
                <IconButton size="sm" :label="trDynamic('编辑宏')" @click="editLatest(definition)">
                  <Pencil :size="14" />
                </IconButton>
                <IconButton
                  size="sm"
                  variant="danger"
                  :label="trDynamic('删除宏')"
                  :disabled="busy"
                  @click="removeMacro(definition)"
                >
                  <Trash2 :size="14" />
                </IconButton>
                <IconButton
                  size="sm"
                  :label="trDynamic('详情')"
                  :aria-expanded="expandedId === definition.macroId"
                  @click="toggleDetails(definition.macroId)"
                >
                  <Info :size="14" />
                </IconButton>
              </div>
            </div>

            <div v-if="expandedId === definition.macroId" class="macro-details">
              <div class="detail-id">
                <span class="muted">{{ tr("标识") }}</span>
                <code class="mono text-truncate" :title="definition.macroId">
                  {{ definition.macroId }}
                </code>
                <Button size="sm" variant="ghost" @click="copyId(definition.macroId)">
                  {{ trDynamic(copiedId === definition.macroId ? "已复制" : "复制 ID") }}
                </Button>
              </div>

              <div v-if="runReportFor(definition.macroId)" class="detail-block">
                <p class="detail-title">
                  {{ tr("最近运行") }}
                  <span class="muted">
                    · {{ statusLabel(runReportFor(definition.macroId)?.result ?? "") }} ·
                    {{ runReportFor(definition.macroId)?.elapsedMs }}ms
                  </span>
                </p>
                <div class="run-report">
                  <span
                    v-for="step in runReportFor(definition.macroId)?.steps ?? []"
                    :key="step.index"
                    class="run-step"
                  >
                    #{{ step.index + 1 }} {{ actionLabel(step.action) }} ·
                    {{ statusLabel(step.outcome) }} · {{ step.elapsedMs }}ms
                  </span>
                </div>
              </div>

              <div class="detail-block version-block">
                <p class="detail-title">
                  {{ tr("版本历史") }}
                  <span class="muted">
                    · {{ tr("{count} 个版本", { count: versionCount(definition.macroId) }) }}
                  </span>
                </p>
                <ul class="version-list">
                  <li
                    v-for="version in versionsFor(definition.macroId)"
                    :key="version.version"
                    class="version-row"
                  >
                    <span class="mono version-tag">v{{ version.version }}</span>
                    <span class="version-name text-truncate" :title="version.name">
                      {{ version.name }}
                    </span>
                    <span class="muted version-steps">
                      {{ tr("{count} 个步骤", { count: version.steps.length }) }}
                    </span>
                    <Button size="sm" variant="ghost" @click="openEdit(version)">
                      {{ tr("基于此版本编辑") }}
                    </Button>
                    <IconButton
                      size="sm"
                      variant="danger"
                      :label="trDynamic('删除此版本')"
                      :disabled="busy || versionCount(definition.macroId) <= 1"
                      @click="removeVersion(version)"
                    >
                      <Trash2 :size="14" />
                    </IconButton>
                  </li>
                </ul>
              </div>
            </div>
          </li>
        </ul>
      </Card>

      <div ref="editorHost" class="editor-host">
        <MacroEditor
          v-if="editor"
          v-model:draft="editor"
          :editing-new="editingNew"
          :busy="busy"
          :dirty="dirty"
          :other-macros="otherMacros"
          @save="saveEditor"
          @cancel="cancelEdit"
        />

        <Card v-else class="editor-placeholder">
          <EmptyState title="从左侧选择一个宏开始编辑" description="新建一个宏，或从导入宏库开始。">
            <template #icon><Command :size="20" /></template>
            <template #actions>
              <Button variant="primary" @click="openNew">{{ tr("新建宏") }}</Button>
            </template>
          </EmptyState>
        </Card>
      </div>
    </div>
  </section>
</template>

<style scoped>
.library-tools {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.macro-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.macro-item {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.macro-item:hover {
  background: var(--bg-subtle);
}

.macro-item.editing {
  border-color: var(--accent);
}

.macro-row {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr);
  gap: var(--space-2) var(--space-3);
  align-items: start;
  padding: var(--space-3);
}

.macro-icon {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  background: var(--bg-hover);
  color: var(--accent);
}

.macro-main {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.macro-name-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.macro-name {
  font-weight: 600;
}

.macro-steps {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.macro-meta {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.macro-actions {
  display: flex;
  flex-wrap: wrap;
  grid-column: 1 / -1;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-1);
}

.macro-details {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  border-top: 1px dashed var(--border);
  padding: var(--space-3);
}

.detail-id {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  font-size: var(--text-xs);
}

.detail-id code {
  flex: 1 1 120px;
  min-width: 0;
}

.detail-block {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.detail-title {
  color: var(--text-secondary);
  font-size: var(--text-xs);
  font-weight: 600;
}

.run-report {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-1);
}

.run-step {
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 2px 6px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.version-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.version-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  border-left: 2px solid var(--border-strong);
  padding: var(--space-1) var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.version-tag {
  color: var(--text);
}

.version-name {
  flex: 1 1 100px;
  min-width: 0;
}

.version-steps {
  flex: 0 0 auto;
}

.editor-host {
  min-width: 0;
  scroll-margin-top: var(--space-4);
}

@media (max-width: 1120px) {
  .editor-placeholder {
    display: none;
  }
}
</style>
