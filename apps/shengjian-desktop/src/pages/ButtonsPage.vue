<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { open, save } from "@tauri-apps/plugin-dialog";
import { Copy, FileDown, FileUp, Info, Keyboard, Pencil, Plus, Trash2 } from "lucide-vue-next";
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
import Card from "../components/ui/Card.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import IconButton from "../components/ui/IconButton.vue";
import Notice from "../components/ui/Notice.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import SelectInput from "../components/ui/SelectInput.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import ProfileEditor from "../components/ProfileEditor.vue";
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

const PROFILE_TEMPLATES = [
  {
    id: "notepad",
    name: "Notepad 聚焦输入",
    targets: ["notepad.exe"],
  },
  {
    id: "chrome",
    name: "Chrome 聚焦输入",
    targets: ["chrome.exe"],
  },
  {
    id: "netease",
    name: "网易云音乐聚焦",
    targets: ["cloudmusic.exe"],
  },
] as const;

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
const expandedId = ref<string | null>(null);
const copiedId = ref<string | null>(null);
const templateKey = ref(0);
const editorHost = ref<HTMLElement | null>(null);
const baselineJson = ref("");
let timer: number | undefined;
let copyTimer: number | undefined;

const { dirty, confirmDiscard } = useDraftGuard({
  draft: editor,
  baseline: baselineJson,
  discardMessage: "当前方案的修改尚未保存，继续将丢失这些修改。",
});

const currentProfileLabel = computed(() => {
  const id = activeProfileId.value;
  if (!id) return tr("未激活");
  return (
    profilesDocument.value?.profiles.find((profile) => profile.id === id)?.name ?? tr("未激活")
  );
});

const libraryHint = computed(() =>
  tr("{count} 个方案 · 当前 {id}", {
    count: profilesDocument.value?.profiles.length ?? 0,
    id: currentProfileLabel.value,
  }),
);

function isActiveProfile(profileId: string): boolean {
  return activeProfileId.value === profileId;
}

function bindingCount(profile: ButtonProfile): number {
  return Object.values(profile.bindings ?? {}).reduce(
    (total, triggers) => total + Object.keys(triggers ?? {}).length,
    0,
  );
}

async function load() {
  try {
    const state = await getButtonProfileState();
    profilesDocument.value = state.document;
    activeProfileId.value = state.activeProfileId;
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

async function createFromTemplate(templateId: string) {
  const template = PROFILE_TEMPLATES.find((item) => item.id === templateId);
  if (!template) return;
  if (!(await confirmDiscard())) return;
  const bindings: ProfileDraft["bindings"] = {
    ok: { long: { type: "focus-input" } },
  };
  editingNew.value = true;
  editor.value = {
    id: `profile-${crypto.randomUUID()}`,
    name: trDynamic(template.name),
    appTargets: template.targets.join(", "),
    autoApply: true,
    holdRepeat: "",
    voiceInputTool: "",
    bindings,
  };
  syncBaseline();
}

async function selectTemplate(templateId: string) {
  templateKey.value += 1;
  if (!templateId) return;
  await createFromTemplate(templateId);
}

async function editProfile(profile: ButtonProfile) {
  if (editor.value?.id === profile.id) return;
  if (!(await confirmDiscard())) return;
  editingNew.value = false;
  editor.value = {
    id: profile.id,
    name: profile.name,
    appTargets: profile.appTargets.join(", "),
    autoApply: profile.autoApply,
    holdRepeat: profile.holdRepeat ?? "",
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
      voiceInputTool: editor.value.voiceInputTool || null,
      bindings: cleanBindings(),
    };
    const profiles = profilesDocument.value.profiles.some((item) => item.id === profile.id)
      ? profilesDocument.value.profiles.map((item) => (item.id === profile.id ? profile : item))
      : [...profilesDocument.value.profiles, profile];
    await saveButtonProfiles({ ...profilesDocument.value, profiles });
    closeEditor();
    await load();
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
    if (expandedId.value === profile.id) expandedId.value = null;
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

async function copyId(profileId: string) {
  try {
    await navigator.clipboard.writeText(profileId);
    copiedId.value = profileId;
    if (copyTimer !== undefined) window.clearTimeout(copyTimer);
    copyTimer = window.setTimeout(() => {
      copiedId.value = null;
    }, 1500);
  } catch {
    error.value = tr("无法复制 ID，请手动选择文本。");
  }
}

function toggleDetails(profileId: string) {
  expandedId.value = expandedId.value === profileId ? null : profileId;
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
  await Promise.all([load(), loadMacros(), loadHostCapabilities(), loadForeground()]);
  timer = window.setInterval(() => {
    if (window.document.visibilityState === "visible") void load();
  }, 3000);
});

onUnmounted(() => {
  if (timer !== undefined) window.clearInterval(timer);
  if (copyTimer !== undefined) window.clearTimeout(copyTimer);
});
</script>

<template>
  <section class="page">
    <PageHeader
      eyebrow="Buttons"
      title="按键与方案"
      description="为遥控器按键绑定动作或宏，并按前台应用自动切换方案。"
    />

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

    <div class="split-layout">
      <Card class="stack stack-sm list-card">
        <SectionHeader title="键位方案" :hint="libraryHint" />
        <div class="library-tools">
          <SelectInput
            :key="templateKey"
            class="template-select"
            :model-value="''"
            :disabled="busy"
            :aria-label="trDynamic('从模板新建…')"
            @update:model-value="selectTemplate"
          >
            <option value="">{{ tr("从模板新建…") }}</option>
            <option v-for="template in PROFILE_TEMPLATES" :key="template.id" :value="template.id">
              {{ trDynamic(template.name) }}
            </option>
          </SelectInput>
          <div class="toolbar-actions">
            <Button size="sm" :disabled="busy" @click="importFile">
              <FileUp :size="14" />
              {{ tr("导入") }}
            </Button>
            <Button size="sm" :disabled="busy || !profilesDocument" @click="exportFile">
              <FileDown :size="14" />
              {{ tr("导出") }}
            </Button>
            <Button size="sm" variant="primary" :disabled="busy" @click="startNew">
              <Plus :size="14" />
              {{ tr("新建") }}
            </Button>
          </div>
        </div>

        <LoadingState v-if="loading" label="正在加载…" description="正在读取本地键位方案…" />

        <EmptyState
          v-else-if="profilesDocument && profilesDocument.profiles.length === 0"
          title="还没有键位方案"
          description="新建一个方案开始绑定按键。"
        >
          <template #icon><Keyboard :size="20" /></template>
          <template #actions>
            <Button variant="primary" @click="startNew">{{ tr("新建方案") }}</Button>
          </template>
        </EmptyState>

        <ul v-else-if="profilesDocument" class="profile-list">
          <li
            v-for="profile in profilesDocument.profiles"
            :key="profile.id"
            class="profile-row"
            :class="{ active: isActiveProfile(profile.id) }"
          >
            <span class="profile-icon"><Keyboard :size="16" /></span>
            <span class="profile-main">
              <span class="profile-name-row">
                <span class="profile-name text-truncate" :title="profile.name">{{
                  profile.name
                }}</span>
                <StatusPill
                  v-if="isActiveProfile(profile.id)"
                  :label="trDynamic('已激活')"
                  tone="accent"
                  dot
                />
                <StatusPill
                  :label="statusLabel(profile.autoApply ? 'auto' : 'manual')"
                  :tone="profile.autoApply ? 'info' : 'neutral'"
                />
              </span>
              <span class="profile-targets text-truncate" :title="profile.appTargets.join(', ')">
                {{
                  profile.appTargets.length ? profile.appTargets.join(", ") : tr("未设置应用目标")
                }}
              </span>
            </span>
            <span class="profile-actions">
              <Button
                size="sm"
                :disabled="busy || isActiveProfile(profile.id)"
                @click="activate(profile)"
              >
                {{ trDynamic(isActiveProfile(profile.id) ? "已激活" : "激活") }}
              </Button>
              <IconButton
                size="sm"
                :label="trDynamic('复制方案')"
                :disabled="busy"
                @click="duplicateProfile(profile)"
              >
                <Copy :size="14" />
              </IconButton>
              <IconButton size="sm" :label="trDynamic('编辑方案')" @click="editProfile(profile)">
                <Pencil :size="14" />
              </IconButton>
              <IconButton
                size="sm"
                variant="danger"
                :label="trDynamic('删除方案')"
                :disabled="busy"
                @click="removeProfile(profile)"
              >
                <Trash2 :size="14" />
              </IconButton>
              <IconButton
                size="sm"
                :label="trDynamic('详情')"
                :aria-expanded="expandedId === profile.id"
                @click="toggleDetails(profile.id)"
              >
                <Info :size="14" />
              </IconButton>
            </span>
            <div v-if="expandedId === profile.id" class="profile-details">
              <span class="muted">{{ tr("标识") }}</span>
              <code class="mono text-truncate" :title="profile.id">{{ profile.id }}</code>
              <Button size="sm" variant="ghost" @click="copyId(profile.id)">
                {{ trDynamic(copiedId === profile.id ? "已复制" : "复制 ID") }}
              </Button>
              <span class="muted">
                {{ tr("{count} 个绑定", { count: bindingCount(profile) }) }}
              </span>
            </div>
          </li>
        </ul>

        <div v-if="profilesDocument?.activeManualProfileId" class="footer-actions">
          <Button size="sm" :disabled="busy" @click="clearActivation">
            {{ tr("清除手动激活") }}
          </Button>
        </div>
      </Card>

      <div ref="editorHost" class="editor-host">
        <ProfileEditor
          v-if="editor"
          v-model:draft="editor"
          :editing-new="editingNew"
          :busy="busy"
          :dirty="dirty"
          :baseline-json="baselineJson"
          :foreground-app="foregroundApp"
          :common-phrases-available="commonPhrasesAvailable"
          :latest-macros="latestMacros"
          @save="saveEditor"
          @cancel="cancelEdit"
          @error="onEditorError"
        />

        <Card v-else class="editor-placeholder">
          <EmptyState title="从左侧选择一个方案开始编辑" description="新建方案，或从模板快速创建。">
            <template #icon><Keyboard :size="20" /></template>
            <template #actions>
              <Button variant="primary" @click="startNew">{{ tr("新建方案") }}</Button>
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

.template-select {
  width: 100%;
}

.profile-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.profile-row {
  display: grid;
  grid-template-columns: 32px minmax(0, 1fr);
  gap: var(--space-2) var(--space-3);
  align-items: start;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.profile-row:hover {
  background: var(--bg-subtle);
}

.profile-row.active {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.profile-icon {
  display: grid;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  background: var(--bg-hover);
  color: var(--accent);
}

.profile-main {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.profile-name-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.profile-name {
  font-weight: 600;
}

.profile-targets {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.profile-actions {
  display: flex;
  flex-wrap: wrap;
  grid-column: 1 / -1;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-1);
}

.profile-details {
  display: flex;
  flex-wrap: wrap;
  grid-column: 1 / -1;
  align-items: center;
  gap: var(--space-2);
  border-top: 1px dashed var(--border);
  padding-top: var(--space-2);
  font-size: var(--text-xs);
}

.profile-details code {
  flex: 1 1 120px;
  min-width: 0;
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
