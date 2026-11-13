<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Copy,
  FileDown,
  FileUp,
  Keyboard,
  LayoutGrid,
  Plus,
  RotateCcw,
  SlidersHorizontal,
  Table2,
  Trash2,
} from "lucide-vue-next";
import { latestMacroDefinitions } from "../lib/macros";
import {
  PROFILE_BUTTONS,
  PROFILE_TRIGGERS,
  isDefaultProfile,
  profileAppliesToModel,
  type ProfileDraft,
  type ProfileTrigger,
} from "../lib/buttons";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { confirmAction } from "../lib/confirm";
import { useDraftGuard } from "../composables/useDraftGuard";
import { useEngineStore } from "../stores/engine";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import IconButton from "../components/ui/IconButton.vue";
import Notice from "../components/ui/Notice.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import Switch from "../components/ui/Switch.vue";
import ProfileWorkbench from "../components/ProfileWorkbench.vue";
import ProfileSettingsDrawer from "../components/ProfileSettingsDrawer.vue";
import {
  activateButtonProfile,
  clearButtonProfileActivation,
  errorMessage,
  exportProfiles,
  getButtonProfileState,
  getForegroundExecutable,
  getHostCapabilities,
  getMacroLibrary,
  importProfiles,
  saveButtonProfiles,
} from "../lib/api";
import type {
  ButtonProfile,
  ButtonProfilesDocument,
  LogicalAction,
  MacroDefinition,
} from "../lib/types";

const TRIGGERS = PROFILE_TRIGGERS;
type Trigger = ProfileTrigger;

const profilesDocument = ref<ButtonProfilesDocument | null>(null);
const activeProfileId = shallowRef<string | null>(null);
const engineStore = useEngineStore();
const remoteConnected = computed(() => engineStore.isConnected);
const remoteModel = computed(() => engineStore.snapshot?.connection.remote?.model ?? null);
/** 蓝牙名：型号识别不出来时，界面退回显示它。 */
const remoteName = computed(() => engineStore.snapshot?.connection.remote?.name ?? null);

/** 上次连接过的型号（未连接时用它决定展示哪一套方案）。 */
const LAST_MODEL_KEY = "shengjian.last-model";
const rememberedModel = shallowRef<string | null>(readRememberedModel());

function readRememberedModel(): string | null {
  try {
    const value = window.localStorage.getItem(LAST_MODEL_KEY)?.trim();
    return value ? value.toUpperCase() : null;
  } catch {
    return null;
  }
}

watch(
  remoteModel,
  (model) => {
    const normalized = model?.trim().toUpperCase();
    if (!normalized) return;
    rememberedModel.value = normalized;
    try {
      window.localStorage.setItem(LAST_MODEL_KEY, normalized);
    } catch {
      // localStorage 不可用不影响主流程
    }
  },
  { immediate: true },
);

/**
 * 当前型号：已连接型号 → 上次连接型号 → 未知。
 *
 * 方案按型号隔离，所以「现在该展示哪一套方案」完全取决于这个值；型号未知时只有
 * 通用方案可见。
 */
const effectiveModel = computed(
  () => remoteModel.value?.trim().toUpperCase() || rememberedModel.value,
);

/** 当前型号可见的方案；型号已有自己的默认方案时不再列出通用默认。 */
const visibleProfiles = computed(() => {
  const model = effectiveModel.value;
  const all = profilesDocument.value?.profiles ?? [];
  const own = (profile: ButtonProfile) => (profile.deviceModel ?? "").trim().toUpperCase();
  const hasOwnDefault = Boolean(
    model && all.some((profile) => isDefaultProfile(profile) && own(profile) === model),
  );
  return all.filter((profile) => {
    if (!profileAppliesToModel(profile, model)) return false;
    if (hasOwnDefault && isDefaultProfile(profile) && !own(profile)) return false;
    return true;
  });
});

/** 当前型号的默认方案（后端保证每个型号恒有一份）。 */
const defaultProfile = computed(
  () => visibleProfiles.value.find((profile) => isDefaultProfile(profile)) ?? null,
);

/**
 * 型号变了就把编辑区换到新型号的默认方案。
 *
 * 不换的话会留下一个「型号 A 的方案」的草稿，而遥控器已经换成型号 B——界面上的
 * 触发总数、按键布局全对不上，用户还会以为正在编辑的是当前这台的方案。
 * 有未保存修改时先确认，避免默默丢弃。
 */
watch(effectiveModel, async (model) => {
  const draft = editor.value;
  if (!draft) return;
  const draftModel = (draft.deviceModel ?? "").trim().toUpperCase();
  if (!draftModel || draftModel === (model ?? "")) return;
  if (dirty.value && !(await confirmDiscard())) return;
  closeEditor();
  autoOpened = false;
  const next = visibleProfiles.value.find((profile) => isDefaultProfile(profile));
  if (next) {
    autoOpened = true;
    openDraft(next);
  }
});

const macroDefinitions = ref<MacroDefinition[]>([]);
const commonPhrasesAvailable = shallowRef(false);
const foregroundApp = shallowRef("");
const latestMacros = computed(() => latestMacroDefinitions(macroDefinitions.value));

const autoProfileConflicts = computed(() => {
  const byTarget = new Map<string, string[]>();
  for (const profile of profilesDocument.value?.profiles ?? []) {
    if (!profile.autoApply) continue;
    for (const target of profile.appTargets) {
      const key = target.trim().toLowerCase();
      if (!key) continue;
      const names = byTarget.get(key) ?? [];
      names.push(profile.name);
      byTarget.set(key, names);
    }
  }
  return [...byTarget.entries()]
    .filter(([, names]) => names.length > 1)
    .map(([target, names]) => ({ target, names }));
});
const busy = shallowRef(false);
const loading = shallowRef(true);
const error = shallowRef<string | null>(null);
const editor = ref<ProfileDraft | null>(null);
const editingNew = shallowRef(false);
const baselineJson = shallowRef("");
const view = ref<"workbench" | "table">("workbench");
const settingsOpen = shallowRef(false);
const justSaved = shallowRef(false);
let timer: number | undefined;
let savedTimer: number | undefined;
let autoOpened = false;

const { dirty, confirmDiscard } = useDraftGuard({
  draft: editor,
  baseline: baselineJson,
  discardMessage: "当前方案的修改尚未保存，继续将丢失这些修改。",
});

async function load() {
  try {
    const state = await getButtonProfileState();
    profilesDocument.value = state.document;
    activeProfileId.value = state.activeProfileId;
    if (!editor.value && !autoOpened) {
      const candidates = visibleProfiles.value;
      if (candidates.length > 0) {
        autoOpened = true;
        const initial =
          candidates.find((profile) => profile.id === state.activeProfileId) ??
          candidates.find((profile) => isDefaultProfile(profile)) ??
          candidates[0];
        openDraft(initial);
      }
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}

async function loadMacros() {
  try {
    macroDefinitions.value = (await getMacroLibrary()).definitions;
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function loadHostCapabilities() {
  try {
    commonPhrasesAvailable.value = (await getHostCapabilities()).commonPhrases;
  } catch {
    commonPhrasesAvailable.value = false;
  }
}

async function loadForeground() {
  try {
    foregroundApp.value = (await getForegroundExecutable()) ?? "";
  } catch {
    foregroundApp.value = "";
  }
}

function cloneBindings(profile: ButtonProfile): ProfileDraft["bindings"] {
  return JSON.parse(JSON.stringify(profile.bindings ?? {})) as ProfileDraft["bindings"];
}

function repeatSettingsFromProfile(profile: ButtonProfile): ProfileDraft["holdRepeatByButton"] {
  return Object.fromEntries(
    Object.entries(profile.holdRepeatByButton ?? {}).map(([button, setting]) => [
      button,
      setting === null ? "off" : setting,
    ]),
  ) as ProfileDraft["holdRepeatByButton"];
}

function repeatSettingsForSave(draft: ProfileDraft): Record<string, "single" | "long" | null> {
  return Object.fromEntries(
    Object.entries(draft.holdRepeatByButton ?? {}).map(([button, setting]) => [
      button,
      setting === "off" ? null : setting,
    ]),
  );
}

function syncBaseline() {
  baselineJson.value = editor.value ? JSON.stringify(editor.value) : "";
}

function closeEditor() {
  editor.value = null;
  editingNew.value = false;
  baselineJson.value = "";
}

function createNewDraft() {
  editingNew.value = true;
  justSaved.value = false;
  editor.value = {
    id: `profile-${crypto.randomUUID()}`,
    name: tr("新方案"),
    appTargets: "",
    autoApply: false,
    holdRepeat: "",
    voiceInputTool: "",
    // 新方案归当前型号所有，这样它与别的型号互不干扰。
    deviceModel: effectiveModel.value,
    isDefault: false,
    bindings: {},
  };
  syncBaseline();
}

async function startNew() {
  if (!(await confirmDiscard())) return;
  createNewDraft();
}

function onProfileWheel(event: WheelEvent) {
  const element = event.currentTarget as HTMLElement;
  if (element.scrollWidth <= element.clientWidth) return;
  if (Math.abs(event.deltaY) <= Math.abs(event.deltaX)) return;
  element.scrollLeft += event.deltaY;
  event.preventDefault();
}

watch(
  () => editor.value?.id,
  async () => {
    await nextTick();
    document
      .querySelector(".profile-chip.selected")
      ?.scrollIntoView?.({ block: "nearest", inline: "nearest", behavior: "smooth" });
  },
);

async function editProfile(profile: ButtonProfile) {
  if (editor.value?.id === profile.id) return;
  if (!(await confirmDiscard())) return;
  openDraft(profile);
}

function openDraft(profile: ButtonProfile) {
  editingNew.value = false;
  justSaved.value = false;
  editor.value = {
    id: profile.id,
    name: profile.name,
    appTargets: profile.appTargets.join(", "),
    autoApply: profile.autoApply,
    holdRepeat: profile.holdRepeat ?? "",
    holdRepeatByButton: repeatSettingsFromProfile(profile),
    voiceInputTool: profile.voiceInputTool ?? "",
    deviceModel: profile.deviceModel ?? null,
    isDefault: isDefaultProfile(profile),
    bindings: cloneBindings(profile),
  };
  syncBaseline();
}

async function cancelEdit() {
  if (!(await confirmDiscard())) return;
  closeEditor();
}

function cleanBindings(): ProfileDraft["bindings"] {
  const result: ProfileDraft["bindings"] = {};
  for (const button of PROFILE_BUTTONS) {
    const triggers = editor.value?.bindings[button.id];
    if (!triggers) continue;
    const clean: Partial<Record<Trigger, LogicalAction>> = {};
    for (const trigger of TRIGGERS) {
      const action = triggers[trigger];
      if (action) clean[trigger] = action;
    }
    if (Object.keys(clean).length > 0) result[button.id] = clean;
  }
  return result;
}

function validateProfileDraft(): string | null {
  if (!editor.value) return tr("没有正在编辑的方案");
  if (!editor.value.name.trim()) return tr("方案名称不能为空");
  if (editor.value.autoApply && editor.value.appTargets.trim() === "") {
    return tr("开启自动应用时必须填写至少一个应用目标");
  }
  for (const [button, triggers] of Object.entries(editor.value.bindings)) {
    for (const [trigger, action] of Object.entries(triggers ?? {})) {
      if (!action) continue;
      const buttonLabel = PROFILE_BUTTONS.find((item) => item.id === button)?.label ?? button;
      const position = `${trDynamic(buttonLabel)} / ${statusLabel(trigger)}`;
      if (action.type === "shortcut" && (!Array.isArray(action.keys) || action.keys.length === 0)) {
        return tr("{position} 缺少快捷键", { position });
      }
      if (action.type === "open-app" && !String(action.app ?? "").trim()) {
        return tr("{position} 缺少应用目标", { position });
      }
      if (action.type === "run-macro" && !String(action.macro_id ?? "").trim()) {
        return tr("{position} 未选择宏", { position });
      }
    }
  }
  return null;
}

function onEditorError(message: string) {
  error.value = message;
}

async function saveEditor() {
  if (!editor.value || !profilesDocument.value) return;
  const validation = validateProfileDraft();
  if (validation) {
    error.value = validation;
    return;
  }
  busy.value = true;
  error.value = null;
  try {
    const profile: ButtonProfile = {
      id: editor.value.id,
      name: editor.value.name,
      appTargets: editor.value.appTargets
        .split(",")
        .map((target) => target.trim())
        .filter(Boolean),
      autoApply: editor.value.autoApply,
      holdRepeat: editor.value.holdRepeat || null,
      holdRepeatByButton: repeatSettingsForSave(editor.value),
      voiceInputTool: editor.value.voiceInputTool || null,
      deviceModel: editor.value.deviceModel ?? effectiveModel.value ?? null,
      isDefault: Boolean(editor.value.isDefault),
      bindings: cleanBindings(),
    };
    const profiles = profilesDocument.value.profiles.some((item) => item.id === profile.id)
      ? profilesDocument.value.profiles.map((item) => (item.id === profile.id ? profile : item))
      : [...profilesDocument.value.profiles, profile];
    await saveButtonProfiles({ ...profilesDocument.value, profiles });
    editingNew.value = false;
    await load();
    syncBaseline();
    justSaved.value = true;
    if (savedTimer !== undefined) window.clearTimeout(savedTimer);
    savedTimer = window.setTimeout(() => {
      justSaved.value = false;
    }, 2200);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function exportFile() {
  if (!profilesDocument.value) return;
  error.value = null;
  try {
    const path = await save({
      defaultPath: "button-profiles.json",
      filters: [{ name: "JSON", extensions: ["json"] }],
    });
    if (!path) return;
    await exportProfiles(path, profilesDocument.value);
  } catch (cause) {
    error.value = errorMessage(cause);
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
    await importProfiles(path);
    closeEditor();
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function removeProfile(profile: ButtonProfile) {
  if (!profilesDocument.value) return;
  // 默认方案是每个型号的基线，不可删除；后端归一化也会重建它。
  if (isDefaultProfile(profile)) return;
  const confirmed = await confirmAction({
    title: tr("删除方案“{name}”？", { name: profile.name }),
    message: tr("此操作不可恢复。"),
    confirmLabel: tr("删除"),
    danger: true,
  });
  if (!confirmed) return;
  busy.value = true;
  error.value = null;
  try {
    const profiles = profilesDocument.value.profiles.filter((item) => item.id !== profile.id);
    await saveButtonProfiles({ ...profilesDocument.value, profiles });
    if (editor.value?.id === profile.id) closeEditor();
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function duplicateProfile(profile: ButtonProfile) {
  if (!profilesDocument.value) return;
  busy.value = true;
  error.value = null;
  try {
    const copy: ButtonProfile = {
      ...profile,
      id: `profile-${crypto.randomUUID()}`,
      name: tr("{name} 副本", { name: profile.name }),
      autoApply: false,
      // 副本永远是普通方案：默认方案每个型号只能有一份。
      isDefault: false,
      bindings: JSON.parse(JSON.stringify(profile.bindings ?? {})),
    };
    await saveButtonProfiles({
      ...profilesDocument.value,
      profiles: [...profilesDocument.value.profiles, copy],
    });
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function activate(profile: ButtonProfile) {
  busy.value = true;
  error.value = null;
  try {
    if (!(await activateButtonProfile(profile.id))) {
      error.value = tr("未找到该方案，可能已被删除。");
    }
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

function currentProfile(): ButtonProfile | null {
  if (!editor.value) return null;
  return profilesDocument.value?.profiles.find((item) => item.id === editor.value?.id) ?? null;
}

const currentProfileRecord = computed(() => currentProfile());
/** 后端 `null` 表示「没有手动方案」→ 当前型号的默认方案在生效。 */
const effectiveActiveId = computed(() => activeProfileId.value ?? defaultProfile.value?.id ?? null);

function isActiveProfile(id: string): boolean {
  return effectiveActiveId.value === id;
}

async function activateEditorProfile() {
  const profile = currentProfile();
  if (profile) await activate(profile);
}

/** One control that reflects the active state: click again to deactivate. */
async function toggleEditorActivation() {
  if (activeProfileId.value && activeProfileId.value === editor.value?.id) {
    await clearActivation();
    return;
  }
  await activateEditorProfile();
}

async function clearActivation() {
  busy.value = true;
  error.value = null;
  try {
    await clearButtonProfileActivation();
    await load();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function duplicateCurrentProfile() {
  const profile = currentProfile();
  if (profile) await duplicateProfile(profile);
}

async function removeCurrentProfile() {
  const profile = currentProfile();
  if (profile) await removeProfile(profile);
}
/**
 * Clear every explicit binding in the open draft so the profile falls back to
 * its inherited defaults (the global profile for normal profiles, the factory
 * mapping set for the global profile itself). Confirmed because it is destructive.
 */
async function resetCurrentProfile() {
  if (!editor.value || busy.value) return;
  const isDefault = Boolean(editor.value.isDefault);
  const confirmed = await confirmAction({
    title: tr("重置方案“{name}”？", { name: editor.value.name || tr("未命名方案") }),
    message: isDefault
      ? tr("将清空默认方案的所有动作，全部回到遥控器出厂默认。此操作不可恢复。")
      : tr("将清空本方案的所有动作，全部回到默认方案的默认值。此操作不可恢复。"),
    confirmLabel: tr("重置"),
    danger: true,
  });
  if (!confirmed) return;
  editor.value = {
    ...editor.value,
    bindings: {},
    holdRepeat: "",
    holdRepeatByButton: {},
  };
}

function openSettings() {
  settingsOpen.value = true;
}

function setView(next: "workbench" | "table") {
  view.value = next;
}

onMounted(async () => {
  await Promise.all([load(), loadMacros(), loadHostCapabilities(), loadForeground()]);
  timer = window.setInterval(() => {
    if (window.document.visibilityState === "visible") void load();
  }, 3000);
});

onUnmounted(() => {
  if (timer !== undefined) window.clearInterval(timer);
  if (savedTimer !== undefined) window.clearTimeout(savedTimer);
});
</script>

<template>
  <section class="page buttons-page">
    <PageHeader
      eyebrow="Buttons"
      title="按键方案"
      description="为遥控器按键绑定动作或宏，并按前台应用自动切换。"
    >
      <template #actions>
        <!-- No profiles and no draft: nothing to switch between yet. -->
        <div
          v-if="profilesDocument?.profiles.length || editor"
          class="view-switch"
          role="tablist"
          aria-label="视图切换"
        >
          <button
            type="button"
            class="view-button"
            :class="{ active: view === 'workbench' }"
            role="tab"
            :aria-selected="view === 'workbench'"
            @click="setView('workbench')"
          >
            <LayoutGrid :size="15" />
            {{ tr("工作台") }}
          </button>
          <button
            type="button"
            class="view-button"
            :class="{ active: view === 'table' }"
            role="tab"
            :aria-selected="view === 'table'"
            @click="setView('table')"
          >
            <Table2 :size="15" />
            {{ tr("表格视图") }}
          </button>
        </div>
      </template>
    </PageHeader>

    <Notice v-if="error" kind="error">{{ error }}</Notice>
    <Notice v-if="autoProfileConflicts.length" kind="warning">
      <strong>{{ tr("自动方案冲突：") }}</strong>
      <span v-for="conflict in autoProfileConflicts" :key="conflict.target">
        {{
          tr("{target} 同时匹配 {names}；", {
            target: conflict.target,
            names: conflict.names.join(" / "),
          })
        }}
      </span>
      {{ tr("请只保留一个自动方案，或修改应用目标。") }}
    </Notice>

    <LoadingState v-if="loading" label="正在加载…" description="正在读取本地键位方案…" />

    <template v-else>
      <div v-if="profilesDocument" class="profile-bar">
        <div
          v-if="profilesDocument.profiles.length || editingNew"
          class="profile-chips"
          @wheel="onProfileWheel"
        >
          <button
            v-if="editingNew && editor"
            type="button"
            class="profile-chip new-chip selected"
            :title="editor.name || tr('新方案')"
          >
            <span class="profile-chip-index">+</span>
            <span class="profile-chip-copy">
              <strong class="text-truncate">{{ editor.name || tr("新方案") }}</strong>
            </span>
          </button>
          <button
            v-for="(profile, index) in visibleProfiles"
            :key="profile.id"
            type="button"
            class="profile-chip"
            :class="{
              active: isActiveProfile(profile.id),
              selected: editor?.id === profile.id,
            }"
            :aria-pressed="editor?.id === profile.id"
            :title="`${profile.name} · ${profile.appTargets.join(', ') || tr('未设置应用目标')}`"
            @click="editProfile(profile)"
          >
            <span class="profile-chip-index">{{ index + 1 }}</span>
            <span class="profile-chip-copy">
              <strong class="text-truncate" :title="profile.name">{{ profile.name }}</strong>
            </span>
            <span
              v-if="isDefaultProfile(profile)"
              class="profile-chip-flag"
              :title="tr('该型号的默认方案')"
            >
              {{ tr("默认") }}
            </span>
            <span
              v-else-if="isActiveProfile(profile.id)"
              class="profile-chip-flag"
              :title="tr('已激活')"
            >
              {{ tr("激活") }}
            </span>
          </button>
        </div>

        <div class="profile-tools">
          <Button size="sm" :disabled="busy" @click="importFile">
            <FileUp :size="14" />
            {{ tr("导入") }}
          </Button>
          <Button
            size="sm"
            :disabled="busy || !profilesDocument || !profilesDocument.profiles.length"
            @click="exportFile"
          >
            <FileDown :size="14" />
            {{ tr("导出") }}
          </Button>
          <Button size="sm" variant="primary" :disabled="busy" @click="startNew">
            <Plus :size="14" />
            {{ tr("新建") }}
          </Button>
        </div>
      </div>

      <div v-if="editor" class="editor-toolbar">
        <div class="editor-context">
          <span class="eyebrow">{{ editingNew ? tr("新建方案") : tr("编辑方案：") }}</span>
          <strong class="editor-name">{{ editor.name || tr("未命名方案") }}</strong>
          <span class="editor-target text-truncate" :title="editor.appTargets">
            {{ editor.appTargets || tr("未设置应用目标") }}
          </span>
          <StatusPill
            :label="
              editor.isDefault ? tr('默认方案') : editor.autoApply ? tr('自动应用') : tr('手动')
            "
            :tone="editor.isDefault ? 'accent' : editor.autoApply ? 'info' : 'neutral'"
            :title="
              editor.isDefault
                ? tr('该型号的基线方案，始终生效')
                : editor.autoApply
                  ? tr('前台应用匹配时自动切换到此方案')
                  : tr('不自动切换，需要手动激活')
            "
          />
          <!-- 默认方案就是当前型号的基线，不存在「激活/未激活」，不显示开关。 -->
          <span v-if="!editingNew && !editor.isDefault" class="activation-control">
            <Switch
              :model-value="isActiveProfile(editor.id)"
              :disabled="busy || !currentProfileRecord"
              :label="tr('激活')"
              @update:model-value="toggleEditorActivation"
            />
            <span class="activation-state" :class="{ on: isActiveProfile(editor.id) }">
              {{ isActiveProfile(editor.id) ? tr("已激活") : tr("未激活") }}
            </span>
          </span>
          <StatusPill v-if="justSaved" :label="trDynamic('已保存')" tone="accent" dot />
          <StatusPill v-if="dirty" :label="trDynamic('未保存')" tone="warning" dot />
        </div>
        <div class="editor-toolbar-actions">
          <Button size="sm" :disabled="busy" @click="cancelEdit">{{ tr("取消") }}</Button>
          <Button
            size="sm"
            variant="primary"
            :disabled="busy || (!dirty && !editingNew)"
            @click="saveEditor"
          >
            {{ tr("保存") }}
          </Button>
          <IconButton
            v-if="!editingNew && currentProfileRecord"
            size="sm"
            :label="trDynamic('重置方案')"
            :disabled="busy"
            @click="resetCurrentProfile"
          >
            <RotateCcw :size="14" />
          </IconButton>
          <IconButton size="sm" :label="trDynamic('属性')" @click="openSettings">
            <SlidersHorizontal :size="15" />
          </IconButton>
          <IconButton
            v-if="!editingNew && currentProfileRecord"
            size="sm"
            :label="trDynamic('复制方案')"
            :disabled="busy"
            @click="duplicateCurrentProfile"
          >
            <Copy :size="14" />
          </IconButton>
          <IconButton
            v-if="!editingNew && currentProfileRecord && !isDefaultProfile(currentProfileRecord)"
            size="sm"
            variant="danger"
            :label="trDynamic('删除方案')"
            :disabled="busy"
            @click="removeCurrentProfile"
          >
            <Trash2 :size="14" />
          </IconButton>
        </div>
      </div>

      <ProfileWorkbench
        v-if="editor"
        v-model:draft="editor"
        :view="view"
        :busy="busy"
        :dirty="dirty"
        :baseline-json="baselineJson"
        :foreground-app="foregroundApp"
        :common-phrases-available="commonPhrasesAvailable"
        :latest-macros="latestMacros"
        :remote-connected="remoteConnected"
        :remote-model="remoteModel"
        :remote-name="remoteName"
        @error="onEditorError"
      />

      <EmptyState
        v-else-if="!profilesDocument?.profiles.length"
        title="还没有键位方案"
        description="新建一个方案开始绑定按键。"
      >
        <template #icon><Keyboard :size="20" /></template>
        <template #actions>
          <Button variant="primary" @click="startNew">{{ tr("新建方案") }}</Button>
        </template>
      </EmptyState>

      <div v-else class="empty-workspace">
        <EmptyState title="选择一个方案开始编辑" description="也可以新建方案，或从模板快速创建。">
          <template #icon><Keyboard :size="20" /></template>
          <template #actions>
            <Button variant="primary" @click="startNew">{{ tr("新建方案") }}</Button>
          </template>
        </EmptyState>
      </div>
    </template>

    <ProfileSettingsDrawer
      v-if="editor"
      v-model:draft="editor"
      :open="settingsOpen"
      :busy="busy"
      :foreground-app="foregroundApp"
      @close="settingsOpen = false"
      @error="onEditorError"
    />
  </section>
</template>

<style scoped>
.buttons-page {
  gap: var(--space-3);
}

.view-switch {
  display: inline-flex;
  gap: 3px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 3px;
}

.view-button {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  padding: 6px 9px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 650;
  cursor: pointer;
}

.view-button:hover {
  color: var(--text);
}
.view-button.active {
  background: var(--text);
  color: var(--bg);
  box-shadow: 0 2px 7px rgba(15, 30, 25, 0.16);
}

.profile-bar {
  display: flex;
  flex-wrap: nowrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.profile-chips {
  display: flex;
  flex: 1 1 auto;
  flex-wrap: nowrap;
  align-items: center;
  gap: 4px;
  min-width: 0;
  overflow-x: auto;
  overflow-y: hidden;
  padding: 3px;
  border: 1px solid var(--border);
  border-radius: 10px;
  background: var(--bg-subtle);
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.profile-chips::-webkit-scrollbar {
  display: none;
}

.profile-chip {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 8px;
  max-width: 245px;
  border: 1px solid transparent;
  border-radius: 7px;
  background: transparent;
  padding: 7px 14px;
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.profile-chip:hover {
  border-color: var(--border-strong);
  background: var(--bg-raised);
}
.profile-chip.active {
  background: var(--bg-raised);
}
.profile-chip.selected {
  border-color: var(--accent);
  background: var(--bg-raised);
  box-shadow: 0 1px 2px rgba(0, 0, 0, 0.06);
}
.profile-chip.selected .profile-chip-copy strong {
  color: var(--accent-ink);
}

.profile-chip-index {
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

.profile-chip.selected .profile-chip-index {
  background: var(--accent);
  color: var(--text-on-accent);
}

.profile-chip.new-chip {
  cursor: default;
}

/* Equal specificity to `.profile-chip.selected .profile-chip-index` but later,
   so the "+" badge keeps its accent-soft treatment on the selected new chip. */
.profile-chip.new-chip .profile-chip-index {
  background: var(--accent-soft);
  color: var(--accent);
  font-weight: 600;
}

.profile-chip-flag {
  border-radius: 4px;
  background: var(--info-soft, color-mix(in srgb, var(--info) 14%, transparent));
  padding: 1px 5px;
  color: var(--info-text);
  font-size: 9px;
  line-height: 1.4;
}

.profile-chip-copy {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.profile-chip-copy strong {
  font-size: var(--text-xs);
  font-weight: 500;
}
.profile-chip.selected .profile-chip-copy strong {
  font-weight: 600;
}

.profile-tools {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: var(--space-2);
  /* Keep the actions on the right edge even when the chip strip is hidden. */
  margin-left: auto;
}

.editor-toolbar {
  display: flex;
  /* Single row at every supported width: the context block shrinks and
     truncates instead of pushing the action buttons onto a second line. */
  flex-wrap: nowrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) 0;
}

.editor-context {
  display: flex;
  flex: 1 1 auto;
  flex-wrap: nowrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  overflow: hidden;
}

.editor-context .eyebrow {
  flex: 0 0 auto;
}

.editor-name {
  flex: 0 1 auto;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.editor-context .eyebrow {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.editor-name {
  font-size: var(--text-lg);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.activation-control {
  display: inline-flex;
  align-items: center;
  gap: 6px;
}

.activation-state {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.activation-state.on {
  color: var(--accent-ink);
  font-weight: 600;
}
.editor-target {
  max-width: 230px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
}

.editor-toolbar-actions {
  display: flex;
  flex: 0 0 auto;
  /* Cap the row at its intrinsic width so the context block keeps the rest. */
  width: max-content;
  max-width: 100%;
  flex-wrap: nowrap;
  align-items: center;
  gap: var(--space-2);
}

.empty-workspace {
  padding-top: var(--space-2);
}

@media (max-width: 1100px) {
  .profile-bar {
    align-items: flex-start;
  }
  .profile-chip {
    max-width: none;
    flex: 0 0 auto;
  }
  .profile-tools {
    justify-content: flex-end;
  }
  .editor-toolbar-actions {
    width: 100%;
  }
  .editor-target {
    max-width: 100%;
  }
}
</style>
