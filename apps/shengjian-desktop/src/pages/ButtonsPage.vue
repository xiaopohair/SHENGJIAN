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
  Copy,
  Keyboard,
  LayoutGrid,
  MoreVertical,
  Plus,
  RotateCcw,
  SlidersHorizontal,
  Sparkles,
  Table2,
  Trash2,
} from "lucide-vue-next";
import { latestMacroDefinitions, mergeRecommendedMacros } from "../lib/macros";
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
import { pushToast } from "../lib/toast";
import { mergeAppTargetsText, splitAppTargets } from "../lib/appTargets";
import { supportsKeyEnhancement } from "../lib/keyEnhancement";
import { useAppPicker } from "../composables/useAppPicker";
import AppPickerDialog from "../components/app-picker/AppPickerDialog.vue";
import {
  hasRecommendedUpdate,
  recommendedSeenVersion,
  rememberRecommendedVersion,
} from "../lib/recommended";
import { useDraftGuard } from "../composables/useDraftGuard";
import { useEngineStore } from "../stores/engine";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import IconButton from "../components/ui/IconButton.vue";
import Notice from "../components/ui/Notice.vue";
import ProfileWorkbench from "../components/ProfileWorkbench.vue";
import ProfileSettingsDrawer from "../components/ProfileSettingsDrawer.vue";
import ProfileTransferDialog from "../components/ProfileTransferDialog.vue";
import RecommendedProfilesDialog from "../components/RecommendedProfilesDialog.vue";
import {
  activateButtonProfile,
  clearButtonProfileActivation,
  copyRecommendedButtonProfile,
  errorMessage,
  getButtonProfileState,
  getForegroundExecutable,
  getHostCapabilities,
  getMacroLibrary,
  getRecommendedButtonProfiles,
  getSettings,
  getRc003HelperInstalled,
  installRc003Helper,
  getRecommendedMacros,
  saveButtonProfiles,
  saveSettings,
} from "../lib/api";
import type {
  ButtonProfile,
  ButtonProfilesDocument,
  LogicalAction,
  MacroDefinition,
  RecommendedButtonProfile,
  SettingsDocument,
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
/** 配置目录下发的只读推荐宏；只用于「运行其他宏」下拉，不进入宏页。 */
const recommendedMacroDefinitions = ref<MacroDefinition[]>([]);
const commonPhrasesAvailable = shallowRef(false);
const foregroundApp = shallowRef("");
const latestMacros = computed(() =>
  latestMacroDefinitions(
    mergeRecommendedMacros(macroDefinitions.value, recommendedMacroDefinitions.value),
  ),
);

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

/** 推荐方案：只读列表 + 「复制并使用」；「已查看版本」只存 localStorage。 */
const recommendedOpen = shallowRef(false);
const recommendedBundleVersion = shallowRef("");
const recommendedSeen = shallowRef(recommendedSeenVersion());
const recommendedHasUpdate = computed(() =>
  hasRecommendedUpdate(recommendedBundleVersion.value, recommendedSeen.value),
);

function openRecommendedProfiles() {
  recommendedOpen.value = true;
}

async function loadRecommendedVersion() {
  try {
    const bundle = await getRecommendedButtonProfiles(effectiveModel.value);
    recommendedBundleVersion.value = bundle.bundleVersion ?? "";
  } catch {
    // The badge is only a hint; the dialog owns the visible load error.
    recommendedBundleVersion.value = "";
  }
}

/** 打开/查看过推荐列表后记下版本，工具栏的「有更新」徽标随之消失。 */
function onRecommendedLoaded(version: string) {
  recommendedBundleVersion.value = version;
  recommendedSeen.value = version;
  rememberRecommendedVersion(version);
}

/**
 * 推荐方案 →「复制并使用」：复制成用户方案并立即激活、进入编辑。
 * 推荐源只读，所有写入都落在 copy 上。
 */
async function useRecommendedProfile(recommended: RecommendedButtonProfile) {
  if (busy.value || !profilesDocument.value) return;
  // The centered dialog covers ConfirmDialog (z-index), so ask after closing it.
  if (dirty.value) {
    recommendedOpen.value = false;
    if (!(await confirmDiscard())) {
      recommendedOpen.value = true;
      return;
    }
  }
  busy.value = true;
  error.value = null;
  try {
    const copy = await copyRecommendedButtonProfile(recommended.id, effectiveModel.value);
    const document = profilesDocument.value;
    if (!document) return;
    const profiles = document.profiles.some((profile) => profile.id === copy.id)
      ? document.profiles.map((profile) => (profile.id === copy.id ? copy : profile))
      : [...document.profiles, copy];
    await saveButtonProfiles({ ...document, profiles });
    await load();
    if (!(await activateButtonProfile(copy.id))) {
      error.value = tr("未找到该方案，可能已被删除。");
    }
    await load();
    const created =
      profilesDocument.value?.profiles.find((profile) => profile.id === copy.id) ?? copy;
    openDraft(created);
    recommendedOpen.value = false;
  } catch (cause) {
    // Close the centered dialog so the page-level error notice is visible.
    recommendedOpen.value = false;
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

watch(effectiveModel, () => {
  void loadRecommendedVersion();
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
  try {
    const macros = await getRecommendedMacros();
    recommendedMacroDefinitions.value = Array.isArray(macros) ? macros : [];
  } catch {
    // 推荐宏只是补充：命令缺失/加载失败时下拉仍显示用户宏库，不影响编辑。
    recommendedMacroDefinitions.value = [];
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
  baselineJson.value = "";
}

/**
 * 「添加」：先让用户选应用，选完才创建方案并进入编辑。
 *
 * 方案的意义就是「匹配某个前台应用时切到这套键位」，没有应用目标的新方案
 * 只能靠手动激活，绝大多数用户并不需要——所以把选应用提到创建之前。
 * 取消选择 = 不创建。
 */
async function startNew() {
  if (!(await confirmDiscard())) return;
  if (!profilesDocument.value) return;
  openPickerFor("new-profile");
}

/** 用选中的应用目标创建方案并进入编辑（由应用选择器回调触发）。 */
async function createProfileWithTarget(target: string) {
  if (!profilesDocument.value) return;
  busy.value = true;
  error.value = null;
  try {
    const created: ButtonProfile = {
      id: `profile-${crypto.randomUUID()}`,
      name: tr("新方案"),
      appTargets: [target],
      autoApply: false,
      holdRepeat: null,
      holdRepeatByButton: {},
      voiceInputTool: null,
      // 新方案归当前型号所有，这样它与别的型号互不干扰。
      deviceModel: effectiveModel.value ?? null,
      isDefault: false,
      bindings: {},
    };
    await saveButtonProfiles({
      ...profilesDocument.value,
      profiles: [...profilesDocument.value.profiles, created],
    });
    await load();
    const saved = profilesDocument.value?.profiles.find((item) => item.id === created.id);
    if (saved) {
      openDraft(saved);
      justSaved.value = true;
      if (savedTimer !== undefined) window.clearTimeout(savedTimer);
      savedTimer = window.setTimeout(() => {
        justSaved.value = false;
      }, 2200);
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
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
  // 方案存在的意义就是「匹配某个程序时生效」；没有程序的普通方案无法自动生效，
  // 也不符合「新建必须先选程序」的流程，所以保存时直接拦住。默认方案是基线，豁免。
  if (!editor.value.isDefault && editor.value.appTargets.trim() === "") {
    return tr("请先为方案选择一个程序");
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
      if (action.type === "system-action" && !String(action.key ?? "").trim()) {
        return tr("{position} 缺少系统动作", { position });
      }
    }
  }
  return null;
}

function onEditorError(message: string) {
  error.value = message;
}

/** 把当前编辑草稿转成方案对象（保存与「导出为推荐方案」共用同一份构造逻辑）。 */
function draftToProfile(): ButtonProfile | null {
  if (!editor.value) return null;
  return {
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
    const profile = draftToProfile();
    if (!profile) return;
    const profiles = profilesDocument.value.profiles.some((item) => item.id === profile.id)
      ? profilesDocument.value.profiles.map((item) => (item.id === profile.id ? profile : item))
      : [...profilesDocument.value.profiles, profile];
    await saveButtonProfiles({ ...profilesDocument.value, profiles });
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

/** 导入与导出合并成一个弹窗：选方案 → 导出一个文件或每个方案一个文件。 */
const transferOpen = shallowRef(false);

/** 「更多」菜单（竖向三点）：导入 / 导出等低频操作收在这里。 */
const moreMenuOpen = shallowRef(false);
const moreMenuRef = useTemplateRef<HTMLElement>("moreMenuRef");

function toggleMoreMenu() {
  moreMenuOpen.value = !moreMenuOpen.value;
}

function openTransfer() {
  moreMenuOpen.value = false;
  transferOpen.value = true;
}

/** 已保存的方案才能重置/复制；新建草稿或未保存时这两项不可用。 */
const canEditCurrentProfile = computed(() => Boolean(currentProfileRecord.value));

/** 按键增强（RC003 helper 的增强捕获）来自全局设置，随页面加载并立即保存。 */
const enhancedCapture = shallowRef(false);
const enhancedCaptureInstalled = shallowRef(false);
const settingsDocument = shallowRef<SettingsDocument | null>(null);
/** 三款小米遥控器都支持按键增强，型号未知时不支持。 */
const supportsEnhancement = computed(() => supportsKeyEnhancement(remoteModel.value));

async function loadSettingsDocument() {
  try {
    const [settings, helperInstalled] = await Promise.all([
      getSettings(),
      getRc003HelperInstalled(),
    ]);
    settingsDocument.value = settings;
    enhancedCapture.value = settings.global.rc003_capture_enabled ?? false;
    enhancedCaptureInstalled.value = helperInstalled;
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

/** 等服务真的出现：UAC 由用户手动确认，装完计划任务才会出现。 */
async function waitForHelperInstalled(timeoutMs = 60_000): Promise<boolean> {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await getRc003HelperInstalled().catch(() => false)) return true;
    await new Promise((resolve) => window.setTimeout(resolve, 1000));
  }
  return false;
}

/**
 * 按键增强开关。
 *
 * 打开时如果服务没装，先走一次管理员安装（会弹 UAC）：装完再落盘设置，
 * 用户取消 UAC 就保持关闭并提示。关闭只撤销增强捕获、不卸载服务——
 * 卸载同样需要管理员权限，而关掉后 helper 不接管任何按键，没必要折腾。
 */
async function toggleEnhancedCapture(value: boolean) {
  if (busy.value) return;
  if (value && !enhancedCaptureInstalled.value) {
    busy.value = true;
    pushToast(tr("开启按键增强需要管理员授权，请在弹窗中确认"), "info");
    try {
      await installRc003Helper();
      const installed = await waitForHelperInstalled();
      if (!installed) {
        pushToast(tr("没有完成按键增强服务安装，开关已保持关闭"), "warning");
        return;
      }
      enhancedCaptureInstalled.value = true;
      pushToast(tr("按键增强服务已安装"), "success");
    } catch (cause) {
      error.value = errorMessage(cause);
      return;
    } finally {
      busy.value = false;
    }
  }

  const previous = enhancedCapture.value;
  enhancedCapture.value = value;
  const current = settingsDocument.value;
  if (!current) return;
  try {
    const next: SettingsDocument = {
      ...current,
      global: { ...current.global, rc003_capture_enabled: value },
    };
    await saveSettings(next);
    settingsDocument.value = next;
    if (!value) pushToast(tr("已关闭按键增强，服务保留可随时再开启"), "info");
  } catch (cause) {
    enhancedCapture.value = previous;
    error.value = errorMessage(cause);
  }
}

/**
 * 应用选择器：一个对话框服务两种用途。
 *
 * - `auto-apply`：开「自动应用」时方案还没有应用目标 → 选完写进方案并开启自动应用；
 * - `new-profile`：点「添加」→ 必须先选程序，选完才创建方案。
 *
 * 两个用途的标题、recent targets、选中后的行为都不同，所以用 purpose 区分，
 * 而不是开两个对话框状态。
 */
const {
  open: appPickerOpen,
  openPicker: openAppPicker,
  closePicker: closeAppPicker,
} = useAppPicker();
const appPickerPurpose = shallowRef<"auto-apply" | "new-profile" | null>(null);

const appPickerTitle = computed(() =>
  appPickerPurpose.value === "new-profile"
    ? tr("选择这个方案要匹配的应用")
    : tr("选择用于自动匹配的应用"),
);
const appPickerDescription = computed(() =>
  appPickerPurpose.value === "new-profile"
    ? tr("先选一个程序：方案的键位会在它切到前台时生效。")
    : tr("选择用于自动匹配的应用（按可执行文件名匹配前台）"),
);
const appPickerRecentTargets = computed(() => splitAppTargets(editor.value?.appTargets ?? ""));

function openPickerFor(purpose: "auto-apply" | "new-profile") {
  appPickerPurpose.value = purpose;
  openAppPicker();
}

function closeAppPickerAndReset() {
  appPickerPurpose.value = null;
  closeAppPicker();
}

async function onAppPicked(target: string) {
  const purpose = appPickerPurpose.value;
  appPickerPurpose.value = null;
  closeAppPicker();
  if (purpose === "new-profile") {
    await createProfileWithTarget(target);
    return;
  }
  if (editor.value) {
    editor.value.appTargets = mergeAppTargetsText(editor.value.appTargets, target);
    if (purpose === "auto-apply") editor.value.autoApply = true;
  }
}

function toggleAutoApply(value: boolean) {
  if (!editor.value || editor.value.isDefault) return;
  if (!value) {
    editor.value.autoApply = false;
    return;
  }
  if (splitAppTargets(editor.value.appTargets).length > 0) {
    editor.value.autoApply = true;
    return;
  }
  // 没配应用：先让用户选一个，选完再开启。
  openPickerFor("auto-apply");
}
const canDeleteCurrentProfile = computed(
  () =>
    Boolean(currentProfileRecord.value) &&
    !isDefaultProfile(currentProfileRecord.value ?? { isDefault: false }),
);

/** 菜单项动作：先收起菜单再执行，避免菜单盖在确认框上。 */
function runFromMoreMenu(action: () => void | Promise<void>) {
  moreMenuOpen.value = false;
  void action();
}

/** 点菜单外或按 Esc 收起，避免菜单挂在那里挡住工具栏。 */
function onMoreMenuPointerDown(event: MouseEvent) {
  if (!moreMenuOpen.value) return;
  const target = event.target as Node | null;
  if (target && moreMenuRef.value?.contains(target)) return;
  moreMenuOpen.value = false;
}

function onMoreMenuKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && moreMenuOpen.value) {
    event.preventDefault();
    moreMenuOpen.value = false;
  }
}
/**
 * 弹窗自己完成文件读写；页面只负责把新状态读回来。
 *
 * 导入是合并而不是覆盖，正在编辑的草稿依然指向一条存在的方案，
 * 所以这里不关编辑器（关掉等于静默丢掉用户没保存的修改）。
 */
async function onTransferApplied() {
  await load();
  if (!editor.value) syncBaseline();
  pushToast(tr("方案列表已更新"), "success");
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
  window.addEventListener("pointerdown", onMoreMenuPointerDown, true);
  window.addEventListener("keydown", onMoreMenuKeydown, true);
  await Promise.all([
    load(),
    loadMacros(),
    loadHostCapabilities(),
    loadForeground(),
    loadRecommendedVersion(),
    loadSettingsDocument(),
  ]);
  timer = window.setInterval(() => {
    if (window.document.visibilityState === "visible") void load();
  }, 3000);
});

onUnmounted(() => {
  window.removeEventListener("pointerdown", onMoreMenuPointerDown, true);
  window.removeEventListener("keydown", onMoreMenuKeydown, true);
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
        <div v-if="profilesDocument.profiles.length" class="profile-chips" @wheel="onProfileWheel">
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
          <Button
            size="sm"
            variant="primary"
            :disabled="busy || !editor || !dirty"
            @click="saveEditor"
          >
            {{ tr("保存") }}
          </Button>
          <Button size="sm" :disabled="busy" @click="startNew">
            <Plus :size="14" />
            {{ tr("添加") }}
          </Button>
          <div ref="moreMenuRef" class="profile-more">
            <IconButton
              size="sm"
              :label="trDynamic('更多操作')"
              :aria-expanded="moreMenuOpen"
              aria-haspopup="menu"
              @click="toggleMoreMenu"
            >
              <MoreVertical :size="15" />
            </IconButton>
            <div v-if="moreMenuOpen" class="profile-more-menu" role="menu">
              <button type="button" class="profile-more-item" role="menuitem" @click="openTransfer">
                <ArrowLeftRight :size="14" />
                {{ tr("导入 / 导出") }}
              </button>
              <div class="profile-more-divider" role="separator" />
              <button
                type="button"
                class="profile-more-item"
                role="menuitem"
                :disabled="busy || !canEditCurrentProfile"
                @click="runFromMoreMenu(resetCurrentProfile)"
              >
                <RotateCcw :size="14" />
                {{ tr("重置方案") }}
              </button>
              <button
                type="button"
                class="profile-more-item"
                role="menuitem"
                :disabled="busy || !canEditCurrentProfile"
                @click="runFromMoreMenu(duplicateCurrentProfile)"
              >
                <Copy :size="14" />
                {{ tr("复制方案") }}
              </button>
              <template v-if="canDeleteCurrentProfile">
                <div class="profile-more-divider" role="separator" />
                <button
                  type="button"
                  class="profile-more-item danger"
                  role="menuitem"
                  :disabled="busy"
                  @click="runFromMoreMenu(removeCurrentProfile)"
                >
                  <Trash2 :size="14" />
                  {{ tr("删除方案") }}
                </button>
              </template>
            </div>
          </div>
          <IconButton
            size="sm"
            :label="trDynamic('属性')"
            :disabled="busy || !editor"
            @click="openSettings"
          >
            <SlidersHorizontal :size="15" />
          </IconButton>
          <Button size="sm" :disabled="busy" @click="openRecommendedProfiles">
            <Sparkles :size="14" />
            {{ tr("推荐方案") }}
            <span
              v-if="recommendedHasUpdate"
              class="recommended-update-badge"
              :title="tr('推荐方案有更新')"
            >
              {{ tr("有更新") }}
            </span>
          </Button>
        </div>
      </div>

      <AppPickerDialog
        :open="appPickerOpen"
        mode="match"
        :title="appPickerTitle"
        :description="appPickerDescription"
        :foreground-app="foregroundApp"
        :recent-targets="appPickerRecentTargets"
        @select="onAppPicked"
        @close="closeAppPickerAndReset"
      />

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
        :enhanced-capture="enhancedCapture"
        :enhanced-capture-enabled="supportsEnhancement"
        :enhanced-capture-installed="enhancedCaptureInstalled"
        :auto-apply="editor.autoApply"
        :auto-apply-enabled="!editor.isDefault"
        :activation-active="isActiveProfile(editor.id)"
        :activation-enabled="!editor.isDefault"
        :toggle-disabled="busy || !currentProfileRecord"
        @toggle-enhanced-capture="toggleEnhancedCapture"
        @toggle-auto-apply="toggleAutoApply"
        @toggle-activation="toggleEditorActivation"
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

    <ProfileTransferDialog
      :open="transferOpen"
      :document="profilesDocument"
      :model="effectiveModel"
      :busy="busy"
      @close="transferOpen = false"
      @applied="onTransferApplied"
    />

    <RecommendedProfilesDialog
      :open="recommendedOpen"
      :model="effectiveModel"
      :busy="busy"
      @use="useRecommendedProfile"
      @close="recommendedOpen = false"
      @loaded="onRecommendedLoaded"
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

/* 竖向三点菜单：定位在按钮下方，靠右对齐，浮在工具栏之上。 */
.profile-more {
  position: relative;
  display: flex;
}

.profile-more-menu {
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

.profile-more-item {
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

.profile-more-item:hover:not(:disabled) {
  background: var(--bg-hover);
  color: var(--text);
}

.profile-more-item:disabled {
  color: var(--text-disabled);
  cursor: default;
}

.profile-more-item.danger {
  color: var(--danger-text);
}

.profile-more-item.danger:hover:not(:disabled) {
  background: var(--danger-soft);
}

.profile-more-divider {
  margin: var(--space-1) 0;
  border-top: 1px solid var(--border);
}

.recommended-update-badge {
  border-radius: var(--radius-pill);
  background: var(--accent-soft-strong);
  padding: 1px 6px;
  color: var(--accent-ink);
  font-size: var(--text-2xs);
  font-weight: 700;
  white-space: nowrap;
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
}
</style>
