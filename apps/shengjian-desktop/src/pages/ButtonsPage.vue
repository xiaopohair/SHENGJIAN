<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import {
  Copy,
  FileDown,
  FileUp,
  Keyboard,
  LayoutGrid,
  Plus,
  SlidersHorizontal,
  Table2,
  Trash2,
} from "lucide-vue-next";
import { latestMacroDefinitions } from "../lib/macros";
import {
  PROFILE_BUTTONS,
  PROFILE_TRIGGERS,
  type ProfileDraft,
  type ProfileTrigger,
} from "../lib/buttons";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { confirmAction } from "../lib/confirm";
import { useDraftGuard } from "../composables/useDraftGuard";
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
const activeProfileId = ref<string | null>(null);
const macroDefinitions = ref<MacroDefinition[]>([]);
const commonPhrasesAvailable = ref(false);
const foregroundApp = ref("");
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
const busy = ref(false);
const loading = ref(true);
const error = ref<string | null>(null);
const editor = ref<ProfileDraft | null>(null);
const editingNew = ref(false);
const baselineJson = ref("");
const view = ref<"workbench" | "table">("workbench");
const settingsOpen = ref(false);
const justSaved = ref(false);
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
    if (!editor.value && !autoOpened && state.document.profiles.length > 0) {
      autoOpened = true;
      const initial =
        state.document.profiles.find((profile) => profile.id === state.activeProfileId) ??
        state.document.profiles[0];
      openDraft(initial);
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
            v-for="(profile, index) in profilesDocument.profiles"
            :key="profile.id"
            type="button"
            class="profile-chip"
            :class="{
              active: activeProfileId === profile.id,
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
              v-if="activeProfileId === profile.id"
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
            :label="editor.autoApply ? tr('自动应用') : tr('手动')"
            :tone="editor.autoApply ? 'info' : 'neutral'"
            :title="
              editor.autoApply
                ? tr('前台应用匹配时自动切换到此方案')
                : tr('不自动切换，需要手动激活')
            "
          />
          <span v-if="!editingNew" class="activation-control">
            <Switch
              :model-value="activeProfileId === editor.id"
              :disabled="busy || !currentProfileRecord"
              :label="tr('激活')"
              @update:model-value="toggleEditorActivation"
            />
            <span class="activation-state" :class="{ on: activeProfileId === editor.id }">
              {{ activeProfileId === editor.id ? tr("已激活") : tr("未激活") }}
            </span>
          </span>
          <StatusPill v-if="justSaved" :label="trDynamic('已保存')" tone="accent" dot />
          <StatusPill v-if="dirty" :label="trDynamic('未保存')" tone="warning" dot />
        </div>
        <div class="editor-toolbar-actions">
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
            v-if="!editingNew && currentProfileRecord"
            size="sm"
            variant="danger"
            :label="trDynamic('删除方案')"
            :disabled="busy"
            @click="removeCurrentProfile"
          >
            <Trash2 :size="14" />
          </IconButton>
          <Button size="sm" :disabled="busy" @click="cancelEdit">{{ tr("取消") }}</Button>
          <Button
            size="sm"
            variant="primary"
            :disabled="busy || (!dirty && !editingNew)"
            @click="saveEditor"
          >
            {{ tr("保存") }}
          </Button>
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
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) 0;
}

.editor-context {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
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
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.empty-workspace {
  padding-top: var(--space-2);
}

@media (max-width: 1260px) {
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
