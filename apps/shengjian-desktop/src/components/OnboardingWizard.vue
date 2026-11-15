<script setup lang="ts">
import {
  computed,
  onMounted,
  onUnmounted,
  ref,
  shallowRef,
  useTemplateRef,
  watch,
} from "vue";
import { Check, CheckCircle2, Keyboard, Mic, Radio, ShieldCheck } from "lucide-vue-next";
import {
  errorMessage,
  getAudioDependencyStatus,
  getRc003HelperInstalled,
  getSettings,
  installRc003Helper,
  listAudioEndpoints,
  openExternalUrl,
  saveSettings,
} from "../lib/api";
import {
  audioDependencyDetailKey,
  audioDependencyTitleKey,
  audioDependencyTone,
  cableEndpointNames,
  isAudioDependencyReady,
  mergeAudioEndpoints,
  suggestedEndpointId,
  VB_CABLE_INSTALL_URL,
} from "../lib/audioDependency";
import { profileButtonsForModel } from "../lib/buttons";
import { deviceProductNameKey } from "../lib/deviceProduct";
import { supportsKeyEnhancement } from "../lib/keyEnhancement";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { pushToast } from "../lib/toast";
import { useModalFocus } from "../composables/useModalFocus";
import { requiresReportLayerSynthesis, voiceToolOptions } from "../lib/voiceTools";
import { useVoiceToolCatalog } from "../composables/useVoiceToolCatalog";
import type { AudioDependencyStatus, AudioEndpoint, SettingsDocument } from "../lib/types";
import { useEngineStore } from "../stores/engine";
import { usePairingStore } from "../stores/pairing";
import { useStatusStore } from "../stores/status";
import ButtonCheckList from "./ButtonCheckList.vue";
import Button from "./ui/Button.vue";
import Checkbox from "./ui/Checkbox.vue";
import Field from "./ui/Field.vue";
import Notice from "./ui/Notice.vue";
import SelectInput from "./ui/SelectInput.vue";
import StatusPill from "./ui/StatusPill.vue";

type StatusTone = "neutral" | "success" | "warning" | "danger" | "info" | "accent";

const emit = defineEmits<{ complete: [] }>();
const store = useEngineStore();
const statusStore = useStatusStore();
const pairing = usePairingStore();
const step = shallowRef(1);
const connecting = shallowRef(false);
const settings = ref<SettingsDocument | null>(null);
const voiceInputTool = shallowRef("");
const { catalog: voiceToolCatalog, load: loadVoiceToolCatalog } = useVoiceToolCatalog();
const audioEndpointId = shallowRef("");
const endpoints = ref<AudioEndpoint[]>([]);
/** VB-CABLE 依赖：step 3 的依赖卡片与「下一步」硬阻断都由它决定。 */
const audioDependency = shallowRef<AudioDependencyStatus | null>(null);
const audioDependencyLoading = shallowRef(false);
const audioDependencyError = shallowRef<string | null>(null);
const rc003Capture = shallowRef(false);
const rc003HelperInstalled = shallowRef(false);
const helperBusy = shallowRef(false);
/** 安装全按键支持服务的长流程提示：等授权用常驻 Notice，结果用 Toast（§4.1）。 */
const helperNotice = shallowRef<{ text: string; kind: "info" | "error" } | null>(null);
const detectedButtons = ref<string[]>([]);
const voiceFailure = shallowRef(false);

const saving = shallowRef(false);
const error = shallowRef<string | null>(null);
const wizardRef = useTemplateRef<HTMLElement>("wizardRef");
let helperPoll: number | undefined;
let buttonBaseline = 0;

const steps = ["连接遥控器", "扫描设备", "语音测试", "按键测试", "完成"] as const;

const hasVoiceEvent = computed(() =>
  store.voiceHistory.some((event) => event.state === "streaming"),
);
const connected = computed(() => store.phase === "ready");
const currentRemoteId = computed(() => store.snapshot?.connection.remote?.remoteId ?? null);
function isCurrentRemote(remoteId: string): boolean {
  return connected.value && currentRemoteId.value === remoteId;
}

function remoteProductName(remote: { name: string; model: string | null }): string {
  const key = deviceProductNameKey(remote.model);
  return key ? trDynamic(key) : remote.name;
}
const supportsKeyEnhancementCapture = computed(() =>
  supportsKeyEnhancement(store.snapshot?.connection.remote?.model),
);
const detectedCount = computed(() => detectedButtons.value.length);
// 按键清单按型号过滤：ARN9 没有 TV 键，全量布局会让它永远凑不齐。
const remoteModel = computed(() => store.snapshot?.connection.remote?.model ?? null);
const checkButtons = computed(() => profileButtonsForModel(remoteModel.value));
const buttonChecklist = computed(() =>
  checkButtons.value.map((button) => ({
    ...button,
    detected: detectedButtons.value.includes(button.id),
  })),
);
const voiceResult = computed<"success" | "failure" | null>(() => {
  if (hasVoiceEvent.value) return "success";
  if (voiceFailure.value) return "failure";
  return null;
});
const voiceListening = computed(() => step.value === 3 && store.voiceState === "streaming");

const audioDependencyReady = computed(() => isAudioDependencyReady(audioDependency.value));
const audioDependencyTitle = computed(() => {
  if (audioDependencyError.value) return tr("无法检测 VB-CABLE");
  return tr(audioDependencyTitleKey(audioDependency.value?.state));
});
const audioDependencyDetail = computed(() => {
  const key = audioDependencyDetailKey(audioDependency.value?.state);
  if (!key) return "";
  const names = audioDependencyReady.value ? cableEndpointNames(audioDependency.value) : "";
  return names ? tr("语音输出设备：{names}", { names }) : tr(key);
});
const audioDependencyToneValue = computed(() => audioDependencyTone(audioDependency.value?.state));

/**
 * 选中的语音输出设备是否是我们支持的那款虚拟线（CABLE Input）。
 *
 * 依赖装好不等于选对了设备：下拉里还有扬声器、16ch 变体等选项，选错了遥控器语音
 * 不会进输入法，所以第 3 步必须卡住，让用户改选或先安装再「重新检测」。
 */
/**
 * 语音输出设备只支持 VB-CABLE：下拉里直接筛掉扬声器等真实输出设备，
 * 历史保存过的其它设备也自动回到"未设置（自动选 CABLE）"。
 */
const cableOutputs = computed(() => {
  const status = audioDependency.value;
  if (!isAudioDependencyReady(status)) return [];
  const cableIds = new Set((status?.cableEndpoints ?? []).map((endpoint) => endpoint.id));
  const seen = new Set<string>();
  return endpoints.value.filter((endpoint) => {
    const supported = cableIds.has(endpoint.id) || endpoint.recommended;
    if (!supported || seen.has(endpoint.id)) return false;
    seen.add(endpoint.id);
    return true;
  });
});
watch(
  [audioDependencyReady, audioEndpointId, cableOutputs],
  () => {
    if (!audioDependencyReady.value) return;
    const chosen = audioEndpointId.value.trim();
    if (!chosen) return;
    if (cableOutputs.value.some((endpoint) => endpoint.id === chosen)) return;
    audioEndpointId.value = "";
  },
  { immediate: true },
);

/** 会丢弃程序注入按键的输入法：必须走报告层合成（全按键支持服务），否则语音键无效。 */
const voiceToolNeedsHelper = computed(() => requiresReportLayerSynthesis(voiceInputTool.value));
const helperRequired = computed(
  () => voiceToolNeedsHelper.value && supportsKeyEnhancementCapture.value,
);
const helperReady = computed(
  () => !helperRequired.value || (rc003Capture.value && rc003HelperInstalled.value),
);
watch(
  helperRequired,
  (required) => {
    if (required && !rc003Capture.value) rc003Capture.value = true;
  },
  { immediate: true },
);
const showVbCableDownload = computed(
  () => !audioDependencyReady.value && !audioDependencyLoading.value,
);
const stepBlocked = computed(() => {
  if (step.value === 2 && !connected.value) return true;
  if (step.value === 3) {
    if (!isAudioDependencyReady(audioDependency.value)) return true;
    return voiceResult.value !== "success";
  }
  if (step.value === 4) {
    if (!helperReady.value) return true;
    return detectedCount.value !== checkButtons.value.length;
  }
  return false;
});
const blockReason = computed(() => {
  if (!stepBlocked.value) return "";
  if (step.value === 2 && !connected.value) return tr("请先连接遥控器，才能继续。");
  if (step.value === 3 && !isAudioDependencyReady(audioDependency.value)) {
    if (audioDependencyLoading.value) return tr("正在检测 VB-CABLE，请稍候。");
    if (audioDependencyError.value) return tr("无法确认 VB-CABLE 状态，请重试。");
    return tr("请先安装并检测通过 VB-CABLE，才能继续。");
  }
  if (step.value === 3) {
    return voiceResult.value === "failure"
      ? tr("语音测试未通过，请重试。")
      : tr("请先完成语音测试，才能继续。");
  }
  if (step.value === 4 && !helperReady.value) {
    return tr("豆包输入法必须安装全按键支持服务，请勾选后再继续。");
  }
  if (step.value === 4) return tr("请先检测全部按键，才能继续。");
  return "";
});

const primaryLabel = computed(() => {
  if (step.value === 1) return "开始配置";
  if (step.value === 2) return "下一步";
  if (step.value === 3) return "下一步";
  if (step.value === 4) {
    return "下一步";
  }
  return saving.value ? "保存中" : "进入应用";
});

useModalFocus({
  container: wizardRef,
  open: () => true,
  onClose: () => emit("complete"),
});

function phaseTone(phase: string): StatusTone {
  if (phase === "ready") return "success";
  if (phase === "connecting" || phase === "scanning" || phase === "reconnecting") {
    return "warning";
  }
  if (phase === "error" || phase === "disconnected") return "danger";
  return "neutral";
}

function resetButtonCheck() {
  const events = store.buttonHistory;
  buttonBaseline = events[events.length - 1]?.seq ?? 0;
  detectedButtons.value = [];
}

watch(step, (value) => {
  if (value === 3) voiceFailure.value = false;
  if (value === 4) resetButtonCheck();
});

watch(
  () => store.snapshot?.voice.lastError,
  (error) => {
    if (step.value === 3 && error) voiceFailure.value = true;
  },
);

watch(
  () => store.buttonHistory,
  (events) => {
    const detected = new Set(detectedButtons.value);
    for (const event of events) {
      if (event.seq > buttonBaseline) detected.add(event.button);
    }
    const next = checkButtons.value
      .filter((button) => detected.has(button.id))
      .map((button) => button.id);
    if (next.length !== detectedButtons.value.length) {
      detectedButtons.value = next;
    }
  },
);

// 向导里的改动只留在本地状态：只有点「进入应用」才落盘，而且只提交真正改过的字段。
// 这样"重跑一次向导看看"不会把系统里已经配好的东西悄悄覆盖掉。

function applyAudioDependency(status: AudioDependencyStatus) {
  audioDependency.value = status;
  // CABLE 设备可能比 list_audio_endpoints 更新；合并后下拉才选得中建议项。
  endpoints.value = mergeAudioEndpoints(endpoints.value, status.cableEndpoints ?? []);
  const suggested = suggestedEndpointId(status);
  if (suggested && !audioEndpointId.value.trim()) {
    audioEndpointId.value = suggested;
  }
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
    // Download failure is not a detection failure: keep the card state and
    // surface the error through the wizard's normal error notice.
    error.value = errorMessage(cause);
  }
}

async function installHelper() {
  if (helperBusy.value) return;
  helperBusy.value = true;
  helperNotice.value = null;
  try {
    await installRc003Helper();
    helperNotice.value = {
      text: tr("已请求管理员授权；完成 UAC 后会自动刷新状态。"),
      kind: "info",
    };
    window.clearInterval(helperPoll);
    helperPoll = window.setInterval(async () => {
      rc003HelperInstalled.value = await getRc003HelperInstalled();
      if (rc003HelperInstalled.value) {
        window.clearInterval(helperPoll);
        // 结果走 Toast；这条常驻提示只留给「还在等授权」。
        helperNotice.value = null;
        pushToast(tr("全按键支持服务已安装"), "success");
      }
    }, 2000);
  } catch (cause) {
    helperNotice.value = { text: errorMessage(cause), kind: "error" };
  } finally {
    helperBusy.value = false;
  }
}

async function scan() {
  await store.scan();
}

async function connect(remoteId: string) {
  connecting.value = true;
  try {
    await store.connect(remoteId);
  } finally {
    connecting.value = false;
  }
}

function next() {
  if (stepBlocked.value) return;
  if (step.value < 5) step.value += 1;
}

function previous() {
  if (step.value > 1) step.value -= 1;
}

async function finish() {
  const current = settings.value;
  if (!current) {
    emit("complete");
    return;
  }
  // 只提交向导里真正改过的字段：重跑向导不应该覆盖系统里已有的配置。
  const global = current.global;
  const patch: Partial<typeof global> = {};
  const tool = voiceInputTool.value || null;
  if (tool !== (global.voice_input_tool ?? null)) patch.voice_input_tool = tool;
  const endpoint = audioEndpointId.value || null;
  if (endpoint !== (global.audio_endpoint_id ?? null)) patch.audio_endpoint_id = endpoint;
  if (rc003Capture.value !== (global.rc003_capture_enabled ?? false)) {
    patch.rc003_capture_enabled = rc003Capture.value;
  }
  if (Object.keys(patch).length === 0) {
    statusStore.setVoiceInputTool(voiceInputTool.value);
    emit("complete");
    return;
  }
  saving.value = true;
  error.value = null;
  try {
    await saveSettings({ ...current, global: { ...global, ...patch } });
    statusStore.setVoiceInputTool(voiceInputTool.value);
  } catch (cause) {
    error.value = errorMessage(cause);
    saving.value = false;
    return;
  }
  saving.value = false;
  emit("complete");
}

onMounted(async () => {
  try {
    await loadVoiceToolCatalog();
    settings.value = await getSettings();
    voiceInputTool.value = String(settings.value.global.voice_input_tool ?? "");
    audioEndpointId.value = String(settings.value.global.audio_endpoint_id ?? "");
    rc003Capture.value = settings.value.global.rc003_capture_enabled ?? false;
    endpoints.value = await listAudioEndpoints().catch(() => []);
    rc003HelperInstalled.value = await getRc003HelperInstalled().catch(() => false);
  } catch (cause) {
    error.value = errorMessage(cause);
  }
  // 依赖检测独立于设置加载：失败只影响向导第 3 步的卡片与继续按钮。
  await loadAudioDependency();
});

onUnmounted(() => {
  window.clearInterval(helperPoll);
});
</script>

<template>
  <div class="overlay">
    <section
      ref="wizardRef"
      class="wizard"
      role="dialog"
      aria-modal="true"
      aria-labelledby="onboarding-title"
      :aria-label="trDynamic('首次配置')"
    >
      <header class="wizard-head">
        <h2 id="onboarding-title" class="wizard-title">{{ tr("首次配置") }}</h2>
        <Button variant="ghost" @click="emit('complete')">{{ tr("跳过") }}</Button>
      </header>

      <ol class="stepper">
        <li
          v-for="(label, index) in steps"
          :key="label"
          class="stepper-step"
          :class="{ active: step === index + 1, done: step > index + 1 }"
          :aria-current="step === index + 1 ? 'step' : undefined"
        >
          <span class="stepper-dot">
            <Check v-if="step > index + 1" :size="13" />
            <template v-else>{{ index + 1 }}</template>
          </span>
          <span class="stepper-label">{{ trDynamic(label) }}</span>
        </li>
      </ol>

      <Notice v-if="error" class="wizard-error" kind="error">{{ error }}</Notice>

      <div v-if="step === 1" class="wizard-body">
        <span class="step-icon"><ShieldCheck :size="26" /></span>
        <h2>{{ tr("连接你的蓝牙语音遥控器") }}</h2>
        <p class="step-lead">
          {{ tr("本向导会检查遥控器连接、语音链路、按键状态。所有配置将在本地保存。") }}
        </p>
      </div>

      <div v-else-if="step === 2" class="wizard-body">
        <span class="step-icon"><Radio :size="26" /></span>
        <h2>{{ tr("扫描并连接") }}</h2>
        <p class="step-lead">{{ tr("按下遥控器任意键唤醒，然后扫描已配对设备。") }}</p>

        <div class="wizard-panel stack-sm">
          <div class="toolbar">
            <Button variant="primary" :disabled="store.scanning" @click="scan">
              {{ trDynamic(store.scanning ? "扫描中" : "扫描设备") }}
            </Button>
            <Button variant="ghost" @click="pairing.show(store.snapshot?.connection.remote?.model)">
              {{ tr("配对帮助") }}
            </Button>
          </div>

          <ul v-if="store.remotes.length" class="remote-list">
            <li v-for="remote in store.remotes" :key="remote.remoteId" class="remote-row">
              <span class="remote-name text-truncate">{{ remoteProductName(remote) }}</span>
              <StatusPill
                v-if="isCurrentRemote(remote.remoteId)"
                :label="statusLabel(store.phase)"
                :tone="phaseTone(store.phase)"
                dot
              />
              <Button
                size="sm"
                :disabled="connecting || isCurrentRemote(remote.remoteId)"
                @click="connect(remote.remoteId)"
              >
                {{ trDynamic(isCurrentRemote(remote.remoteId) ? "已连接" : "连接") }}
              </Button>
            </li>
          </ul>
        </div>
      </div>

      <div v-else-if="step === 3" class="wizard-body">
        <span class="step-icon" :class="{ speaking: store.voiceState === 'streaming' }">
          <Mic :size="26" />
        </span>
        <h2>{{ tr("测试语音键") }}</h2>
        <p class="step-lead">{{ tr("按住遥控器语音键说一句话，再松开。") }}</p>

        <div
          v-if="!audioDependencyReady"
          class="audio-dependency"
          :class="`tone-${audioDependencyToneValue}`"
          role="status"
          aria-live="polite"
        >
          <div class="audio-dependency-copy">
            <strong>{{ audioDependencyTitle }}</strong>
            <p v-if="audioDependencyDetail">{{ audioDependencyDetail }}</p>
            <p v-if="audioDependencyError" class="audio-dependency-error">
              {{ audioDependencyError }}
            </p>
          </div>
        </div>

        <div class="voice-options">
          <Field :label="tr('语音输出设备')">
            <SelectInput v-model="audioEndpointId">
              <option value="">{{ tr("未设置（连接时自动选 CABLE）") }}</option>
              <option v-for="endpoint in cableOutputs" :key="endpoint.id" :value="endpoint.id">
                {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
              </option>
            </SelectInput>
            <p v-if="cableOutputs.length === 0" class="audio-endpoint-hint">
              {{ tr("当前仅支持 VB-CABLE。") }}
            </p>
            <div v-if="cableOutputs.length === 0" class="audio-endpoint-actions">
              <Button v-if="showVbCableDownload" size="sm" @click="downloadVbCable">
                {{ tr("下载 VB-CABLE") }}
              </Button>
              <Button size="sm" :disabled="audioDependencyLoading" @click="loadAudioDependency">
                {{ audioDependencyLoading ? tr("检测中…") : tr("重新检测") }}
              </Button>
            </div>
          </Field>
          <Field :label="tr('语音输入工具')">
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
          </Field>
        </div>

        <div v-if="voiceResult" class="voice-result">
          <StatusPill
            :label="trDynamic(voiceResult === 'success' ? '语音功能可用。' : '语音功能不可用。')"
            :tone="voiceResult === 'success' ? 'success' : 'danger'"
            dot
          />
        </div>
      </div>

      <div v-else-if="step === 4" class="wizard-body">
        <span class="step-icon"><Keyboard :size="26" /></span>
        <h2>{{ tr("检测遥控器按键") }}</h2>
        <p class="step-lead">{{ tr("请依次按下遥控器上的各个键，检测按键是否可用。") }}</p>

        <div class="wizard-panel stack-sm">
          <ButtonCheckList :buttons="buttonChecklist" />

          <div v-if="supportsKeyEnhancementCapture" class="capture-setting">
            <div class="capture-heading">
              <Checkbox v-model="rc003Capture" :disabled="helperRequired">{{
                tr("启用全按键支持服务")
              }}</Checkbox>
              <div v-if="rc003Capture" class="capture-actions">
                <StatusPill
                  :label="trDynamic(rc003HelperInstalled ? '已安装' : '未安装')"
                  :tone="rc003HelperInstalled ? 'success' : 'warning'"
                />
                <Button
                  size="sm"
                  :variant="rc003HelperInstalled ? 'secondary' : 'primary'"
                  :disabled="helperBusy"
                  @click="installHelper"
                >
                  {{ trDynamic(helperBusy ? "安装中" : rc003HelperInstalled ? "更新" : "安装") }}
                </Button>
              </div>
            </div>
            <p v-if="helperRequired" class="capture-hint">
              {{
                tr(
                  "豆包输入法会丢弃程序注入的按键，必须启用并安装全按键支持服务，语音键才生效。",
                )
              }}
            </p>
            <p class="muted capture-hint">
              {{
                tr("返回键和音量键需要安装全按键支持服务并开启才能使用；首次安装会请求管理员授权。")
              }}
            </p>
            <Notice v-if="helperNotice" :kind="helperNotice.kind">
              {{ helperNotice.text }}
            </Notice>
          </div>
        </div>
      </div>

      <div v-else class="wizard-body">
        <span class="step-icon"><CheckCircle2 :size="26" /></span>
        <h2>{{ tr("配置完成") }}</h2>
        <p class="step-lead">
          {{ tr("你可以在按键页管理键位方案、在设置页调整音频和启动行为。向导可以随时重新运行。") }}
        </p>
      </div>

      <footer class="wizard-footer">
        <Button v-if="step > 1" variant="ghost" @click="previous">{{ tr("上一步") }}</Button>
        <p v-if="stepBlocked" class="wizard-block-reason" role="alert">{{ blockReason }}</p>
        <span class="footer-spacer" />
        <Button
          v-if="step === 4 && detectedCount > 0"
          variant="ghost"
          @click="resetButtonCheck"
        >
          {{ tr("重新测试") }}
        </Button>
        <Button v-if="step < 5" variant="primary" :disabled="stepBlocked" @click="next">
          {{ trDynamic(primaryLabel) }}
        </Button>
        <Button v-else variant="primary" :disabled="saving" @click="finish">
          {{ trDynamic(primaryLabel) }}
        </Button>
      </footer>

      <Transition name="voice-layer">
        <div v-if="voiceListening" class="voice-layer" aria-live="polite">
          <div class="voice-layer-card">
            <div class="voice-wave" aria-hidden="true">
              <span
                v-for="index in 12"
                :key="index"
                :style="{
                  animationDelay: `${index * -0.08}s`,
                  height: `${20 + ((index * 13) % 55)}%`,
                }"
              />
            </div>
            <p>{{ tr("正在接收语音…") }}</p>
          </div>
        </div>
      </Transition>
    </section>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  z-index: 50;
  display: grid;
  place-items: center;
  background: var(--overlay);
  padding: var(--space-5);
  backdrop-filter: blur(8px);
}

.wizard {
  position: relative;
  display: flex;
  flex-direction: column;
  width: min(680px, 100%);
  max-height: calc(100vh - 80px);
  overflow: auto;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  padding: var(--space-6);
}

.wizard-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
}

.wizard-title {
  font-size: var(--text-2xl);
  font-weight: 650;
  letter-spacing: -0.02em;
}

.stepper {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: var(--space-2);
  margin: var(--space-5) 0 0;
  padding: 0;
  list-style: none;
}

.stepper-step {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.stepper-step + .stepper-step::before {
  width: 18px;
  height: 1px;
  margin-right: var(--space-1);
  background: var(--border-strong);
  content: "";
}

.stepper-dot {
  display: grid;
  flex: 0 0 22px;
  place-items: center;
  width: 22px;
  height: 22px;
  border: 1px solid var(--border-strong);
  border-radius: 50%;
  font-size: var(--text-2xs);
  font-weight: 650;
}

.stepper-step.active {
  color: var(--text);
  font-weight: 600;
}

.stepper-step.active .stepper-dot {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--text-on-accent);
}

.stepper-step.done .stepper-dot {
  border-color: var(--accent);
  color: var(--accent);
}

.stepper-step.done .stepper-label {
  color: var(--text-secondary);
}

.wizard-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  align-items: center;
  gap: var(--space-3);
  min-height: 280px;
  padding: var(--space-6) 0;
  text-align: center;
}

.wizard-error {
  margin-top: var(--space-4);
}

.step-icon {
  display: grid;
  place-items: center;
  width: 48px;
  height: 48px;
  border-radius: var(--radius-lg);
  background: var(--accent-soft);
  color: var(--accent);
}

.wizard-body h2 {
  font-size: var(--text-xl);
}

.step-lead {
  max-width: 52ch;
  color: var(--text-muted);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.wizard-panel {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  width: 100%;
  max-width: 520px;
  margin-top: var(--space-2);
  text-align: left;
}

.voice-options {
  display: grid;
  gap: var(--space-3);
  width: 100%;
  max-width: 520px;
  margin-top: var(--space-1);
  text-align: left;
}

.audio-dependency {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  width: 100%;
  max-width: 520px;
  margin-top: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-3) var(--space-4);
  text-align: left;
}

.audio-dependency.tone-success {
  border-color: var(--success);
  background: var(--success-soft);
}

.audio-dependency.tone-warning {
  border-color: var(--warning);
  background: var(--warning-soft);
}

.audio-dependency-copy {
  display: flex;
  flex: 1 1 260px;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.audio-dependency-copy strong {
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 650;
  line-height: 1.4;
}

.audio-dependency-copy p {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.audio-dependency-copy .audio-dependency-error {
  color: var(--danger-text);
}

.audio-dependency-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}

.audio-endpoint-actions {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.audio-endpoint-hint {
  margin: 4px 0 0;
  color: var(--warning-text);
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.voice-result {
  display: flex;
  justify-content: center;
}

.voice-layer {
  position: absolute;
  inset: 0;
  z-index: 5;
  display: grid;
  place-items: center;
  border-radius: inherit;
  background: var(--overlay);
  backdrop-filter: blur(6px);
  pointer-events: none;
}

.voice-layer-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-4);
  padding: var(--space-6);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.voice-layer-card p {
  margin: 0;
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.voice-wave {
  display: flex;
  align-items: center;
  gap: 5px;
  height: 64px;
}

.voice-wave span {
  width: 5px;
  min-height: 8px;
  border-radius: var(--radius-pill);
  background: linear-gradient(180deg, var(--accent), var(--success));
  transform-origin: center;
  animation: voice-wave 0.9s ease-in-out infinite;
}

@keyframes voice-wave {
  0%,
  100% {
    opacity: 0.55;
    transform: scaleY(0.3);
  }
  50% {
    opacity: 1;
    transform: scaleY(1);
  }
}

.voice-layer-enter-active,
.voice-layer-leave-active {
  transition: opacity 180ms ease;
}

.voice-layer-enter-from,
.voice-layer-leave-to {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .voice-wave span {
    animation: none;
  }
}

.step-icon.speaking {
  animation: voice-pulse 1.2s ease-in-out infinite;
}

@keyframes voice-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 var(--accent-soft);
  }
  50% {
    box-shadow: 0 0 0 8px var(--accent-soft);
  }
}

@media (prefers-reduced-motion: reduce) {
  .step-icon.speaking {
    animation: none;
  }
}

.remote-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  max-height: 240px;
  margin: 0;
  padding: 0 var(--space-1) 0 0;
  overflow-y: auto;
  overscroll-behavior: contain;
  list-style: none;
}

.remote-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
}

.remote-name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-sm);
  font-weight: 600;
}

.capture-hint {
  margin: 0;
  color: var(--warning-text);
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.capture-setting {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding-top: var(--space-2);
  text-align: left;
}

.capture-heading {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.capture-hint {
  font-size: var(--text-xs);
  line-height: 1.5;
}

.capture-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.wizard-footer {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding-top: var(--space-4);
}

.wizard-block-reason {
  margin: 0;
  color: var(--warning-text);
  font-size: var(--text-xs);
  line-height: 1.45;
}

.footer-spacer {
  flex: 1;
}

@media (max-width: 1260px) {
  .wizard {
    padding: var(--space-5);
  }

  .stepper-label {
    display: none;
  }
}
</style>
