<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { Bluetooth, RefreshCw } from "lucide-vue-next";
import {
  errorMessage,
  getRc003HelperInstalled,
  getSettings,
  listAudioEndpoints,
  saveSettings,
} from "../lib/api";
import { statusLabel, tr, trDynamic, uiLocale } from "../lib/i18n";
import { voiceToolOptions } from "../lib/voiceTools";
import { useVoiceToolCatalog } from "../composables/useVoiceToolCatalog";
import {
  deviceAction,
  findLastRemote,
  type DeviceAction,
  type PendingDeviceOperation,
} from "../lib/connection";
import { deviceProductNameKey } from "../lib/deviceProduct";
import { supportsKeyEnhancement } from "../lib/keyEnhancement";
import { deviceLineArt } from "../lib/deviceArt";
import type { AudioEndpoint, SettingsDocument } from "../lib/types";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
import Checkbox from "../components/ui/Checkbox.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import Field from "../components/ui/Field.vue";
import Notice from "../components/ui/Notice.vue";
import RangeInput from "../components/ui/RangeInput.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import SelectInput from "../components/ui/SelectInput.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import { pushToast } from "../lib/toast";
import { useEngineStore } from "../stores/engine";
import { useStatusStore } from "../stores/status";

const store = useEngineStore();
const statusStore = useStatusStore();
const battery = computed(() => statusStore.batteryLevel);
const deviceStatus = computed(() => statusStore.deviceStatus);
const settingsError = shallowRef<string | null>(null);
const endpoints = ref<AudioEndpoint[]>([]);
const useDeviceOverride = shallowRef(false);
const deviceGain = shallowRef(0);
const deviceEndpointId = shallowRef("");
const deviceVoiceInputTool = shallowRef("");
const { catalog: voiceToolCatalog, load: loadVoiceToolCatalog } = useVoiceToolCatalog();
const rc003Capture = shallowRef(false);
const rc003HelperInstalled = shallowRef(false);
const settingsDocument = ref<SettingsDocument | null>(null);
const settingsDeviceId = shallowRef<string | null>(null);
let saveTimer: number | undefined;
let helperPoll: number | undefined;
let loadingSettings = false;
const lowBattery = computed(() => battery.value !== null && battery.value <= 20);
const lastRemoteKey = "shengjian.last-remote";
const autoConnectAttemptedKey = "shengjian.auto-connect-attempted";
const lastRemoteId = ref(localStorage.getItem(lastRemoteKey) ?? "");
const currentRemoteId = computed(() => store.snapshot?.connection.remote?.remoteId ?? null);
const deviceId = computed(() => store.snapshot?.connection.remote?.deviceId ?? null);
const deviceName = computed(() => store.snapshot?.connection.remote?.name ?? tr("未连接设备"));
/** 三款小米遥控器都支持按键增强服务。 */
const supportsKeyEnhancementCapture = computed(
  () => deviceId.value !== null && supportsKeyEnhancement(remoteModel.value),
);
const remoteModel = computed(() => store.snapshot?.connection.remote?.model ?? null);
const deviceProductName = computed(() => {
  const key = deviceProductNameKey(remoteModel.value);
  return key ? trDynamic(key) : deviceName.value;
});
/** 型号 → 产品线稿只在 `lib/deviceArt` 里维护一份，导入/导出弹窗共用同一套资源。 */
const deviceArtwork = computed(() => (store.isConnected ? deviceLineArt(remoteModel.value) : null));
const autoConnecting = shallowRef(false);
const autoConnectFailed = shallowRef(false);
const connectionError = computed(() => (store.error ? errorMessage(store.error) : null));
const phaseFailure = computed(() => {
  const phase = String(store.phase);
  return phase === "failed" || phase === "error";
});
const hasConnectionNotices = computed(
  () =>
    autoConnecting.value ||
    autoConnectFailed.value ||
    lowBattery.value ||
    Boolean(connectionError.value) ||
    phaseFailure.value ||
    Boolean(settingsError.value),
);

function isCurrent(remoteId: string): boolean {
  return store.isConnected && currentRemoteId.value === remoteId;
}

/**
 * 正在进行中的连接操作落在哪台设备上、是连还是断。
 *
 * `store.connecting` 只是「有连接操作在进行」的全局标记，不能拿它给每一行上
 * 文案——否则点一台设备时，列表里所有其他设备的按钮都会变成「连接中」。
 */
const pendingOperation = shallowRef<PendingDeviceOperation | null>(null);

/** 四种动作的双语文案；判定逻辑在 lib/connection.ts 的 deviceAction 里，已单测覆盖。 */
const DEVICE_ACTION_TEXT: Record<DeviceAction, { zh: string; en: string }> = {
  connect: { zh: "连接", en: "Connect" },
  disconnect: { zh: "断开", en: "Disconnect" },
  connecting: { zh: "连接中", en: "Connecting" },
  disconnecting: { zh: "断开中", en: "Disconnecting" },
};

function deviceActionLabel(remoteId: string): string {
  const action = deviceAction({
    remoteId,
    isCurrent: isCurrent(remoteId),
    connecting: store.connecting,
    pending: pendingOperation.value,
  });
  const text = DEVICE_ACTION_TEXT[action];
  return uiLocale.value === "en" ? text.en : text.zh;
}

async function toggleDevice(remoteId: string) {
  if (isCurrent(remoteId)) {
    await disconnect();
  } else {
    await connect(remoteId);
  }
}

watch(lowBattery, (low) => {
  if (low && battery.value !== null) {
    pushToast(tr("遥控器电量低：{level}%", { level: battery.value }), "error");
  }
});

function applyDeviceOverride(settings: SettingsDocument) {
  const id = deviceId.value;
  const override = id ? settings.devices[id]?.windows : undefined;
  useDeviceOverride.value = override !== undefined;
  deviceGain.value = override?.gainDb ?? Math.round(settings.global.gain_db);
  deviceEndpointId.value = override?.audioEndpointId ?? settings.global.audio_endpoint_id ?? "";
  deviceVoiceInputTool.value = String(override?.voiceInputTool ?? "");
}

function applyTopbarVoiceInputTool() {
  const globalTool = String(settingsDocument.value?.global.voice_input_tool ?? "");
  const overrideTool = useDeviceOverride.value ? deviceVoiceInputTool.value : "";
  statusStore.setVoiceInputTool(overrideTool || globalTool);
}

async function loadDeviceSettings() {
  loadingSettings = true;
  settingsError.value = null;
  try {
    const settings = await getSettings();
    settingsDocument.value = settings;
    settingsDeviceId.value = deviceId.value;
    endpoints.value = await listAudioEndpoints();
    rc003Capture.value = settings.global.rc003_capture_enabled ?? false;
    rc003HelperInstalled.value = await getRc003HelperInstalled();
    applyDeviceOverride(settings);
    applyTopbarVoiceInputTool();
  } catch (cause) {
    settingsError.value = errorMessage(cause);
  } finally {
    loadingSettings = false;
  }
}

async function saveConnectionSettings() {
  const id = deviceId.value ?? settingsDeviceId.value;
  try {
    const latest = await getSettings();
    const devices = JSON.parse(JSON.stringify(latest.devices)) as SettingsDocument["devices"];
    if (id) {
      const platforms = devices[id] ?? {};
      if (useDeviceOverride.value) {
        const override = { ...(platforms.windows ?? {}) };
        override.gainDb = Math.round(deviceGain.value);
        if (deviceEndpointId.value) {
          override.audioEndpointId = deviceEndpointId.value;
        } else {
          delete override.audioEndpointId;
        }
        if (deviceVoiceInputTool.value) {
          override.voiceInputTool = deviceVoiceInputTool.value;
        } else {
          delete override.voiceInputTool;
        }
        platforms.windows = override;
      } else {
        delete platforms.windows;
      }
      if (Object.keys(platforms).length > 0) {
        devices[id] = platforms;
      } else {
        delete devices[id];
      }
    }
    const next: SettingsDocument = {
      ...latest,
      global: {
        ...latest.global,
        rc003_capture_enabled: rc003Capture.value,
      },
      devices,
    };
    await saveSettings(next);
    settingsDocument.value = next;
    applyTopbarVoiceInputTool();
  } catch (cause) {
    settingsError.value = errorMessage(cause);
  }
}

function scheduleConnectionSave() {
  if (loadingSettings) return;
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    void saveConnectionSettings();
  }, 250);
}

watch([rc003Capture, deviceGain, deviceEndpointId], scheduleConnectionSave);

watch(useDeviceOverride, () => {
  if (loadingSettings) return;
  applyTopbarVoiceInputTool();
  void saveConnectionSettings();
});

watch(deviceVoiceInputTool, () => {
  if (loadingSettings) return;
  applyTopbarVoiceInputTool();
  void saveConnectionSettings();
});

watch(deviceId, () => {
  window.clearTimeout(saveTimer);
  if (deviceId.value) {
    void loadDeviceSettings();
  } else {
    statusStore.setVoiceInputTool(String(settingsDocument.value?.global.voice_input_tool ?? ""));
  }
});

async function refresh() {
  autoConnectFailed.value = false;
  await store.scan();
  await statusStore.refreshDeviceStatus();
  // A failed settings load is part of the page's failure state; recover it from
  // the same button so the primary action always clears the error it shows.
  if (settingsError.value) {
    await loadDeviceSettings();
  }
}

async function connect(remoteId: string) {
  // 整个操作（含先断开旧设备）都算在这一行上，用户点了哪台就哪台显示「连接中」。
  pendingOperation.value = { remoteId, kind: "connect" };
  try {
    if (store.isConnected && currentRemoteId.value !== remoteId) {
      await store.disconnect();
    }
    await store.connect(remoteId);
    lastRemoteId.value = remoteId;
    localStorage.setItem(lastRemoteKey, remoteId);
    await statusStore.refreshDeviceStatus();
  } finally {
    pendingOperation.value = null;
  }
}

async function disconnect() {
  const remoteId = currentRemoteId.value;
  if (remoteId) pendingOperation.value = { remoteId, kind: "disconnect" };
  try {
    await store.disconnect();
    statusStore.clearDeviceStatus();
  } finally {
    pendingOperation.value = null;
  }
}

function modelLabel(model: string | null): string {
  return model ?? tr("未知");
}

function remoteLabel(remote: { name: string; model: string | null }): string {
  const key = deviceProductNameKey(remote.model);
  return key ? trDynamic(key) : remote.name;
}

async function autoConnectLast() {
  if (
    sessionStorage.getItem(autoConnectAttemptedKey) === "1" ||
    !lastRemoteId.value ||
    store.isConnected ||
    autoConnecting.value
  ) {
    return;
  }
  sessionStorage.setItem(autoConnectAttemptedKey, "1");
  autoConnecting.value = true;
  autoConnectFailed.value = false;
  try {
    if (store.remotes.length === 0) {
      await store.scan();
    }
    const remote = findLastRemote(store.remotes, lastRemoteId.value);
    if (remote) {
      await connect(remote.remoteId);
    } else {
      autoConnectFailed.value = true;
    }
  } catch {
    autoConnectFailed.value = true;
  } finally {
    autoConnecting.value = false;
  }
}

onMounted(async () => {
  await Promise.all([
    statusStore.refreshDeviceStatus(),
    loadDeviceSettings(),
    loadVoiceToolCatalog(),
  ]);
  await autoConnectLast();
});

onUnmounted(() => {
  window.clearTimeout(saveTimer);
  window.clearInterval(helperPoll);
});
</script>

<template>
  <section class="page">
    <PageHeader eyebrow="Connection" title="连接遥控器" description="管理遥控器连接与设备状态。">
      <template #actions>
        <Button variant="primary" :disabled="store.scanning" @click="refresh">
          <RefreshCw :size="16" :class="{ spin: store.scanning }" />
          {{ trDynamic(store.scanning ? "扫描中" : "扫描设备") }}
        </Button>
      </template>
    </PageHeader>

    <div v-if="hasConnectionNotices" class="stack stack-sm">
      <Notice v-if="autoConnecting" kind="info">
        {{ tr("正在自动连接上次使用的遥控器…") }}
      </Notice>
      <Notice v-if="autoConnectFailed" kind="warning">
        {{ tr("未找到上次使用的遥控器，请手动扫描并连接。") }}
      </Notice>
      <Notice v-if="lowBattery" kind="warning">
        {{ tr("遥控器电量低：{level}%，建议及时充电。", { level: battery ?? 0 }) }}
      </Notice>
      <Notice v-if="connectionError" kind="error">{{ connectionError }}</Notice>
      <Notice v-else-if="phaseFailure" kind="error">{{ tr("连接失败，请重试") }}</Notice>
      <Notice v-if="settingsError" kind="error">{{ settingsError }}</Notice>
    </div>

    <Card class="stack">
      <SectionHeader title="设备状态" />
      <div class="device-status-body">
        <div class="device-visual" :class="{ offline: !store.isConnected }">
          <img
            v-if="deviceArtwork"
            :src="deviceArtwork"
            :alt="store.snapshot?.connection.remote?.name ?? tr('遥控器')"
            class="device-image is-line"
            draggable="false"
          />
          <svg
            v-else
            viewBox="0 0 120 240"
            class="device-line-art"
            role="presentation"
            aria-hidden="true"
          >
            <rect x="17" y="8" width="86" height="224" rx="30" class="remote-body" />
            <rect x="29" y="22" width="62" height="196" rx="22" class="remote-inner" />
            <circle cx="60" cy="47" r="13" class="remote-accent" />
            <rect x="57" y="41" width="6" height="9" rx="3" class="remote-ink" />
            <path d="M52 47a8 8 0 0 0 16 0M60 55v5" class="remote-line" />
            <circle cx="60" cy="112" r="36" class="remote-pad" />
            <circle cx="60" cy="112" r="14" class="remote-accent" />
            <path
              d="M60 96l5 6H55zM60 128l5-6H55zM44 112l6-5v10zM76 112l-6-5v10z"
              class="remote-ink"
            />
            <circle cx="36" cy="176" r="9" class="remote-key" />
            <circle cx="60" cy="176" r="9" class="remote-key" />
            <circle cx="84" cy="176" r="9" class="remote-key" />
            <rect x="46" y="198" width="28" height="10" rx="5" class="remote-key" />
          </svg>
        </div>
        <div class="device-summary">
          <div class="device-identity">
            <p class="metric-label">{{ tr("遥控器") }}</p>
            <p class="device-product-name">
              {{ store.isConnected ? deviceProductName : tr("未连接") }}
            </p>
          </div>
          <div class="metric-grid device-metrics">
            <div class="metric">
              <p class="metric-label">{{ tr("型号") }}</p>
              <p class="metric-value">
                {{ modelLabel(store.snapshot?.connection.remote?.model ?? null) }}
              </p>
            </div>
            <div class="metric">
              <p class="metric-label">{{ tr("固件版本") }}</p>
              <p class="metric-value">
                {{ store.isConnected ? (deviceStatus?.firmwareVersion ?? tr("未知")) : tr("未知") }}
              </p>
            </div>
            <div class="metric">
              <p class="metric-label">{{ tr("电量") }}</p>
              <p class="metric-value" :class="{ 'metric-warning': lowBattery }">
                {{ battery === null ? tr("未知") : `${battery}%` }}
              </p>
            </div>
            <div class="metric">
              <p class="metric-label">{{ tr("重连次数") }}</p>
              <p class="metric-value">{{ deviceStatus?.reconnectAttempt ?? 0 }}</p>
            </div>
            <div class="metric">
              <p class="metric-label">{{ tr("电源通知") }}</p>
              <p class="metric-value">
                {{
                  store.isConnected && deviceStatus
                    ? trDynamic(deviceStatus.powerNotificationsAvailable ? "可用" : "不可用")
                    : tr("未知")
                }}
              </p>
            </div>
          </div>
        </div>
      </div>
    </Card>

    <Card class="stack">
      <SectionHeader title="发现的设备" hint="扫描结果，可能包含未适配型号。">
        <template #actions>
          <span class="muted">{{ store.remotes.length }} {{ tr("台") }}</span>
        </template>
      </SectionHeader>

      <EmptyState
        v-if="store.remotes.length === 0"
        title="还没有扫描结果"
        description="点击“扫描设备”开始扫描附近的遥控器。"
      >
        <template #icon>
          <Bluetooth :size="20" />
        </template>
        <template #actions>
          <Button variant="secondary" :disabled="store.scanning" @click="refresh">
            <RefreshCw :size="15" :class="{ spin: store.scanning }" />
            {{ trDynamic(store.scanning ? "扫描中" : "扫描设备") }}
          </Button>
        </template>
      </EmptyState>

      <ul v-else class="device-list">
        <li
          v-for="remote in store.remotes"
          :key="remote.remoteId"
          class="device-row"
          :class="{ current: isCurrent(remote.remoteId) }"
        >
          <span class="device-icon"><Bluetooth :size="17" /></span>
          <span class="device-main">
            <span class="device-head">
              <span class="device-name">{{ remoteLabel(remote) }}</span>
              <StatusPill
                v-if="isCurrent(remote.remoteId)"
                :label="trDynamic('已连接')"
                tone="success"
                dot
              />
              <StatusPill
                :label="statusLabel(remote.isSupported ? 'supported' : 'unsupported')"
                :tone="remote.isSupported ? 'success' : 'neutral'"
              />
              <StatusPill
                v-if="remote.remoteId === lastRemoteId"
                :label="trDynamic('上次使用')"
                tone="accent"
              />
            </span>
            <span class="device-meta muted">
              <span>{{ tr("型号") }}={{ modelLabel(remote.model) }}</span>
              <template v-if="isCurrent(remote.remoteId)">
                <span>
                  {{ tr("固件版本") }}={{ deviceStatus?.firmwareVersion ?? tr("未知") }}
                </span>
                <span>{{ tr("电量") }}={{ battery === null ? tr("未知") : `${battery}%` }}</span>
              </template>
            </span>
          </span>
          <Button
            class="device-action"
            size="sm"
            :variant="isCurrent(remote.remoteId) ? 'secondary' : 'primary'"
            :disabled="store.connecting"
            @click="toggleDevice(remote.remoteId)"
          >
            {{ deviceActionLabel(remote.remoteId) }}
          </Button>
        </li>
      </ul>
    </Card>

    <!-- Only offer per-device overrides while a device is actually connected. -->
    <Card v-if="store.isConnected && deviceId" class="stack">
      <SectionHeader title="当前设备覆盖" :hint="deviceProductName" />
      <Checkbox v-model="useDeviceOverride">
        {{ tr("为此设备使用独立音频设置") }}
      </Checkbox>
      <template v-if="useDeviceOverride">
        <Field label="设备增益">
          <div class="gain-row">
            <RangeInput v-model="deviceGain" :min="0" :max="24" :step="1" />
            <span class="gain-value mono muted">{{ deviceGain }} dB</span>
          </div>
        </Field>
        <Field label="语音输出设备">
          <SelectInput v-model="deviceEndpointId">
            <option value="">{{ tr("继承全局端点") }}</option>
            <option v-for="endpoint in endpoints" :key="endpoint.id" :value="endpoint.id">
              {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
            </option>
          </SelectInput>
        </Field>
        <Field label="语音输入工具">
          <SelectInput v-model="deviceVoiceInputTool">
            <option value="">{{ tr("继承全局工具") }}</option>
            <option
              v-for="tool in voiceToolOptions(voiceToolCatalog, deviceVoiceInputTool)"
              :key="tool.id"
              :value="tool.id"
            >
              {{ tool.label }}
            </option>
          </SelectInput>
        </Field>
      </template>
    </Card>

    <Card v-if="supportsKeyEnhancementCapture" class="stack">
      <SectionHeader title="增强捕获" hint="返回键、音量键" />
      <Checkbox v-model="rc003Capture">
        {{ tr("启用按键增强") }}
      </Checkbox>

      <p class="helper-description">
        {{
          tr("返回键、音量键需要按键增强服务；安装、更新与卸载在「设置 → 音频及语音」中统一管理。")
        }}
      </p>
    </Card>
  </section>
</template>

<style scoped>
.device-status-body {
  display: flex;
  align-items: center;
  gap: var(--space-5);
  min-width: 0;
}

.device-visual {
  display: grid;
  flex: 0 0 96px;
  width: 96px;
  height: 192px;
  place-items: center;
}

.device-visual.offline {
  opacity: 0.72;
}

.device-image {
  width: auto;
  height: 188px;
  max-width: 100%;
  object-fit: contain;
  filter: drop-shadow(var(--shadow-sm));
}

.device-image.is-line {
  filter: invert(var(--line-art-invert)) drop-shadow(var(--shadow-sm));
}

.device-line-art {
  width: 88px;
  height: 176px;
  filter: drop-shadow(var(--shadow-sm));
}

.remote-body {
  fill: var(--bg-sidebar);
  stroke: var(--border-strong);
  stroke-width: 2;
}

.remote-inner {
  fill: var(--bg-raised);
}

.remote-accent {
  fill: var(--accent-soft-strong);
  stroke: var(--accent);
  stroke-width: 1.5;
}

.remote-ink {
  fill: var(--text-muted);
}

.remote-line {
  fill: none;
  stroke: var(--text-muted);
  stroke-width: 1.6;
  stroke-linecap: round;
}

.remote-pad {
  fill: var(--bg-subtle);
  stroke: var(--border-strong);
  stroke-width: 1.5;
}

.remote-key {
  fill: var(--bg-subtle);
  stroke: var(--border);
  stroke-width: 1.2;
}

.device-summary {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}

.device-identity {
  min-width: 0;
}

.device-product-name {
  margin-top: 2px;
  font-size: var(--text-lg);
  font-weight: 650;
  line-height: 1.3;
  overflow-wrap: anywhere;
}

.device-metrics {
  flex: none;
  width: 100%;
  min-width: 0;
}

.metric-warning {
  color: var(--warning-text);
}

.gain-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  min-width: 0;
}

.gain-value {
  flex: 0 0 auto;
  white-space: nowrap;
}

.device-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
}

.device-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  align-items: center;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-3);
}

.device-row.current {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.device-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border-radius: var(--radius-md);
  background: var(--accent-soft);
  color: var(--accent);
}

.device-main {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.device-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.device-name {
  font-weight: 600;
  overflow-wrap: anywhere;
}

.device-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 2px 12px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.device-action {
  justify-self: end;
}

.helper-panel {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-4);
}

.helper-actions {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  gap: var(--space-2);
}

.helper-main {
  display: flex;
  flex: 1 1 320px;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.helper-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.helper-name {
  font-size: var(--text-sm);
  font-weight: 650;
}

.helper-description {
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.6;
}

/* Layout stays multi-column down to the app minimum window width (1200px);
   the content area is ~960px there once the 208px sidebar is subtracted.
   These rules only apply below the supported minimum, as a safety net. */
@media (max-width: 1100px) {
  .device-status-body {
    flex-direction: column;
    align-items: flex-start;
  }

  .device-visual {
    flex-basis: 88px;
    width: 88px;
    height: 176px;
  }

  .device-image {
    height: 168px;
    width: auto;
  }

  .device-line-art {
    width: 78px;
    height: 156px;
  }

  .device-row {
    grid-template-columns: auto minmax(0, 1fr);
  }

  .device-action {
    grid-column: 1 / -1;
    justify-self: stretch;
  }

  .helper-panel {
    flex-direction: column;
    align-items: stretch;
  }
}
</style>
