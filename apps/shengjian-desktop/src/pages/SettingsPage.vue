<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { ChevronDown, ChevronUp, Download, FolderOpen, RotateCcw, Upload } from "lucide-vue-next";
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
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
import Field from "../components/ui/Field.vue";
import LoadingState from "../components/ui/LoadingState.vue";
import Notice from "../components/ui/Notice.vue";
import RangeInput from "../components/ui/RangeInput.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import SelectInput from "../components/ui/SelectInput.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import Switch from "../components/ui/Switch.vue";
import Tabs from "../components/ui/Tabs.vue";
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

const SECTIONS: { id: SectionId; label: string }[] = [
  { id: "appearance", label: "外观与语言" },
  { id: "audio", label: "音频" },
  { id: "startup", label: "启动与窗口" },
  { id: "diagnostics", label: "数据与诊断" },
  { id: "advanced", label: "高级" },
];

const store = useEngineStore();
const statusStore = useStatusStore();
const settingsDocument = ref<SettingsDocument | null>(null);
const endpoints = ref<AudioEndpoint[]>([]);
const gain = ref(0);
const endpointId = ref("");
const autostart = ref(false);
const closeToTray = ref(true);
const theme = ref("system");
const language = ref("system");
const accent = ref(localStorage.getItem(ACCENT_KEY) ?? "default");
const backupBusy = ref(false);
const backupMessage = ref<string | null>(null);
const diagnosticText = ref<string | null>(null);
const diagnosticBusy = ref(false);
const logOpen = ref(false);
const healthChecks = ref<HealthCheck[]>([]);
const healthBusy = ref(false);
const activeSection = ref<SectionId>("appearance");
const busy = ref(false);
const loading = ref(true);
const error = ref<string | null>(null);
let saveTimer: number | undefined;
let loadingSettings = false;

function sectionVisible(id: SectionId): boolean {
  return activeSection.value === id;
}

const sectionModel = computed({
  get: () => activeSection.value,
  set: (value: string) => {
    activeSection.value = value as SectionId;
  },
});

const accentModel = computed({
  get: () => accent.value,
  set: (value: string) => changeAccent(value),
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
    gain.value = settings.global.gain_db;
    endpointId.value = settings.global.audio_endpoint_id ?? "";
    autostart.value = await getAutostartEnabled();
    closeToTray.value = settings.global.close_to_tray ?? true;
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
        close_to_tray: closeToTray.value,
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

watch([gain, endpointId, theme, language, autostart, closeToTray], scheduleAutoSave);

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
        detail: String(currentSettings.global.voice_input_tool ?? tr("未选择")),
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
    await exportConfiguration(
      path,
      accent.value,
      localStorage.getItem("shengjian.last-remote"),
    );
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
      "恢复完成：{profiles} 个 profile，{macros} 个宏定义，{transcripts} 条转写。",
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
  await load();
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

    <Tabs v-model="sectionModel" :items="SECTIONS" aria-label="设置分区" />

    <div
      class="settings-content stack"
      role="tabpanel"
      :id="`panel-${activeSection}`"
      :aria-labelledby="`tab-${activeSection}`"
      :aria-busy="loading || undefined"
    >
      <LoadingState v-if="loading" label="正在加载…" description="正在读取本地设置…" />

      <template v-if="!loading && sectionVisible('appearance')">
        <Card v-if="settingsDocument">
          <SectionHeader title="外观与语言" hint="界面主题、强调色与应用语言" />
          <div class="card-body settings-form">
            <Field label="主题">
              <SelectInput v-model="theme">
                <option value="system">{{ tr("跟随系统") }}</option>
                <option value="light">{{ tr("浅色") }}</option>
                <option value="dark">{{ tr("深色") }}</option>
              </SelectInput>
            </Field>

            <Field label="强调色">
              <SelectInput v-model="accentModel">
                <option value="default">{{ tr("默认") }}</option>
                <option value="blue">{{ tr("蓝色") }}</option>
                <option value="purple">{{ tr("紫色") }}</option>
                <option value="orange">{{ tr("橙色") }}</option>
              </SelectInput>
            </Field>

            <Field label="应用语言">
              <SelectInput v-model="languageModel">
                <option value="system">{{ tr("跟随系统") }}</option>
                <option value="zh-Hans">{{ tr("中文") }}</option>
                <option value="en">{{ tr("English") }}</option>
              </SelectInput>
            </Field>
          </div>
        </Card>
      </template>

      <template v-if="!loading && sectionVisible('audio')">
        <Card v-if="settingsDocument">
          <SectionHeader title="音频" hint="全局设置" />
          <div class="card-body settings-form">
            <Field label="增益">
              <div class="range-control">
                <RangeInput v-model="gain" :min="0" :max="24" :step="1" />
                <span class="range-value mono">{{ gain }} dB</span>
              </div>
            </Field>

            <Field label="音频输入设备">
              <SelectInput v-model="endpointId">
                <option value="">{{ tr("未设置（连接时自动选 CABLE）") }}</option>
                <option v-for="endpoint in endpoints" :key="endpoint.id" :value="endpoint.id">
                  {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
                </option>
              </SelectInput>
            </Field>
          </div>
        </Card>
      </template>

      <template v-if="!loading && sectionVisible('startup')">
        <Card v-if="settingsDocument">
          <SectionHeader title="启动与窗口" hint="系统启动与关闭行为" />
          <div class="card-body settings-form">
            <div class="setting-toggle">
              <div class="setting-toggle-copy">
                <span class="setting-toggle-label">{{ tr("开机自动启动") }}</span>
              </div>
              <Switch v-model="autostart" :label="trDynamic('开机自动启动')" />
            </div>

            <div class="setting-toggle">
              <div class="setting-toggle-copy">
                <span class="setting-toggle-label">
                  {{ tr("关闭窗口时隐藏到托盘（关闭则直接退出）") }}
                </span>
              </div>
              <Switch
                v-model="closeToTray"
                :label="trDynamic('关闭窗口时隐藏到托盘（关闭则直接退出）')"
              />
            </div>
          </div>
        </Card>
      </template>

      <template v-if="!loading && sectionVisible('diagnostics')">
        <Card>
          <SectionHeader title="配置备份" hint="单 JSON 备份" />
          <div class="card-body">
            <p class="muted">
              {{ tr("导出为单个 JSON，用于迁移或故障恢复；恢复会覆盖当前配置。") }}
            </p>
            <p class="muted">
              {{ tr("备份包含 设置、方案、宏库、转写历史、强调色和上次设备。") }}
            </p>
            <div class="footer-actions">
              <Button :disabled="backupBusy" @click="exportBackup">
                <Download :size="16" />
                {{ tr("导出备份") }}
              </Button>
              <Button :disabled="backupBusy" @click="importBackup">
                <Upload :size="16" />
                {{ tr("恢复备份") }}
              </Button>
            </div>
            <p v-if="backupMessage" class="muted">{{ backupMessage }}</p>
          </div>
        </Card>

        <Card>
          <SectionHeader title="诊断" hint="不包含语音内容" />
          <div class="card-body">
            <p class="muted">
              {{ tr("诊断日志位于本机 `%LOCALAPPDATA%\\Shengjian\\Logs`。") }}
            </p>
            <div class="footer-actions">
              <Button @click="openLogs">
                <FolderOpen :size="16" />
                {{ tr("打开日志目录") }}
              </Button>
              <Button :disabled="diagnosticBusy" @click="loadDiagnostics">
                {{ trDynamic(diagnosticBusy ? "读取中" : "查看日志") }}
              </Button>
              <Button :disabled="diagnosticBusy" @click="exportDiagnostics">
                {{ tr("导出诊断包") }}
              </Button>
            </div>
            <div v-if="diagnosticText !== null" class="log-block">
              <div class="row-between">
                <span class="muted">{{ tr("日志预览") }}</span>
                <Button variant="ghost" size="sm" @click="logOpen = !logOpen">
                  <component :is="logOpen ? ChevronUp : ChevronDown" :size="14" />
                  {{ trDynamic(logOpen ? "收起日志" : "展开日志") }}
                </Button>
              </div>
              <pre v-show="logOpen" class="log-preview">{{ diagnosticText || tr("日志为空") }}</pre>
            </div>
          </div>
        </Card>

        <Card>
          <SectionHeader title="健康检查" hint="本机运行状态" />
          <div class="card-body">
            <p class="muted">
              {{ tr("检查平台、RC003增强服务、音频、连接、电量和前台应用识别。") }}
            </p>
            <div class="footer-actions">
              <Button :disabled="healthBusy" @click="runHealthCheck">
                {{ trDynamic(healthBusy ? "检查中" : "运行健康检查") }}
              </Button>
            </div>
            <ul v-if="healthChecks.length" class="health-list">
              <li v-for="check in healthChecks" :key="check.label" class="health-row">
                <span class="health-label">{{ check.label }}</span>
                <StatusPill
                  :label="trDynamic(check.ok ? '可用' : '不可用')"
                  :tone="check.ok ? 'success' : 'warning'"
                  dot
                />
                <span class="mono muted health-detail">{{ check.detail }}</span>
              </li>
            </ul>
          </div>
        </Card>
      </template>

      <template v-if="!loading && sectionVisible('advanced')">
        <Card>
          <SectionHeader title="高级" hint="首次配置与维护操作" />
          <div class="card-body">
            <p class="muted">
              {{ tr("重新运行首次配置向导，可重新检查遥控器连接与语音链路。") }}
            </p>
            <div class="footer-actions">
              <Button @click="restartOnboarding">
                <RotateCcw :size="16" />
                {{ tr("重新运行首次配置") }}
              </Button>
            </div>
          </div>
        </Card>

        <Card class="danger-card">
          <SectionHeader title="危险区域" hint="以下操作不可恢复" />
          <div class="card-body">
            <p class="muted">
              {{ tr("恢复默认设置会清除全局设置和设备覆盖，不影响 profile、宏和转写历史。") }}
            </p>
            <div class="footer-actions">
              <Button variant="danger" :disabled="busy" @click="resetAllSettings">
                {{ tr("恢复默认设置") }}
              </Button>
              <span class="muted">{{ tr("更改会即时保存") }}</span>
            </div>
          </div>
        </Card>
      </template>
    </div>
  </section>
</template>

<style scoped>
.settings-content {
  min-width: 0;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  max-width: 720px;
  margin-top: var(--space-4);
  min-width: 0;
}

.settings-form {
  max-width: 640px;
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

@media (max-width: 900px) {
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
