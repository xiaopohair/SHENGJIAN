<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import {
  BatteryMedium,
  Bluetooth,
  CircleHelp,
  Clock3,
  Cpu,
  Mic,
  MoreVertical,
  RefreshCw,
  SlidersHorizontal,
  Trash2,
  Volume2,
  Wrench,
} from "lucide-vue-next";
import {
  errorMessage,
  forgetRemote,
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
import { usePairingStore } from "../stores/pairing";
import { deviceSettingsRequest, takeDeviceSettingsRequest } from "../lib/navigation";
import {
  applyRememberedModels,
  forgetDeviceRecord,
  loadDeviceRecords,
  markDeviceConnected,
} from "../lib/deviceMemory";
import { formatRelativeTime } from "../lib/relativeTime";
import { effectiveGainDb } from "../lib/effectiveAudio";
import { supportsKeyEnhancement } from "../lib/keyEnhancement";
import { deviceLineArt } from "../lib/deviceArt";
import type { AudioEndpoint, SettingsDocument } from "../lib/types";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import IconButton from "../components/ui/IconButton.vue";
import Card from "../components/ui/Card.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import DeviceSettingsDrawer from "../components/DeviceSettingsDrawer.vue";
import Notice from "../components/ui/Notice.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import { pushToast } from "../lib/toast";
import { confirmAction } from "../lib/confirm";
import { useEngineStore } from "../stores/engine";
import { useStatusStore } from "../stores/status";

const store = useEngineStore();
const statusStore = useStatusStore();
const pairing = usePairingStore();
const battery = computed(() => statusStore.batteryLevel);
const deviceStatus = computed(() => statusStore.deviceStatus);
const settingsError = shallowRef<string | null>(null);
const endpoints = ref<AudioEndpoint[]>([]);
const useDeviceOverride = shallowRef(false);
const deviceGain = shallowRef(0);
const deviceEndpointId = shallowRef("");
const deviceVoiceInputTool = shallowRef("");
/** 用户给当前遥控器起的名字；空串表示沿用广播名。 */
const deviceNickname = shallowRef("");
/** 本次连接开始的时刻，用于「已连接 N 分钟」。 */
const connectedAt = shallowRef<number | null>(null);
const nowTick = shallowRef(Date.now());

const { catalog: voiceToolCatalog, load: loadVoiceToolCatalog } = useVoiceToolCatalog();
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
/** 连接过的设备档案（令牌 → 型号 + 最近连接时间）：扫描后回填，同名设备才分得清。 */
const deviceRecords = shallowRef(loadDeviceRecords());
const displayRemotes = computed(() => applyRememberedModels(store.remotes, deviceRecords.value));
const currentRemoteId = computed(() => store.snapshot?.connection.remote?.remoteId ?? null);
const deviceId = computed(() => store.snapshot?.connection.remote?.deviceId ?? null);
const deviceName = computed(() => store.snapshot?.connection.remote?.name ?? tr("未连接设备"));
const remoteModel = computed(() => store.snapshot?.connection.remote?.model ?? null);

// 连上就记下"这支设备是什么型号、什么时候连的"，断开后重扫还能标出来。
watch([currentRemoteId, remoteModel], ([id, model]) => {
  if (!store.isConnected || !id) return;
  deviceRecords.value = markDeviceConnected(id, model);
});
/** 卡片标题：用户起的名字优先，否则退回广播名。 */
const displayDeviceName = computed(() => deviceNickname.value.trim() || deviceName.value);
/** 已连接时长（分钟）；刚连上显示 0 分钟。 */
const connectedMinutes = computed(() => {
  if (!connectedAt.value) return 0;
  return Math.max(0, Math.floor((nowTick.value - connectedAt.value) / 60000));
});
/** 副标题固定为「产品正式名 · 型号」，型号代码前永远有产品名字。 */
const heroSubtitle = computed(
  () => `${deviceProductName.value} · ${modelLabel(remoteModel.value)}`,
);
/** 语音胶囊：设备覆盖优先，否则用全局工具名。 */
const voiceChipLabel = computed(() => {
  const toolId =
    (useDeviceOverride.value ? deviceVoiceInputTool.value : "") ||
    String(settingsDocument.value?.global.voice_input_tool ?? "");
  const label = voiceToolOptions(voiceToolCatalog.value, toolId).find(
    (tool) => tool.id === toolId,
  )?.label;
  return label || tr("未选择");
});
/** 全按键支持服务状态：按最后一次连上的型号判断，断开后也照常显示。 */
const rc003ChipVisible = computed(
  () => supportsKeyEnhancement(lastKnownModel.value) && rc003HelperInstalled.value,
);
const deviceProductName = computed(() => {
  const key = deviceProductNameKey(remoteModel.value);
  return key ? trDynamic(key) : deviceName.value;
});
/**
 * 型号 → 产品线稿只在 `lib/deviceArt` 里维护一份，导入/导出弹窗共用同一套资源。
 *
 * 记住最后一次连上时的型号：断开后继续显示同一张产品图，避免断开瞬间图像跳成
 * 兜底线稿、卡片高度跟着抖一下。
 */
const lastKnownModel = shallowRef<string | null>(null);
// 重启/断线后，主卡仍用上次那台设备的型号来画产品图。
if (lastRemoteId.value) lastKnownModel.value = deviceRecords.value[lastRemoteId.value]?.model ?? null;
watch(
  remoteModel,
  (model) => {
    if (store.isConnected && model) lastKnownModel.value = model;
  },
  { immediate: true },
);
const deviceArtwork = computed(() => deviceLineArt(lastKnownModel.value));
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

/** 列表行的连接状态：已连接 / 相对时间（3 天前、1 分钟前）/ 从未连接。 */
function connectionLabel(remoteId: string): string {
  if (isCurrent(remoteId)) return trDynamic("已连接");
  const record = deviceRecords.value[remoteId];
  // 旧版本只缓存了型号、没存时间：知道它连过，但说不出多久以前。
  if (!record) return tr("从未连接");
  if (!record.lastConnectedAt) return tr("已连接过");
  return formatRelativeTime(record.lastConnectedAt, nowTick.value);
}

function connectionTone(remoteId: string): "success" | "neutral" {
  return isCurrent(remoteId) ? "success" : "neutral";
}

function connectionTitle(remoteId: string): string {
  const record = deviceRecords.value[remoteId];
  if (!record) return trDynamic("从未连接");
  if (!record.lastConnectedAt) return trDynamic("已连接过，但没记下时间");
  return tr("上次连接：{time}", { time: new Date(record.lastConnectedAt).toLocaleString() });
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
    // 电量低是环境预警，不是操作失败（标准：§4.1 的 kind 语义）。
    pushToast(tr("遥控器电量低：{level}%", { level: battery.value }), "warning");
  }
});

// 设备状态里展示"实际生效"的增益：有独立设置时是设备的值，否则是全局值。
const effectiveGain = computed(() => {
  const settings = settingsDocument.value;
  if (!settings) return null;
  // 未连接时没有设备覆盖，展示全局值——与顶栏保持一致，不要整块消失。
  return effectiveGainDb(settings, deviceId.value).value;
});

function applyDeviceOverride(settings: SettingsDocument) {
  const id = deviceId.value;
  const override = id ? settings.devices[id]?.windows : undefined;
  useDeviceOverride.value = override !== undefined;
  deviceGain.value = override?.gainDb ?? Math.round(settings.global.gain_db);
  deviceEndpointId.value = override?.audioEndpointId ?? settings.global.audio_endpoint_id ?? "";
  deviceVoiceInputTool.value = String(override?.voiceInputTool ?? "");
  deviceNickname.value = override?.name ?? "";
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
      const override = { ...(platforms.windows ?? {}) };
      // 昵称是设备属性，跟「独立音频设置」开关无关：关掉音频覆盖也要保留名字。
      const nickname = deviceNickname.value.trim();
      if (nickname) {
        override.name = nickname;
      } else {
        delete override.name;
      }
      if (useDeviceOverride.value) {
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
        // 只撤掉音频覆盖字段，昵称仍留在覆盖对象里。
        delete override.gainDb;
        delete override.audioEndpointId;
        delete override.voiceInputTool;
        if (Object.keys(override).length > 0) {
          platforms.windows = override;
        } else {
          delete platforms.windows;
        }
      }
      if (Object.keys(platforms).length > 0) {
        devices[id] = platforms;
      } else {
        delete devices[id];
      }
    }
    // 全按键支持开关统一在「设置 → 音频及语音」里管理，这里不再改写该字段。
    const next: SettingsDocument = {
      ...latest,
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

watch([deviceGain, deviceEndpointId, deviceNickname], scheduleConnectionSave);

/**
 * 连上时记下时刻、真正断开时清空，每 30 秒刷新一次「已连接 N 分钟」。
 *
 * 不能跟 `isConnected`：core 在已连接状态下也会短暂发 `scanning`（重新扫描、
 * 重连过渡），isConnected 会瞬间变 false，计时器于是被反复清零——现场表现就是
 * 一直显示「0 分钟」。改为跟"当前连的是哪一支"：设备变了才重新计时，
 * 只有真的 `disconnected` 才清零。
 */
const connectedRemoteKey = shallowRef<string | null>(null);
watch(
  () => store.snapshot?.connection.remote?.remoteId ?? null,
  (remoteId) => {
    if (!remoteId || connectedRemoteKey.value === remoteId) return;
    connectedRemoteKey.value = remoteId;
    connectedAt.value = Date.now();
    nowTick.value = Date.now();
  },
  { immediate: true },
);
watch(
  () => String(store.phase),
  (phase) => {
    if (phase !== "disconnected") return;
    connectedRemoteKey.value = null;
    connectedAt.value = null;
  },
);
const minutesTimer = window.setInterval(() => {
  // 断开的设备也要刷新"3 天前"这类相对时间，所以无条件走时。
  nowTick.value = Date.now();
}, 30000);
onUnmounted(() => {
  window.clearInterval(minutesTimer);
  document.removeEventListener("pointerdown", onMorePointerDown, true);
  document.removeEventListener("keydown", onMoreKeydown, true);
});


/** 「更多操作」菜单：属性（设备设置）与忘记设备都收在这里。 */
const moreOpen = shallowRef(false);
const moreRoot = ref<HTMLElement | null>(null);
const moreMenu = ref<HTMLElement | null>(null);
const moreTrigger = ref<InstanceType<typeof IconButton> | null>(null);
/** 菜单用 fixed 定位挂到 body：hero 卡片是 overflow:hidden，直接放里面会被裁掉。 */
const moreMenuStyle = ref<Record<string, string>>({ top: "0px", right: "0px" });

function toggleMore() {
  if (moreOpen.value) {
    closeMore();
    return;
  }
  const element = (moreTrigger.value as unknown as { $el?: HTMLElement })?.$el;
  const rect = element?.getBoundingClientRect();
  if (rect) {
    moreMenuStyle.value = {
      top: `${Math.round(rect.bottom + 6)}px`,
      right: `${Math.round(window.innerWidth - rect.right)}px`,
    };
  }
  moreOpen.value = true;
}

function closeMore(restoreFocus = false) {
  if (!moreOpen.value) return;
  moreOpen.value = false;
  if (restoreFocus) {
    const element = (moreTrigger.value as unknown as { $el?: HTMLElement })?.$el;
    element?.focus();
  }
}

function runMore(action: () => void) {
  closeMore(true);
  action();
}

function onMorePointerDown(event: PointerEvent) {
  if (!moreOpen.value) return;
  const target = event.target as Node | null;
  if (target && (moreRoot.value?.contains(target) || moreMenu.value?.contains(target))) return;
  closeMore();
}

function onMoreScroll() {
  closeMore();
}

function onMoreKeydown(event: KeyboardEvent) {
  if (!moreOpen.value) return;
  if (event.key === "Escape") {
    event.preventDefault();
    closeMore(true);
  }
}

watch(moreOpen, (open) => {
  if (open) {
    document.addEventListener("pointerdown", onMorePointerDown, true);
    document.addEventListener("keydown", onMoreKeydown, true);
    window.addEventListener("scroll", onMoreScroll, true);
    window.addEventListener("resize", onMoreScroll);
  } else {
    document.removeEventListener("pointerdown", onMorePointerDown, true);
    document.removeEventListener("keydown", onMoreKeydown, true);
    window.removeEventListener("scroll", onMoreScroll, true);
    window.removeEventListener("resize", onMoreScroll);
  }
});

/** 「忘记」进行中：按钮置灰，避免重复解除配对。 */
const forgetting = shallowRef(false);

/** 忘记设备：断开 + 解除 Windows 配对 + 清掉本机档案。 */
async function forgetCurrentDevice() {
  const remoteId = currentRemoteId.value;
  if (!remoteId) return;
  const name = displayDeviceName.value;
  const confirmed = await confirmAction({
    title: tr("忘记「{name}」？", { name }),
    message: tr("这会解除它与 Windows 的配对，之后需要重新配对才能使用。"),
    confirmLabel: tr("忘记"),
    danger: true,
  });
  if (!confirmed) return;
  forgetting.value = true;
  try {
    await forgetRemote(remoteId);
    deviceRecords.value = forgetDeviceRecord(remoteId);
    if (lastRemoteId.value === remoteId) {
      lastRemoteId.value = "";
      localStorage.removeItem(lastRemoteKey);
    }
    await store.scan();
    pushToast(tr("已忘记「{name}」", { name }), "success");
  } catch (cause) {
    pushToast(errorMessage(cause), "error");
  } finally {
    forgetting.value = false;
  }
}

/** 配对指南：带上当前设备型号，直接落在对应型号的标签页。 */
function openPairingGuide() {
  pairing.show(remoteModel.value);
}

/** 「设置」打开右侧的设备设置抽屉。 */
const deviceSettingsOpen = shallowRef(false);

function openDeviceSettings() {
  deviceSettingsOpen.value = true;
}

// 从设置页点「去设备设置」跳过来：已挂载时靠计数器监听，挂载时另见 onMounted。
watch(deviceSettingsRequest, () => {
  if (takeDeviceSettingsRequest()) openDeviceSettings();
});

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
  // 从设置页跳过来：等数据加载完再展开抽屉。
  if (takeDeviceSettingsRequest()) openDeviceSettings();
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
        <Button variant="secondary" @click="openPairingGuide">
          <CircleHelp :size="16" />
          {{ tr("配对指南") }}
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

    <Card class="device-hero">
      <div class="hero-body">
        <div class="hero-visual" :class="{ offline: !store.isConnected }">
          <img
            v-if="deviceArtwork"
            :src="deviceArtwork"
            :alt="displayDeviceName"
            class="hero-image"
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

        <div class="hero-main">
          <div class="hero-title-row">
            <h2 class="hero-name">{{ store.isConnected ? displayDeviceName : tr("未连接") }}</h2>
            <StatusPill
              :label="trDynamic(store.isConnected ? '使用中' : '未连接')"
              :tone="store.isConnected ? 'success' : 'neutral'"
              dot
            />
          </div>
          <p class="hero-sub">
            {{ store.isConnected ? heroSubtitle : tr("扫描附近遥控器后连接") }}
          </p>
          <div class="hero-stats">
            <span class="hero-stat" :title="tr('遥控器剩余电量，来自设备的电量通知')">
              <BatteryMedium :size="16" aria-hidden="true" />
              {{ battery === null ? tr("未知") : `${battery}%` }}
            </span>
            <span class="hero-stat" :title="tr('本次连接已经持续的时间')">
              <Clock3 :size="16" aria-hidden="true" />
              {{
                store.isConnected ? tr("{minutes} 分钟", { minutes: connectedMinutes }) : tr("未知")
              }}
            </span>
            <span class="hero-stat" :title="tr('遥控器上报的固件版本')">
              <Cpu :size="16" aria-hidden="true" />
              {{ deviceStatus?.firmwareVersion ?? tr("未知") }}
            </span>
            <span
              v-if="effectiveGain !== null"
              class="hero-stat"
              :title="tr('实际生效的语音增益（设备独立设置优先）')"
            >
              <Volume2 :size="16" aria-hidden="true" />
              {{ effectiveGain }} dB
            </span>
          </div>
          <div class="hero-status-row">
            <div class="hero-chips">
              <span class="hero-chip" :class="{ offline: !store.isConnected }">
                <i aria-hidden="true" />
                {{ store.isConnected ? tr("蓝牙 · 已连接") : tr("蓝牙 · 未连接") }}
              </span>
              <span class="hero-chip"
                ><Mic :size="13" aria-hidden="true" />{{ tr("语音") }} · {{ voiceChipLabel }}</span
              >
              <span class="hero-chip">
                <Wrench :size="13" aria-hidden="true" />
                {{ rc003ChipVisible ? tr("全按键支持 · 已安装") : tr("全按键支持 · 未安装") }}
              </span>
            </div>
            <div class="hero-actions">
              <Button
                v-if="store.isConnected"
                size="sm"
                :disabled="store.connecting"
                @click="disconnect"
              >
                {{ tr("断开") }}
              </Button>

              <Button v-else size="sm" :disabled="store.scanning" @click="refresh">
                {{ trDynamic(store.scanning ? "扫描中" : "扫描设备") }}
              </Button>
              <div v-if="store.isConnected" ref="moreRoot" class="more-actions">
                <IconButton
                  ref="moreTrigger"
                  size="sm"
                  :label="trDynamic('更多操作')"
                  :aria-expanded="moreOpen"
                  aria-haspopup="menu"
                  @click="toggleMore"
                >
                  <MoreVertical :size="15" />
                </IconButton>
                <Teleport to="body">
                <div
                  v-if="moreOpen"
                  ref="moreMenu"
                  class="more-menu"
                  role="menu"
                  :style="moreMenuStyle"
                >
                  <button
                    type="button"
                    role="menuitem"
                    class="more-item"
                    @click="runMore(openDeviceSettings)"
                  >
                    <SlidersHorizontal :size="15" />
                    {{ tr("设备属性") }}
                  </button>
                  <button
                    type="button"
                    role="menuitem"
                    class="more-item danger"
                    :disabled="forgetting"
                    @click="runMore(forgetCurrentDevice)"
                  >
                    <Trash2 :size="15" />
                    {{ tr("忘记设备") }}
                  </button>
                </div>
                </Teleport>
              </div>
            </div>
          </div>
        </div>
      </div>
    </Card>

    <Card class="stack">
      <SectionHeader title="发现的设备" hint="扫描结果，可能包含未适配型号。">
        <template #actions>
          <span class="muted">{{ displayRemotes.length }} {{ tr("台") }}</span>
        </template>
      </SectionHeader>

      <EmptyState
        v-if="displayRemotes.length === 0"
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
          v-for="remote in displayRemotes"
          :key="remote.remoteId"
          class="device-row"
          :class="{ current: isCurrent(remote.remoteId) }"
        >
          <span class="device-icon"><Bluetooth :size="17" /></span>
          <span class="device-main">
            <span class="device-head">
              <span class="device-name">{{ remoteLabel(remote) }}</span>
              <StatusPill
                :label="connectionLabel(remote.remoteId)"
                :tone="connectionTone(remote.remoteId)"
                :dot="isCurrent(remote.remoteId)"
                :title="connectionTitle(remote.remoteId)"
              />
              <StatusPill
                :label="statusLabel(remote.isSupported ? 'supported' : 'unsupported')"
                :tone="remote.isSupported ? 'success' : 'neutral'"
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

    <DeviceSettingsDrawer
      v-model:nickname="deviceNickname"
      v-model:use-override="useDeviceOverride"
      v-model:gain="deviceGain"
      v-model:endpoint-id="deviceEndpointId"
      v-model:voice-input-tool="deviceVoiceInputTool"
      :open="deviceSettingsOpen"
      :fallback-name="deviceName"
      :endpoints="endpoints"
      :voice-tools="voiceToolCatalog"
      :can-override-audio="Boolean(store.isConnected && deviceId)"
      @close="deviceSettingsOpen = false"
    />
  </section>
</template>

<style scoped>
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

.device-hero {
  position: relative;
  overflow: hidden;
  padding: var(--space-5);
  /* 立体感：顶部一道内高光，让卡片从背景里"抬"起来 */
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, #ffffff 72%, transparent),
    var(--shadow-sm);
}

/* 顶部边框用强调色渐变：左侧最实、向右淡出，形成一条带方向的立体边 */
.device-hero::before {
  position: absolute;
  inset: 0 0 auto 0;
  height: 2px;
  border-radius: inherit;
  background: linear-gradient(
    90deg,
    color-mix(in srgb, var(--accent) 72%, transparent) 0%,
    color-mix(in srgb, var(--accent) 34%, transparent) 38%,
    color-mix(in srgb, var(--accent) 10%, transparent) 68%,
    transparent 92%
  );
  content: "";
  pointer-events: none;
}

.hero-body {
  display: grid;
  grid-template-columns: 96px minmax(0, 1fr);
  /* 产品图与右侧信息区按中心线对齐；状态栏单独贴顶（见 .hero-side） */
  align-items: center;
  gap: var(--space-5);
  /* 连接状态切换时卡片高度保持一致，不抖动 */
  min-height: 156px;
  min-width: 0;
}

.hero-visual {
  display: grid;
  place-items: center;
  align-self: center;
  justify-self: center;
  /* 固定槽位：连接/断开切换时图像不改变布局尺寸 */
  width: 64px;
  height: 140px;
  min-width: 0;
}

.hero-visual .device-line-art,
.hero-visual img {
  width: auto;
  max-width: 64px;
  max-height: 140px;
  object-fit: contain;
}

.hero-visual.offline {
  opacity: 0.45;
}

.hero-image {
  user-select: none;
}

.hero-main {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.hero-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
}

.hero-name {
  margin: 0;
  color: var(--text);
  font-size: var(--text-xl);
  font-weight: 650;
  letter-spacing: -0.01em;
}

.hero-sub {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.hero-stats {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-4);
  margin-top: 2px;
}

.hero-stat {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-secondary);
  /* 三项统一字号：中文「已连接 42 分钟」与纯数字项视觉大小才对得上 */
  font-size: var(--text-xs);
  font-variant-numeric: tabular-nums;
}

.hero-stat svg {
  color: var(--text-muted);
}

.hero-chips {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  min-height: 26px;
  margin-top: 2px;
}

.hero-chip {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-raised);
  padding: 3px 12px;
  color: var(--text-secondary);
  font-size: var(--text-2xs);
}

.hero-chip i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--success, var(--accent));
}

.hero-chip.offline i {
  background: var(--text-disabled);
}

.hero-chip svg {
  color: var(--text-muted);
}

.hero-status-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
}

.hero-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  /* 胶囊靠左、按钮靠右，两者同一行自然中心对齐 */
  margin-left: auto;
}

.more-actions {
  position: relative;
  display: inline-flex;
}

.more-menu {
  position: fixed;
  z-index: 60;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 176px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  box-shadow: var(--shadow-md, var(--shadow-lg));
  padding: var(--space-1);
}

.more-item {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  width: 100%;
  border: 0;
  border-radius: var(--radius-sm);
  background: none;
  padding: 7px 8px;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
}

.more-item:hover:not(:disabled) {
  background: var(--bg-hover);
}

.more-item.danger {
  color: var(--danger-text, var(--danger, #d64545));
}

.more-item:disabled {
  opacity: 0.5;
  cursor: default;
}
</style>
