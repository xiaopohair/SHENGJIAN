<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, shallowRef, watch, type Component } from "vue";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import {
  ChevronDown,
  ChevronUp,
  Download,
  FolderOpen,
  HardDrive,
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
  getAutostartEnabled,
  getForegroundExecutable,
  getHostCapabilities,
  getRc003HelperInstalled,
  getRemoteBattery,
  getSettings,
  importConfiguration,
  listAudioEndpoints,
  openDiagnosticLogFolder,
  readDiagnosticLog,
  resetSettings,
  saveSettings,
  setAutostartEnabled,
  setTrayLanguage,
} from "../lib/api";
import { ONBOARDING_KEY, ACCENT_KEY, applyAccent, applyTheme } from "../lib/preferences";
import { confirmAction } from "../lib/confirm";
import { setLanguagePreference, statusLabel, tr, trDynamic, uiLocale } from "../lib/i18n";
import { defaultHotkeyFor, voiceToolLabel, voiceToolOptions } from "../lib/voiceTools";

import { useVoiceToolCatalog } from "../composables/useVoiceToolCatalog";
import PageHeader from "../components/PageHeader.vue";
import { useHotkeyRecorder } from "../composables/useHotkeyRecorder";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
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
import type { AudioEndpoint, SettingsDocument } from "../lib/types";
import { useEngineStore } from "../stores/engine";
import { useStatusStore } from "../stores/status";

type SectionId = "appearance" | "audio" | "startup" | "diagnostics" | "advanced";

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
const transcriptHistory = shallowRef(false);
const voiceHotkey = ref<string[] | null>(null);
const { catalog: voiceToolCatalog, load: loadVoiceToolCatalog } = useVoiceToolCatalog();
const {
  recording,
  label: hotkeyLabel,
  hasCustom,
  toggle: toggleHotkey,
  reset: resetHotkey,
} = useHotkeyRecorder({
  hotkey: voiceHotkey,
  // 工具默认热键来自配置目录；旧 id 或加载失败时回退为「不注入热键」。
  fallback: () => defaultHotkeyFor(voiceToolCatalog.value, voiceInputTool.value),
});

const languageModel = computed({
  get: () => language.value,
  set: (value: string) => changeLanguage(value),
});

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
    const [caps, helper, endpoints, battery, foreground, currentSettings] = await Promise.all([
      getHostCapabilities(),
      getRc003HelperInstalled(),
      listAudioEndpoints(),
      getRemoteBattery(),
      getForegroundExecutable(),
      getSettings(),
    ]);
    const log = await readDiagnosticLog(256 * 1024);
    const lines = log.split("\n");
    const resourceLine = [...lines].reverse().find((line) => line.includes("resource_probe"));
    const workingSet = resourceLine?.match(/working_set_kb=(\d+)/)?.[1];
    const recentErrors = lines.filter((line) =>
      /result=(err|failed)|error=|terminal_result=failed/.test(line),
    ).length;
    healthChecks.value = [
      { label: tr("Host 平台"), ok: caps.platform.length > 0, detail: caps.platform },
      {
        label: tr("宏运行时"),
        ok: caps.macroRuntime,
        detail: caps.macroRuntime ? tr("可用") : tr("不可用"),
      },
      { label: tr("RC003 helper"), ok: helper, detail: helper ? tr("已安装") : tr("未安装") },
      { label: tr("音频输入设备"), ok: endpoints.length > 0, detail: `${endpoints.length}` },
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

onMounted(async () => {
  await Promise.all([load(), loadVoiceToolCatalog()]);
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

        <template v-if="!loading && sectionVisible('audio')">
          <p class="settings-group-label">{{ tr("音频") }}</p>
          <Card v-if="settingsDocument">
            <div class="settings-rows">
              <SettingRow label="增益" hint="语音播放的音量增益">
                <div class="range-control">
                  <RangeInput v-model="gain" :min="0" :max="24" :step="1" />
                  <span class="range-value mono">{{ gain }} dB</span>
                </div>
              </SettingRow>

              <SettingRow label="音频输出设备" hint="语音播放到的设备；未设置时连接后自动选 CABLE">
                <SelectInput v-model="endpointId">
                  <option value="">{{ tr("未设置（连接时自动选 CABLE）") }}</option>
                  <option v-for="endpoint in endpoints" :key="endpoint.id" :value="endpoint.id">
                    {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
                  </option>
                </SelectInput>
              </SettingRow>
            </div>
          </Card>

          <p class="settings-group-label">{{ tr("语音") }}</p>
          <Card v-if="settingsDocument">
            <div class="settings-rows">
              <SettingRow label="输入法" hint="遥控器语音转写注入的目标输入法">
                <SelectInput v-model="voiceInputTool">
                  <option value="">{{ tr("未选择") }}</option>
                  <option
                    v-for="tool in voiceToolOptions(voiceToolCatalog, voiceInputTool)"
                    :key="tool.id"
                    :value="tool.id"
                  >
                    {{ tool.label }}
                  </option>
                </SelectInput>
              </SettingRow>

              <SettingRow label="语音热键" hint="按下遥控器语音键时注入的组合键">
                <div class="hotkey-inline">
                  <kbd class="hotkey-value" :class="{ recording }" aria-live="polite">
                    <span v-if="recording" class="hotkey-recording-dot" aria-hidden="true" />
                    {{ hotkeyLabel }}
                  </kbd>
                  <IconButton
                    :label="trDynamic(recording ? '取消录制' : '录制热键')"
                    @click="toggleHotkey"
                  >
                    <Keyboard :size="15" />
                  </IconButton>
                  <IconButton v-if="hasCustom" :label="trDynamic('恢复默认')" @click="resetHotkey">
                    <RotateCcw :size="15" />
                  </IconButton>
                </div>
              </SettingRow>

              <SettingRow label="语音记录" hint="只保存转写文字，不保存音频。">
                <Switch v-model="transcriptHistory" :label="trDynamic('语音记录')" />
              </SettingRow>
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
                hint="检查平台、RC003 增强服务、音频、连接、电量和前台应用识别"
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

.hotkey-value {
  display: inline-flex;
  flex: 1 1 auto;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  min-width: 0;
  min-height: 34px;
  overflow: hidden;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  padding: 0 var(--space-2);
  color: var(--text);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  text-overflow: ellipsis;
  white-space: nowrap;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.hotkey-value.recording {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

.hotkey-recording-dot {
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--accent);
  animation: hotkey-pulse 1.2s ease-in-out infinite;
}

@keyframes hotkey-pulse {
  50% {
    opacity: 0.25;
  }
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
  grid-template-columns: minmax(0, 1fr) auto minmax(0, 180px);
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
  justify-self: end;
  text-align: right;
  overflow-wrap: anywhere;
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
