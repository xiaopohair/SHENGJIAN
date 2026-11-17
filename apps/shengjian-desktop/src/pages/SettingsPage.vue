<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Component } from "vue";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  ChevronDown,
  ChevronUp,
  Download,
  FolderOpen,
  HardDrive,
  MessageSquare,
  Palette,
  Power,
  Keyboard,
  RotateCcw,
  Search,
  Upload,
  Volume2,
  Wrench,
} from "lucide-vue-next";
import {
  errorMessage,
  exportConfiguration,
  exportDiagnosticBundle,
  getAudioDependencyStatus,
  getAutostartEnabled,
  getCurrentProcessId,
  getForegroundExecutable,
  getHostCapabilities,
  getHidHostInstalled,
  getVirtualHidStatus,
  refreshVirtualHidStatus,
  installVirtualHidDriver,
  uninstallVirtualHidDriver,
  getRemoteBattery,
  getSettings,
  importConfiguration,
  installHidHost,
  listAudioEndpoints,
  openDiagnosticLogFolder,
  openExternalUrl,
  readDiagnosticLog,
  resetSettings,
  restartHidHost,
  saveSettings,
  setAutostartEnabled,
  setTrayLanguage,
  uninstallHidHost,
} from "../lib/api";
import { ONBOARDING_KEY, ACCENT_KEY, applyAccent, applyTheme } from "../lib/preferences";
import { confirmAction } from "../lib/confirm";
import { requestButtonsPage, requestDeviceSettings } from "../lib/navigation";
import { setLanguagePreference, statusLabel, tr, trDynamic, uiLocale } from "../lib/i18n";
import {
  customHotkeyAfterToolChange,
  defaultHotkeyFor,
  voiceToolLabel,
  voiceToolOptions,
} from "../lib/voiceTools";
import {
  audioDependencyDetailKey,
  audioDependencyTitleKey,
  audioDependencyTone,
  suggestedCableEndpointName,
  isAudioDependencyReady,
  mergeAudioEndpoints,
  VB_CABLE_INSTALL_URL,
} from "../lib/audioDependency";

import { useVoiceToolCatalog } from "../composables/useVoiceToolCatalog";
import PageHeader from "../components/PageHeader.vue";
import HotkeyPickerDialog from "../components/HotkeyPickerDialog.vue";
import { hotkeyKeyLabel } from "../lib/hotkey";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
import CommonPhrasesSettings from "../components/CommonPhrasesSettings.vue";
import SettingRow from "../components/ui/SettingRow.vue";
import ToggleGroup from "../components/ui/ToggleGroup.vue";
import IconButton from "../components/ui/IconButton.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import Notice from "../components/ui/Notice.vue";
import RangeInput from "../components/ui/RangeInput.vue";
import SelectInput from "../components/ui/SelectInput.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import Switch from "../components/ui/Switch.vue";
import { pushToast } from "../lib/toast";
import type { AudioDependencyStatus, AudioEndpoint, SettingsDocument } from "../lib/types";
import { useEngineStore } from "../stores/engine";
import { useStatusStore } from "../stores/status";
import {
  effectiveAudioEndpointId,
  effectiveGainDb,
  effectiveVoiceInputTool,
} from "../lib/effectiveAudio";

type SectionId = "appearance" | "common-phrases" | "audio" | "startup" | "diagnostics" | "advanced";

interface HealthCheck {
  label: string;
  ok: boolean;
  detail: string;
}

interface SettingsSection {
  id: SectionId;
  label: string;
  icon: Component;
  /** Nav group this section belongs to. */
  group: string;
}

const SECTIONS: SettingsSection[] = [
  { id: "appearance", label: "外观与语言", icon: Palette, group: "常规" },
  { id: "common-phrases", label: "常用语", icon: MessageSquare, group: "常规" },
  { id: "audio", label: "音频及语音", icon: Volume2, group: "常规" },
  { id: "startup", label: "启动与窗口", icon: Power, group: "系统" },
  { id: "diagnostics", label: "数据与诊断", icon: HardDrive, group: "系统" },
  { id: "advanced", label: "高级", icon: Wrench, group: "系统" },
];

/** Options rendered as segmented buttons in the appearance section. */
const THEME_OPTIONS = [
  { value: "system", label: "跟随系统" },
  { value: "light", label: "浅色" },
  { value: "dark", label: "深色" },
] as const;

const ACCENT_OPTIONS = [
  { value: "default", label: "默认", swatch: "var(--accent)" },
  { value: "blue", label: "蓝色", swatch: "#2563eb" },
  { value: "purple", label: "紫色", swatch: "#7c3aed" },
  { value: "orange", label: "橙色", swatch: "#ea7317" },
] as const;

const LANGUAGE_OPTIONS = [
  { value: "system", label: "跟随系统" },
  { value: "zh-Hans", label: "中文" },
  { value: "en", label: "English" },
] as const;

const NAV_GROUPS = ["常规", "系统"];

const navQuery = shallowRef("");

const visibleSections = computed(() => {
  const query = navQuery.value.trim().toLowerCase();
  if (!query) return SECTIONS;
  return SECTIONS.filter((section) => trDynamic(section.label).toLowerCase().includes(query));
});

const store = useEngineStore();
const statusStore = useStatusStore();
const engine = useEngineStore();
const settingsDocument = ref<SettingsDocument | null>(null);
const endpoints = ref<AudioEndpoint[]>([]);
const gain = shallowRef(0);
const endpointId = shallowRef("");
const autostart = shallowRef(false);
const closeToTray = shallowRef(true);
/** 设置页的「启动后隐藏到系统托盘」，持久化时取反写入 `open_window_at_launch`。 */
const startHiddenToTray = shallowRef(false);
const theme = ref("system");
const language = ref("system");
const accent = ref(localStorage.getItem(ACCENT_KEY) ?? "default");
const backupBusy = shallowRef(false);
const backupMessage = shallowRef<string | null>(null);
const diagnosticText = shallowRef<string | null>(null);
const diagnosticBusy = shallowRef(false);
const logOpen = shallowRef(false);
const healthChecks = ref<HealthCheck[]>([]);
const healthBusy = shallowRef(false);
const activeSection = ref<SectionId>("appearance");
const busy = shallowRef(false);
const loading = shallowRef(true);
const error = shallowRef<string | null>(null);

/** 全按键支持服务（RC003 helper）：安装 / 更新 / 卸载都在设置里统一管理。 */
const hidHostInstalled = shallowRef(false);
const helperBusy = shallowRef(false);
/**
 * 安装 / 卸载全按键支持服务的长流程提示（等 UAC、等轮询、装完还差用户开开关）。
 *
 * 按 `docs/ui-design-system.md` §4.1：长流程用页内 `Notice` 常驻，结果另发一条
 * `success` Toast。所以这条 Notice 只放「还需要用户做点什么」的内容——装成功本身
 * 不留在它上面，也不再把「计划任务会自动启动」这种过程承诺写进结果里。
 */
const helperNotice = shallowRef<{
  text: string;
  kind: "info" | "warning";
  /** 服务装好了但开关还关着：给一个内联出口，别让用户自己去找开关。 */
  showOpenSwitch?: boolean;
} | null>(null);
let helperPoll: number | undefined;

async function refreshHelperInstalled(): Promise<boolean> {
  hidHostInstalled.value = await getHidHostInstalled().catch(() => false);
  return hidHostInstalled.value;
}

async function installHelper() {
  if (helperBusy.value) return;
  helperBusy.value = true;
  helperNotice.value = null;
  error.value = null;
  window.clearInterval(helperPoll);
  try {
    await installHidHost();
    helperNotice.value = {
      text: tr("已交给后台组件执行，完成后会自动刷新状态。"),
      kind: "info",
    };
    let attempts = 0;
    helperPoll = window.setInterval(async () => {
      attempts += 1;
      if (await refreshHelperInstalled()) {
        window.clearInterval(helperPoll);
        // 装完只是「结果」，用 Toast 说一句就够。计划任务没有触发器，只有打开
        // 「全按键支持」开关时才会 schtasks /run 拉起它——所以「已安装」不等于
        // 「已经在跑」，这里必须把还差用户做的那一步交给页内 Notice。
        helperNotice.value =
          settingsDocument.value?.global.rc003_capture_enabled === true
            ? null
            : {
                text: tr(
                  "服务已安装；在「按键」页打开「全按键支持」开关后，返回键和音量键才会生效。",
                ),
                kind: "info",
                showOpenSwitch: true,
              };
        pushToast(tr("全按键支持服务已安装"), "success");
      } else if (attempts >= 15) {
        window.clearInterval(helperPoll);
        helperNotice.value = {
          text: tr("没有完成全按键支持服务安装，请确认已在弹窗中通过管理员授权后重新检测。"),
          kind: "warning",
        };
      }
    }, 2000);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    helperBusy.value = false;
  }
}

/**
 * 重启全按键支持服务。
 *
 * 后台 agent 换代后，宿主（WUDFHost）若是共享进程，助手会拒绝自我刷新
 * （`[AGENT-REFRESH] reason=shared_host`），新版本会静默不生效。这对
 * `schtasks /End` + `/Run` 不需要管理员，正好补上这个缺口。
 */
async function restartHelperHost() {
  if (helperBusy.value) return;
  helperBusy.value = true;
  helperNotice.value = null;
  error.value = null;
  window.clearInterval(helperPoll);
  try {
    await restartHidHost();
    helperNotice.value = {
      text: tr("服务已重启；新版本的后台组件从现在起生效。"),
      kind: "info",
    };
    pushToast(tr("已重启全按键支持服务"), "success");
    await refreshHelperInstalled();
    await refreshVirtualHid();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    helperBusy.value = false;
  }
}

async function uninstallHelper() {
  if (helperBusy.value) return;
  const confirmed = await confirmAction({
    title: tr("卸载全按键支持服务？"),
    message: tr(
      "会移除后台计划任务，返回键和音量键的全按键支持将失效；需要时会重新请求管理员授权。",
    ),
    confirmLabel: tr("卸载"),
    danger: true,
  });
  if (!confirmed) return;
  helperBusy.value = true;
  helperNotice.value = null;
  error.value = null;
  window.clearInterval(helperPoll);
  try {
    await uninstallHidHost();
    helperNotice.value = {
      text: tr("已请求管理员授权；完成 UAC 后会移除服务。"),
      kind: "info",
    };
    // 顺手关掉全按键支持，避免留下「开关开着但服务没了」的状态。
    if (settingsDocument.value?.global.rc003_capture_enabled) {
      const next: SettingsDocument = {
        ...settingsDocument.value,
        global: { ...settingsDocument.value.global, rc003_capture_enabled: false },
      };
      await saveSettings(next);
      settingsDocument.value = next;
    }
    let attempts = 0;
    helperPoll = window.setInterval(async () => {
      attempts += 1;
      if (!(await refreshHelperInstalled())) {
        window.clearInterval(helperPoll);
        helperNotice.value = null;
        pushToast(tr("全按键支持服务已卸载"), "success");
      } else if (attempts >= 15) {
        window.clearInterval(helperPoll);
        helperNotice.value = {
          text: tr("没有完成全按键支持服务卸载，请确认已在弹窗中通过管理员授权后重新检测。"),
          kind: "warning",
        };
      }
    }, 2000);
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    helperBusy.value = false;
  }
}

let saveTimer: number | undefined;
let loadingSettings = false;

function sectionVisible(id: SectionId): boolean {
  return activeSection.value === id;
}

const accentModel = computed({
  get: () => accent.value,
  set: (value: string) => changeAccent(value),
});

const voiceInputTool = shallowRef("");

// 「实际生效」的三个音频设置。设备有独立设置时以设备为准（与 core 的
// settings_namespace 解析口径一致）；此时全局控件置灰，并提示去设备设置里改。
const connectedDeviceId = computed(() => engine.snapshot?.connection.remote?.deviceId ?? null);

function deviceOverridden<T>(resolve: () => { value: T; source: "device" | "global" }): T | null {
  if (!settingsDocument.value || !connectedDeviceId.value) return null;
  const resolved = resolve();
  return resolved.source === "device" ? resolved.value : null;
}

const gainOverride = computed(() =>
  deviceOverridden(() => effectiveGainDb(settingsDocument.value!, connectedDeviceId.value)),
);
const endpointOverride = computed(() =>
  deviceOverridden(() =>
    effectiveAudioEndpointId(settingsDocument.value!, connectedDeviceId.value),
  ),
);
const toolOverride = computed(() =>
  deviceOverridden(() => effectiveVoiceInputTool(settingsDocument.value!, connectedDeviceId.value)),
);

const gainValue = computed(() => gainOverride.value ?? gain.value);
const endpointValue = computed(() => endpointOverride.value ?? endpointId.value);
const toolValue = computed(() => toolOverride.value ?? voiceInputTool.value);
const gainLocked = computed(() => gainOverride.value !== null);
const endpointLocked = computed(() => endpointOverride.value !== null);
const toolLocked = computed(() => toolOverride.value !== null);
function onGainInput(value: number) {
  if (!gainLocked.value) gain.value = value;
}
function onEndpointInput(value: string) {
  if (!endpointLocked.value) endpointId.value = value;
}

// 被设备覆盖的项：把用户直接送到「连接页 → 设备设置」抽屉。
function openDeviceSettings() {
  requestDeviceSettings();
}
const transcriptHistory = shallowRef(false);
const voiceHotkey = ref<string[] | null>(null);
const { catalog: voiceToolCatalog, load: loadVoiceToolCatalog } = useVoiceToolCatalog();
/**
 * 语音热键只做点选，不做键盘直接采集。
 *
 * 直接采集要拦系统级按键，输入法会把自己的语音和弦抢先截走（Chatterfly 的
 * Ctrl+Win 就是典型），现场表现为「按了没反应 / 只录到半个组合」。点选既能
 * 确定地选到任意组合，也不会触发任何程序。
 */
const hotkeyPickerOpen = shallowRef(false);
const toolDefaultHotkey = computed(() => defaultHotkeyFor(voiceToolCatalog.value, toolValue.value));
/** 当前生效的热键按「键帽」拆开显示（与按键方案里的快捷键录入同一套展示）。 */
const hotkeyParts = computed(() =>
  (voiceHotkey.value ?? toolDefaultHotkey.value).map(hotkeyKeyLabel),
);
const hasCustom = computed(() => (voiceHotkey.value?.length ?? 0) > 0);
/**
 * 切换输入工具。
 *
 * 热键是工具专属的（豆包默认右 Alt、Chatterfly 默认修饰键组合），换工具后旧的自定义
 * 热键不再适用——继续沿用会「设置里保存成功、遥控器按了没反应」，所以回到新工具的
 * 默认预设；同工具重选不动。
 */
function onToolInput(value: string) {
  if (toolLocked.value) return;
  if (value === voiceInputTool.value) return;
  voiceHotkey.value = customHotkeyAfterToolChange(voiceInputTool.value, value, voiceHotkey.value);
  voiceInputTool.value = value;
}
const currentVoiceToolLabel = computed(
  () =>
    voiceToolOptions(voiceToolCatalog.value, toolValue.value).find(
      (tool) => tool.id === toolValue.value,
    )?.label ?? "",
);

function openHotkeyPicker() {
  hotkeyPickerOpen.value = true;
}

function applyPickedHotkey(keys: string[]) {
  voiceHotkey.value = keys.length > 0 ? keys : null;
  hotkeyPickerOpen.value = false;
}

function resetHotkey() {
  voiceHotkey.value = null;
}

const languageModel = computed({
  get: () => language.value,
  set: (value: string) => changeLanguage(value),
});

/** VB-CABLE 依赖状态（设置页状态行 + 设备下拉内联提示 + 健康检查共用）。 */
const audioDependency = shallowRef<AudioDependencyStatus | null>(null);
const audioDependencyLoading = shallowRef(false);
const audioDependencyError = shallowRef<string | null>(null);
const audioDependencyReady = computed(() => isAudioDependencyReady(audioDependency.value));
const dependencyTone = computed(() => audioDependencyTone(audioDependency.value?.state));
const dependencyStateLabel = computed(() => {
  if (audioDependencyLoading.value) return tr("检测中…");
  if (audioDependencyError.value) return tr("无法检测 VB-CABLE");
  switch (audioDependency.value?.state) {
    case "ready":
      return tr("已安装");
    case "missing":
      return tr("未检测到");
    case "disabled":
      return tr("设备未启用");
    default:
      return tr("检测中…");
  }
});
const dependencyHint = computed(() => {
  if (audioDependencyLoading.value) return tr("正在检测 VB-CABLE…");
  if (audioDependencyError.value) return audioDependencyError.value;
  const state = audioDependency.value?.state;
  if (state === "ready") {
    const name = suggestedCableEndpointName(audioDependency.value);
    return name ? tr("语音输出设备：{name}", { name }) : tr("已找到可用的语音输出设备。");
  }
  const detailKey = audioDependencyDetailKey(state);
  return detailKey ? tr(detailKey) : "";
});
const showVbCableDownload = computed(
  () => audioDependency.value?.state === "missing" || Boolean(audioDependencyError.value),
);
const cableMissingInline = computed(
  () => !audioDependencyReady.value && !audioDependencyLoading.value,
);
/** 已保存的端点不在当前设备列表（含 CABLE 合并结果）里 → 提示重新选择。 */
const endpointMissing = computed(
  () =>
    Boolean(endpointId.value) &&
    !endpoints.value.some((endpoint) => endpoint.id === endpointId.value) &&
    !audioDependencyLoading.value,
);

function applyAudioDependency(status: AudioDependencyStatus) {
  audioDependency.value = status;
  endpoints.value = mergeAudioEndpoints(endpoints.value, status.cableEndpoints ?? []);
}

async function loadAudioDependency() {
  audioDependencyLoading.value = true;
  audioDependencyError.value = null;
  try {
    applyAudioDependency(await getAudioDependencyStatus());
  } catch (cause) {
    audioDependency.value = null;
    audioDependencyError.value = errorMessage(cause);
  } finally {
    audioDependencyLoading.value = false;
  }
}

async function downloadVbCable() {
  const url = audioDependency.value?.installUrl?.trim() || VB_CABLE_INSTALL_URL;
  try {
    await openExternalUrl(url);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

function dependencyHealthDetail(status: AudioDependencyStatus): string {
  const label = tr(audioDependencyTitleKey(status.state));
  // 健康检查只留标题，端点名过长会在行内溢出；设备名在音频设置里展示。
  return label;
}

async function load() {
  loadingSettings = true;
  error.value = null;
  try {
    const settings = await getSettings();
    settingsDocument.value = settings;
    statusStore.setVoiceInputTool(settings.global.voice_input_tool);
    voiceInputTool.value = String(settings.global.voice_input_tool ?? "");
    const storedHotkey = settings.global.voice_hotkey;
    voiceHotkey.value =
      Array.isArray(storedHotkey) && storedHotkey.length > 0 ? storedHotkey : null;
    transcriptHistory.value = settings.global.local_transcript_history_enabled ?? false;
    gain.value = settings.global.gain_db;
    endpointId.value = settings.global.audio_endpoint_id ?? "";
    autostart.value = await getAutostartEnabled();
    closeToTray.value = settings.global.close_to_tray ?? true;
    startHiddenToTray.value = settings.global.open_window_at_launch === false;
    theme.value = String(settings.global.theme_preference ?? "system");
    language.value = String(settings.global.application_language ?? "system");
    setLanguagePreference(language.value);
    endpoints.value = await listAudioEndpoints();
    await refreshHelperInstalled();
  } catch (cause) {
    error.value = errorMessage(cause);
    activeSection.value = "diagnostics";
  } finally {
    loadingSettings = false;
    loading.value = false;
  }
}

function changeLanguage(value: string) {
  language.value = value;
  setLanguagePreference(value);
  void setTrayLanguage(uiLocale.value);
  scheduleAutoSave();
}

function changeAccent(value: string) {
  accent.value = value;
  localStorage.setItem(ACCENT_KEY, value);
  applyAccent(value);
}

async function save(auto = false) {
  if (!settingsDocument.value) return;
  if (!auto) busy.value = true;
  error.value = null;
  try {
    const latest = await getSettings();
    const next: SettingsDocument = {
      ...latest,
      global: {
        ...latest.global,
        gain_db: gain.value,
        audio_endpoint_id: endpointId.value || null,
        voice_input_tool: voiceInputTool.value || null,
        voice_hotkey: voiceHotkey.value,
        local_transcript_history_enabled: transcriptHistory.value,
        close_to_tray: closeToTray.value,
        open_window_at_launch: !startHiddenToTray.value,
        theme_preference: theme.value,
        application_language: language.value,
      },
    };
    await saveSettings(next);
    statusStore.setVoiceInputTool(next.global.voice_input_tool);
    await setAutostartEnabled(autostart.value);
    applyTheme(theme.value);
    settingsDocument.value = next;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

function scheduleAutoSave() {
  if (loadingSettings || !settingsDocument.value) return;
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = undefined;
    void save(true);
  }, 250);
}

watch(
  [
    gain,
    endpointId,
    theme,
    language,
    autostart,
    closeToTray,
    startHiddenToTray,
    voiceInputTool,
    transcriptHistory,
    voiceHotkey,
  ],
  scheduleAutoSave,
);

onBeforeUnmount(() => {
  window.clearInterval(helperPoll);
  if (saveTimer === undefined) return;
  window.clearTimeout(saveTimer);
  saveTimer = undefined;
  if (settingsDocument.value) {
    void save(true);
  }
});

async function openLogs() {
  error.value = null;
  try {
    await openDiagnosticLogFolder();
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function loadDiagnostics() {
  diagnosticBusy.value = true;
  error.value = null;
  try {
    diagnosticText.value = await readDiagnosticLog(128 * 1024);
    logOpen.value = true;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    diagnosticBusy.value = false;
  }
}

async function exportDiagnostics() {
  diagnosticBusy.value = true;
  error.value = null;
  try {
    const path = await saveDialog({
      title: tr("导出诊断包"),
      defaultPath: "shengjian-diagnostics.json",
      filters: [{ name: tr("诊断"), extensions: ["json"] }],
    });
    if (!path) return;
    await exportDiagnosticBundle(path);
    backupMessage.value = tr("诊断包已导出。");
    pushToast(tr("诊断包已导出"), "success");
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    diagnosticBusy.value = false;
  }
}

async function resetAllSettings() {
  const confirmed = await confirmAction({
    title: "恢复默认设置？",
    message: "当前全局设置和设备覆盖会被清除，此操作不可恢复。",
    confirmLabel: "恢复默认设置",
    danger: true,
  });
  if (!confirmed) return;
  busy.value = true;
  error.value = null;
  try {
    await resetSettings();
    await load();
    pushToast(tr("已恢复默认设置"), "success");
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    busy.value = false;
  }
}

async function runHealthCheck() {
  healthBusy.value = true;
  error.value = null;
  try {
    // The catalog is cached; awaiting it here makes sure the check shows the
    // configured display name even if the settings page loads first.
    await loadVoiceToolCatalog();
    const [
      caps,
      helper,
      endpoints,
      battery,
      foreground,
      currentSettings,
      audioDependencyStatus,
      currentPid,
    ] = await Promise.all([
      getHostCapabilities(),
      getHidHostInstalled(),
      listAudioEndpoints(),
      getRemoteBattery(),
      getForegroundExecutable(),
      getSettings(),
      getAudioDependencyStatus(),
      getCurrentProcessId(),
    ]);
    const log = await readDiagnosticLog(256 * 1024);
    const lines = log.split("\n");
    // 只统计本次运行（pid 匹配）：历史会话/上一次安装的失败不应影响当前健康状态。
    const currentRunLines = lines.filter((line) => line.includes(`pid=${currentPid} `));
    const resourceLine = [...currentRunLines]
      .reverse()
      .find((line) => line.includes("resource_probe"));
    const workingSet = resourceLine?.match(/working_set_kb=(\d+)/)?.[1];
    // 同一次失败会写多行（stage / failure_detail / 顶层），按秒去重后计数。
    const seenErrorSeconds = new Set<string>();
    const recentErrors = currentRunLines
      .filter((line) => /result=(err|failed)|error=|terminal_result=failed/.test(line))
      .filter((line) => {
        const key = line.slice(0, 19);
        if (seenErrorSeconds.has(key)) return false;
        seenErrorSeconds.add(key);
        return true;
      }).length;
    healthChecks.value = [
      { label: tr("Host 平台"), ok: caps.platform.length > 0, detail: caps.platform },
      {
        label: tr("宏运行时"),
        ok: caps.macroRuntime,
        detail: caps.macroRuntime ? tr("可用") : tr("不可用"),
      },
      { label: tr("全按键支持服务"), ok: helper, detail: helper ? tr("已安装") : tr("未安装") },
      {
        label: tr("虚拟音频设备"),
        ok: audioDependencyStatus.state === "ready",
        detail: dependencyHealthDetail(audioDependencyStatus),
      },
      { label: tr("音频输出设备"), ok: endpoints.length > 0, detail: `${endpoints.length}` },
      { label: tr("遥控器连接"), ok: store.phase === "ready", detail: statusLabel(store.phase) },
      {
        label: tr("遥控器电量"),
        ok: battery !== null,
        detail: battery === null ? tr("未知") : `${battery}%`,
      },
      {
        label: tr("语音输入工具"),
        ok: Boolean(currentSettings.global.voice_input_tool),
        detail:
          voiceToolLabel(voiceToolCatalog.value, currentSettings.global.voice_input_tool) ||
          tr("未选择"),
      },
      { label: tr("前台应用识别"), ok: Boolean(foreground), detail: foreground ?? tr("未识别") },
      {
        label: tr("进程工作集"),
        ok: workingSet !== undefined,
        detail: workingSet ? `${workingSet} KB` : tr("日志中暂无"),
      },
      {
        label: tr("近期错误行"),
        ok: recentErrors === 0,
        detail: `${recentErrors}`,
      },
    ];
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    healthBusy.value = false;
  }
}

function restartOnboarding() {
  localStorage.removeItem(ONBOARDING_KEY);
  window.location.reload();
}

async function exportBackup() {
  backupBusy.value = true;
  backupMessage.value = null;
  error.value = null;
  try {
    const path = await saveDialog({
      title: tr("导出配置备份"),
      defaultPath: "shengjian-backup.json",
      filters: [{ name: tr("配置备份"), extensions: ["json"] }],
    });
    if (!path) return;
    await exportConfiguration(path, accent.value, localStorage.getItem("shengjian.last-remote"));
    backupMessage.value = tr("配置备份已导出。");
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    backupBusy.value = false;
  }
}

async function importBackup() {
  backupBusy.value = true;
  backupMessage.value = null;
  error.value = null;
  try {
    const selected = await open({
      title: tr("恢复配置备份"),
      multiple: false,
      filters: [{ name: tr("配置备份"), extensions: ["json"] }],
    });
    if (!selected || Array.isArray(selected)) return;
    const confirmed = await confirmAction({
      title: "恢复配置备份？",
      message: "恢复会覆盖当前 settings、profiles 和宏库，继续？",
      confirmLabel: "恢复备份",
      danger: true,
    });
    if (!confirmed) return;
    const result = await importConfiguration(selected);
    if (result.accent) {
      accent.value = result.accent;
      localStorage.setItem(ACCENT_KEY, result.accent);
      applyAccent(result.accent);
    }
    if (result.lastRemoteId) {
      localStorage.setItem("shengjian.last-remote", result.lastRemoteId);
    }
    await load();
    backupMessage.value = tr(
      "恢复完成：{profiles} 个 profile，{macros} 个宏定义，{transcripts} 条语音记录。",
      {
        profiles: result.profiles,
        macros: result.macros,
        transcripts: result.transcripts,
      },
    );
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    backupBusy.value = false;
  }
}

/** 虚拟键盘后端（WinUHid）状态：只读展示，未安装时功能自动回落，不影响使用。 */
const virtualHid = shallowRef<{ available: boolean; driver_version: number }>({
  available: false,
  driver_version: 0,
});

/**
 * 提醒义务：当前热键如果超过两个键，报告层表达不了，只能靠虚拟键盘。
 * 驱动没装时按键到不了输入法（会回落注入，而部分输入法会忽略注入键），
 * 这里提前说清楚，而不是让用户按下去没反应再来查。
 */
const hotkeyNeedsVirtualHid = computed(
  () => !virtualHid.value.available && hotkeyParts.value.length > 2,
);

async function refreshVirtualHid() {
  virtualHid.value = await getVirtualHidStatus().catch(() => ({
    available: false,
    driver_version: 0,
  }));
}

/** 驱动安装/卸载会弹 UAC，完成后轮询状态（安装几秒，卸载立刻）。 */
const virtualHidBusy = shallowRef(false);

async function waitForVirtualHid(expectAvailable: boolean) {
  for (let attempt = 0; attempt < 20; attempt += 1) {
    await new Promise((resolve) => setTimeout(resolve, 1500));
    // 驱动装/卸是助手侧异步做的：先让它重新问一次可用性，再读状态。
    await refreshVirtualHidStatus().catch(() => undefined);
    await new Promise((resolve) => setTimeout(resolve, 400));
    await refreshVirtualHid();
    if (virtualHid.value.available === expectAvailable) return true;
  }
  return false;
}

async function installVirtualHid() {
  if (virtualHidBusy.value) return;
  virtualHidBusy.value = true;
  helperNotice.value = null;
  error.value = null;
  try {
    await installVirtualHidDriver();
    helperNotice.value = {
      text: tr("已交给后台组件执行，完成后会自动刷新状态。"),
      kind: "info",
    };
    if (await waitForVirtualHid(true)) {
      pushToast(tr("虚拟键盘驱动已就绪"), "success");
      helperNotice.value = null;
    } else {
      helperNotice.value = {
        text: tr("驱动安装没有完成：看诊断日志里的 [WINUHID-DRIVER] 行（含失败码）。"),
        kind: "warning",
      };
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    virtualHidBusy.value = false;
  }
}

async function uninstallVirtualHid() {
  if (virtualHidBusy.value) return;
  const confirmed = await confirmAction({
    title: tr("卸载虚拟键盘驱动？"),
    message: tr(
      "会移除 Root\\WinUHid 设备与驱动包；证书保留在受信任发布者里。虚拟键盘后端随即不可用，语音热键自动回落报告层。",
    ),
    confirmLabel: tr("卸载"),
    danger: true,
  });
  if (!confirmed) return;
  virtualHidBusy.value = true;
  helperNotice.value = null;
  error.value = null;
  try {
    await uninstallVirtualHidDriver();
    if (await waitForVirtualHid(false)) {
      pushToast(tr("虚拟键盘驱动已移除"), "success");
      helperNotice.value = null;
    } else {
      helperNotice.value = {
        text: tr("驱动还没有移除：看诊断日志里的 [WINUHID-DRIVER] 行（含失败码）。"),
        kind: "warning",
      };
    }
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    virtualHidBusy.value = false;
  }
}

onMounted(async () => {
  await Promise.all([load(), loadVoiceToolCatalog(), loadAudioDependency(), refreshVirtualHid()]);
});
</script>

<template>
  <section class="page">
    <PageHeader
      eyebrow="Settings"
      title="设置"
      description="调整外观、音频、启动与诊断设置；所有更改即时保存。"
    />

    <Notice v-if="error" kind="error">{{ error }}</Notice>

    <div class="settings-shell">
      <nav class="settings-nav" :aria-label="tr('设置分区')">
        <label class="settings-search">
          <Search :size="14" />
          <input
            v-model="navQuery"
            type="search"
            :placeholder="tr('搜索设置项')"
            :aria-label="tr('搜索设置项')"
          />
        </label>

        <div class="settings-nav-scroll">
          <template v-for="group in NAV_GROUPS" :key="group">
            <p
              v-if="visibleSections.some((section) => section.group === group)"
              class="settings-nav-group"
            >
              {{ trDynamic(group) }}
            </p>
            <button
              v-for="section in visibleSections.filter((item) => item.group === group)"
              :id="`tab-${section.id}`"
              :key="section.id"
              type="button"
              role="tab"
              class="settings-nav-item"
              :class="{ active: activeSection === section.id }"
              :aria-selected="activeSection === section.id"
              :aria-controls="`panel-${section.id}`"
              :tabindex="activeSection === section.id ? 0 : -1"
              @click="activeSection = section.id"
            >
              <component :is="section.icon" :size="15" />
              {{ trDynamic(section.label) }}
            </button>
          </template>

          <p v-if="!visibleSections.length" class="settings-nav-empty">
            {{ tr("没有匹配的设置项") }}
          </p>
        </div>
      </nav>

      <div
        class="settings-content stack"
        role="tabpanel"
        :id="`panel-${activeSection}`"
        :aria-labelledby="`tab-${activeSection}`"
        :aria-busy="loading || undefined"
      >
        <LoadingState v-if="loading" label="正在加载…" description="正在读取本地设置…" />

        <template v-if="!loading && sectionVisible('appearance')">
          <p class="settings-group-label">{{ tr("常规") }}</p>
          <Card v-if="settingsDocument">
            <div class="settings-rows">
              <SettingRow label="主题">
                <ToggleGroup v-model="theme" :items="THEME_OPTIONS" aria-label="主题" />
              </SettingRow>

              <SettingRow label="强调色">
                <ToggleGroup v-model="accentModel" :items="ACCENT_OPTIONS" aria-label="强调色" />
              </SettingRow>

              <SettingRow label="应用语言">
                <ToggleGroup
                  v-model="languageModel"
                  :items="LANGUAGE_OPTIONS"
                  aria-label="应用语言"
                />
              </SettingRow>
            </div>
          </Card>
        </template>

        <template v-if="!loading && sectionVisible('common-phrases')">
          <p class="settings-group-label">{{ tr("常用语") }}</p>
          <Card padding="none">
            <CommonPhrasesSettings />
          </Card>
        </template>

        <template v-if="!loading && sectionVisible('audio')">
          <p class="settings-group-label">{{ tr("音频") }}</p>
          <Card v-if="settingsDocument">
            <div class="settings-rows">
              <SettingRow label="虚拟音频设备" :hint="dependencyHint">
                <div class="audio-dependency-control">
                  <StatusPill
                    :label="dependencyStateLabel"
                    :tone="dependencyTone"
                    :title="dependencyHint"
                    dot
                  />
                  <Button v-if="showVbCableDownload" size="sm" @click="downloadVbCable">
                    {{ tr("下载 VB-CABLE") }}
                  </Button>
                  <Button size="sm" :disabled="audioDependencyLoading" @click="loadAudioDependency">
                    {{ audioDependencyLoading ? tr("检测中…") : tr("重新检测") }}
                  </Button>
                </div>
              </SettingRow>

              <SettingRow
                label="语音输出设备"
                hint="遥控器语音输出到的设备；未设置时连接后自动选 CABLE"
              >
                <SelectInput
                  :model-value="endpointValue"
                  :disabled="endpointLocked"
                  @update:model-value="onEndpointInput"
                >
                  <option value="">{{ tr("未设置（连接时自动选 CABLE）") }}</option>
                  <option v-for="endpoint in endpoints" :key="endpoint.id" :value="endpoint.id">
                    {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
                  </option>
                </SelectInput>
              </SettingRow>
              <button
                v-if="endpointLocked"
                type="button"
                class="settings-inline-note settings-inline-action"
                :title="tr('打开设备设置')"
                @click="openDeviceSettings"
              >
                {{ tr("已被设备设置覆盖") }}
              </button>
              <div v-if="cableMissingInline" class="settings-inline-warning" role="status">
                <span>
                  {{ tr("未检测到 VB-CABLE：遥控器语音将没有声音输出。仍可选择其他输出设备。") }}
                </span>
                <Button
                  v-if="showVbCableDownload"
                  variant="ghost"
                  size="sm"
                  @click="downloadVbCable"
                >
                  {{ tr("下载 VB-CABLE") }}
                </Button>
              </div>
              <p v-if="endpointMissing" class="settings-inline-note warning" role="alert">
                {{ tr("已保存的语音输出设备当前不可用，请重新选择。") }}
              </p>

              <SettingRow label="增益" hint="语音播放的音量增益">
                <div class="range-control">
                  <RangeInput
                    :model-value="gainValue"
                    :min="0"
                    :max="24"
                    :step="1"
                    :disabled="gainLocked"
                    @update:model-value="onGainInput"
                  />
                  <span class="range-value mono">{{ gainValue }} dB</span>
                </div>
              </SettingRow>
              <button
                v-if="gainLocked"
                type="button"
                class="settings-inline-note settings-inline-action"
                :title="tr('打开设备设置')"
                @click="openDeviceSettings"
              >
                {{ tr("已被设备设置覆盖") }}
              </button>
            </div>
          </Card>

          <p class="settings-group-label">{{ tr("遥控器") }}</p>
          <Card>
            <div class="settings-rows">
              <SettingRow
                label="全按键支持服务"
                hint="返回键和音量键需要全按键支持组件；首次安装会请求管理员授权。后台组件更新后，用「重启宿主」立即生效。"
              >
                <div class="helper-control">
                  <StatusPill
                    :label="trDynamic(hidHostInstalled ? '已安装' : '未安装')"
                    :tone="hidHostInstalled ? 'success' : 'warning'"
                    dot
                  />
                  <Button
                    size="sm"
                    :variant="hidHostInstalled ? 'secondary' : 'primary'"
                    :disabled="helperBusy"
                    @click="installHelper"
                  >
                    {{ trDynamic(hidHostInstalled ? "更新" : "安装") }}
                  </Button>
                  <Button
                    v-if="hidHostInstalled"
                    size="sm"
                    variant="secondary"
                    :disabled="helperBusy"
                    @click="restartHelperHost"
                  >
                    {{ tr("重启宿主") }}
                  </Button>
                  <Button
                    v-if="hidHostInstalled"
                    size="sm"
                    variant="danger"
                    :disabled="helperBusy"
                    @click="uninstallHelper"
                  >
                    {{ tr("卸载") }}
                  </Button>
                </div>
              </SettingRow>
              <Notice v-if="helperNotice" :kind="helperNotice.kind">
                <span class="helper-notice-text">
                  {{ helperNotice.text }}
                  <Button v-if="helperNotice.showOpenSwitch" size="sm" @click="requestButtonsPage">
                    {{ tr("去按键页") }}
                  </Button>
                </span>
              </Notice>
              <SettingRow
                label="虚拟键盘（三键 / 任意组合）"
                hint="装上 WinUHid 驱动后，遥控器能发出报告层做不到的组合（如 Ctrl+Shift+Alt、Alt+W）。未安装时自动回落，不影响现有功能。"
              >
                <div class="helper-control">
                  <StatusPill
                    :label="trDynamic(virtualHid.available ? '可用' : '未安装')"
                    :tone="virtualHid.available ? 'success' : 'warning'"
                    dot
                  />
                  <span v-if="!virtualHid.available" class="helper-notice-text">
                    {{ tr("需要先安装 WinUHid 驱动") }}
                  </span>
                  <Button
                    v-if="!virtualHid.available"
                    size="sm"
                    variant="primary"
                    :disabled="virtualHidBusy"
                    @click="installVirtualHid"
                  >
                    {{ tr("安装") }}
                  </Button>
                  <Button
                    v-else
                    size="sm"
                    variant="danger"
                    :disabled="virtualHidBusy"
                    @click="uninstallVirtualHid"
                  >
                    {{ tr("卸载") }}
                  </Button>
                </div>
              </SettingRow>
            </div>
          </Card>

          <p class="settings-group-label">{{ tr("语音") }}</p>
          <Card v-if="settingsDocument">
            <div class="settings-rows">
              <SettingRow label="输入法" hint="遥控器语音转写注入的目标输入法">
                <SelectInput
                  :model-value="toolValue"
                  :disabled="toolLocked"
                  @update:model-value="onToolInput"
                >
                  <option value="">{{ tr("未选择") }}</option>
                  <option
                    v-for="tool in voiceToolOptions(voiceToolCatalog, toolValue)"
                    :key="tool.id"
                    :value="tool.id"
                  >
                    {{ tool.label }}
                  </option>
                </SelectInput>
              </SettingRow>
              <button
                v-if="toolLocked"
                type="button"
                class="settings-inline-note settings-inline-action"
                :title="tr('打开设备设置')"
                @click="openDeviceSettings"
              >
                {{ tr("已被设备设置覆盖") }}
              </button>

              <SettingRow
                label="语音热键"
                hint="按下遥控器语音键时注入的组合键；用点选方式设置，避免被输入法抢走按键"
              >
                <div class="hotkey-inline">
                  <span v-if="hotkeyParts.length" class="hotkey-keys" aria-live="polite">
                    <kbd v-for="(part, index) in hotkeyParts" :key="`${part}-${index}`">
                      {{ part }}
                    </kbd>
                  </span>
                  <span v-else class="hotkey-empty">{{ tr("未设置快捷键") }}</span>
                  <Button size="sm" @click="openHotkeyPicker">
                    <Keyboard :size="14" />
                    {{ tr("配置") }}
                  </Button>
                  <IconButton v-if="hasCustom" :label="trDynamic('恢复默认')" @click="resetHotkey">
                    <RotateCcw :size="15" />
                  </IconButton>
                </div>
              </SettingRow>

              <SettingRow label="语音记录" hint="只保存转写文字，不保存音频。">
                <Switch v-model="transcriptHistory" :label="trDynamic('语音记录')" />
              </SettingRow>
              <Notice v-if="hotkeyNeedsVirtualHid" kind="warning">
                <span class="helper-notice-text">
                  {{
                    tr(
                      "这个组合报告层发不出去，需要虚拟键盘驱动；没装的话按键到不了输入法（会回落成注入，部分输入法会忽略）。",
                    )
                  }}
                </span>
              </Notice>
            </div>
          </Card>
        </template>

        <template v-if="!loading && sectionVisible('startup')">
          <p class="settings-group-label">{{ tr("系统") }}</p>
          <Card v-if="settingsDocument">
            <div class="settings-rows">
              <SettingRow label="开机自动启动" hint="登录 Windows 后自动启动声键">
                <Switch v-model="autostart" :label="trDynamic('开机自动启动')" />
              </SettingRow>

              <SettingRow
                label="启动后隐藏到系统托盘"
                hint="启动时不弹出主窗口，只驻留系统托盘；需要时从托盘打开"
              >
                <Switch v-model="startHiddenToTray" :label="trDynamic('启动后隐藏到系统托盘')" />
              </SettingRow>

              <SettingRow label="关闭窗口时隐藏到托盘" hint="关闭后点窗口关闭按钮将直接退出应用">
                <Switch v-model="closeToTray" :label="trDynamic('关闭窗口时隐藏到托盘')" />
              </SettingRow>
            </div>
          </Card>
        </template>

        <template v-if="!loading && sectionVisible('diagnostics')">
          <p class="settings-group-label">{{ tr("数据") }}</p>
          <Card>
            <div class="settings-rows">
              <SettingRow
                label="导出备份"
                hint="导出为单个 JSON，含设置、方案、宏库、语音记录与上次设备"
              >
                <Button size="sm" :disabled="backupBusy" @click="exportBackup">
                  <Download :size="14" />
                  {{ tr("导出") }}
                </Button>
              </SettingRow>

              <SettingRow label="恢复备份" hint="从备份文件恢复，会覆盖当前配置">
                <Button size="sm" :disabled="backupBusy" @click="importBackup">
                  <Upload :size="14" />
                  {{ tr("恢复") }}
                </Button>
              </SettingRow>

              <p v-if="backupMessage" class="settings-inline-note">{{ backupMessage }}</p>
            </div>
          </Card>

          <p class="settings-group-label">{{ tr("诊断") }}</p>
          <Card>
            <div class="settings-rows">
              <SettingRow label="日志目录" hint="诊断日志位于 %LOCALAPPDATA%\Shengjian\Logs">
                <Button size="sm" @click="openLogs">
                  <FolderOpen :size="14" />
                  {{ tr("打开") }}
                </Button>
              </SettingRow>

              <SettingRow label="诊断日志" hint="查看最近的运行日志，不含语音内容">
                <div class="setting-row-inline">
                  <Button size="sm" :disabled="diagnosticBusy" @click="loadDiagnostics">
                    {{ trDynamic(diagnosticBusy ? "读取中" : "查看") }}
                  </Button>
                  <Button size="sm" :disabled="diagnosticBusy" @click="exportDiagnostics">
                    {{ tr("导出诊断包") }}
                  </Button>
                </div>
              </SettingRow>

              <div v-if="diagnosticText !== null" class="log-block">
                <div class="row-between">
                  <span class="muted">{{ tr("日志预览") }}</span>
                  <Button variant="ghost" size="sm" @click="logOpen = !logOpen">
                    <component :is="logOpen ? ChevronUp : ChevronDown" :size="14" />
                    {{ trDynamic(logOpen ? "收起日志" : "展开日志") }}
                  </Button>
                </div>
                <pre v-show="logOpen" class="log-preview">{{
                  diagnosticText || tr("日志为空")
                }}</pre>
              </div>
            </div>
          </Card>

          <p class="settings-group-label">{{ tr("健康检查") }}</p>
          <Card>
            <div class="settings-rows">
              <SettingRow
                label="运行健康检查"
                hint="检查平台、全按键支持服务、音频、连接、电量和前台应用识别"
              >
                <Button size="sm" :disabled="healthBusy" @click="runHealthCheck">
                  {{ trDynamic(healthBusy ? "检查中" : "运行") }}
                </Button>
              </SettingRow>

              <ul v-if="healthChecks.length" class="health-list">
                <li v-for="check in healthChecks" :key="check.label" class="health-row">
                  <span class="health-label">{{ check.label }}</span>
                  <span class="mono muted health-detail">{{ check.detail }}</span>
                  <StatusPill
                    :label="trDynamic(check.ok ? '可用' : '不可用')"
                    :tone="check.ok ? 'success' : 'warning'"
                    dot
                  />
                </li>
              </ul>
            </div>
          </Card>
        </template>

        <template v-if="!loading && sectionVisible('advanced')">
          <p class="settings-group-label">{{ tr("维护") }}</p>
          <Card>
            <div class="settings-rows">
              <SettingRow label="重新运行首次配置" hint="重新检查遥控器连接与语音链路">
                <Button size="sm" @click="restartOnboarding">
                  <RotateCcw :size="14" />
                  {{ tr("运行") }}
                </Button>
              </SettingRow>
            </div>
          </Card>

          <p class="settings-group-label">{{ tr("危险区域") }}</p>
          <Card class="danger-card">
            <div class="settings-rows">
              <SettingRow
                label="恢复默认设置"
                hint="清除全局设置与设备覆盖，不影响方案、宏和语音记录。此操作不可恢复"
              >
                <Button size="sm" variant="danger" :disabled="busy" @click="resetAllSettings">
                  {{ tr("恢复默认") }}
                </Button>
              </SettingRow>
            </div>
          </Card>
        </template>
      </div>
    </div>
  </section>

  <HotkeyPickerDialog
    :open="hotkeyPickerOpen"
    :model-value="voiceHotkey ?? toolDefaultHotkey"
    :tool-default="toolDefaultHotkey"
    :tool-label="currentVoiceToolLabel"
    @confirm="applyPickedHotkey"
    @close="hotkeyPickerOpen = false"
  />
</template>

<style scoped>
/* Sidebar nav (search + grouped items) beside a scrolling content panel. */
.settings-shell {
  display: grid;
  grid-template-columns: 236px minmax(0, 1fr);
  align-items: start;
  gap: var(--space-5);
  min-width: 0;
}

.settings-nav {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  position: sticky;
  top: var(--space-2);
  min-width: 0;
}

.settings-search {
  display: flex;
  align-items: center;
  gap: 6px;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 0 10px;
  color: var(--text-muted);
  transition: border-color var(--duration-fast) ease;
}

.settings-search:focus-within {
  border-color: var(--accent);
}

.settings-search input {
  flex: 1 1 auto;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  padding: 8px 0;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
}

.settings-search input::-webkit-search-cancel-button {
  display: none;
}

.settings-nav-scroll {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.settings-nav-group {
  margin: var(--space-3) 0 2px;
  padding: 0 12px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 600;
  letter-spacing: 0.04em;
}

.settings-nav-item {
  display: flex;
  align-items: center;
  gap: 10px;
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  padding: 9px 12px;
  color: var(--text-secondary);
  font: inherit;
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.settings-nav-item:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.settings-nav-item.active {
  background: var(--accent-soft);
  color: var(--accent-ink);
  font-weight: 600;
}

.settings-nav-empty {
  margin: var(--space-3) 0 0;
  padding: 0 12px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

/* Group label above a card, mirroring the reference settings layout. */
.settings-group-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 600;
}

/* Several controls inside one row (e.g. 查看 + 导出诊断包). */
.setting-row-inline {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

/* Feedback text that belongs to a row group rather than a single row. */
.settings-inline-note {
  margin: var(--space-2) 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.settings-inline-note.warning {
  color: var(--warning-text);
}

/* 提示本身是入口：点击跳到「连接页 → 设备设置」抽屉。 */
.settings-inline-action {
  /* 与 .setting-row-hint 同一个字号；靠右贴在控件下方。 */
  align-self: flex-end;
  padding: 0;
  border: none;
  background: none;
  color: var(--accent);
  font: inherit;
  font-size: var(--text-xs);
  line-height: 1.5;
  text-align: right;
  cursor: pointer;
}

.settings-inline-action:hover {
  text-decoration: underline;
  text-underline-offset: 2px;
}

.settings-inline-action:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
  border-radius: var(--radius-sm);
}

.helper-control {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

/* Notice 里「一句说明 + 一个内联出口」：按钮跟在文案后面，不另起一段。 */
.helper-notice-text {
  display: inline-flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.settings-inline-warning {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin-bottom: var(--space-2);
  border: 1px solid var(--warning);
  border-radius: var(--radius-md);
  background: var(--warning-soft);
  padding: var(--space-2) var(--space-3);
  color: var(--warning-text);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.audio-dependency-control {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.settings-rows {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.settings-content {
  min-width: 0;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  margin-top: var(--space-4);
  min-width: 0;
}

.settings-form {
  min-width: 0;
}

.range-control {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  width: 100%;
  max-width: 420px;
}

.range-value {
  flex: 0 0 auto;
  min-width: 52px;
  color: var(--text-secondary);
  text-align: right;
}

.setting-toggle {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
}

.setting-toggle-copy {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.setting-toggle-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.hotkey-inline {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

/* 热键展示：与按键方案的快捷键录入同款键帽，不再是"输入框" */
.hotkey-keys {
  display: flex;
  flex: 1 1 auto;
  flex-wrap: wrap;
  gap: 4px;
  min-width: 0;
}

.hotkey-keys kbd {
  border: 1px solid var(--border);
  border-bottom-color: var(--border-strong);
  border-radius: 4px;
  background: var(--bg-raised);
  padding: 0 6px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 11px;
  line-height: 20px;
}

.hotkey-empty {
  flex: 1 1 auto;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.setting-toggle-hint {
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.4;
}

.log-block {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.log-preview {
  max-height: 360px;
  margin: 0;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-3);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  line-height: 1.5;
  white-space: pre-wrap;
  word-break: break-all;
}

.health-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.health-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 220px) auto;
  align-items: center;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) 0;
  font-size: var(--text-sm);
}

.health-label {
  min-width: 0;
}

.health-detail {
  min-width: 0;
  justify-self: end;
  text-align: right;
  overflow-wrap: anywhere;
  white-space: normal;
}

.danger-card {
  border-color: var(--danger-soft);
}

.danger-card :deep(.ui-section-head h2) {
  color: var(--danger-text);
}

@media (max-width: 1100px) {
  .health-row {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .health-detail {
    grid-column: 1 / -1;
    justify-self: start;
    text-align: left;
  }
}
</style>
