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
import { Check, CheckCircle2, Keyboard, Mic, Radio, ShieldCheck } from "lucide-vue-next";
import {
  errorMessage,
  getRc003HelperInstalled,
  getSettings,
  installRc003Helper,
  listAudioEndpoints,
  saveSettings,
} from "../lib/api";
import { PROFILE_BUTTONS } from "../lib/buttons";
import { deviceProductNameKey } from "../lib/deviceProduct";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import type { AudioEndpoint, SettingsDocument } from "../lib/types";
import { useEngineStore } from "../stores/engine";
import { usePairingStore } from "../stores/pairing";
import { useStatusStore } from "../stores/status";
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
const audioEndpointId = shallowRef("");
const endpoints = ref<AudioEndpoint[]>([]);
const rc003Capture = shallowRef(false);
const rc003HelperInstalled = shallowRef(false);
const helperBusy = shallowRef(false);
const helperMessage = shallowRef<string | null>(null);
const detectedButtons = ref<string[]>([]);
const voiceFailure = shallowRef(false);
const settingsReady = shallowRef(false);
const saving = shallowRef(false);
const error = shallowRef<string | null>(null);
const wizardRef = useTemplateRef<HTMLElement>("wizardRef");
let previousFocus: HTMLElement | null = null;
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
const isRc003 = computed(() => store.snapshot?.connection.remote?.model === "RC003");
const detectedCount = computed(() => detectedButtons.value.length);
const buttonChecklist = computed(() =>
  PROFILE_BUTTONS.map((button) => ({
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

const primaryLabel = computed(() => {
  if (step.value === 1) return "开始配置";
  if (step.value === 2) return connected.value ? "下一步" : "稍后连接";
  if (step.value === 3) return hasVoiceEvent.value ? "下一步" : "跳过语音测试";
  if (step.value === 4) {
    return detectedCount.value === PROFILE_BUTTONS.length ? "下一步" : "跳过按键测试";
  }
  return saving.value ? "保存中" : "进入应用";
});

function focusableElements(): HTMLElement[] {
  if (!wizardRef.value) return [];
  return Array.from(
    wizardRef.value.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((element) => !element.hasAttribute("disabled") && element.offsetParent !== null);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    emit("complete");
    return;
  }
  if (event.key !== "Tab") return;
  const elements = focusableElements();
  if (elements.length === 0) return;
  const first = elements[0];
  const last = elements[elements.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

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
    const next = PROFILE_BUTTONS.filter((button) => detected.has(button.id)).map(
      (button) => button.id,
    );
    if (next.length !== detectedButtons.value.length) {
      detectedButtons.value = next;
    }
  },
  { deep: true },
);

async function saveGlobalSettings(patch: Partial<SettingsDocument["global"]>) {
  if (!settingsReady.value || !settings.value) return;
  try {
    const next: SettingsDocument = {
      ...settings.value,
      global: {
        ...settings.value.global,
        ...patch,
      },
    };
    await saveSettings(next);
    settings.value = next;
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

watch(rc003Capture, (value) => {
  void saveGlobalSettings({ rc003_capture_enabled: value });
});

watch(voiceInputTool, (value) => {
  void saveGlobalSettings({ voice_input_tool: value || null });
});

watch(audioEndpointId, (value) => {
  void saveGlobalSettings({ audio_endpoint_id: value || null });
});

async function installHelper() {
  if (helperBusy.value) return;
  helperBusy.value = true;
  helperMessage.value = null;
  try {
    await installRc003Helper();
    helperMessage.value = tr("已请求管理员授权；完成 UAC 后会自动刷新状态。");
    window.clearInterval(helperPoll);
    helperPoll = window.setInterval(async () => {
      rc003HelperInstalled.value = await getRc003HelperInstalled();
      if (rc003HelperInstalled.value) {
        window.clearInterval(helperPoll);
        helperMessage.value = tr("RC003 增强捕获服务已安装。");
      }
    }, 2000);
  } catch (cause) {
    helperMessage.value = errorMessage(cause);
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
  if (step.value < 5) step.value += 1;
}

function previous() {
  if (step.value > 1) step.value -= 1;
}

async function finish() {
  if (settings.value) {
    saving.value = true;
    error.value = null;
    try {
      await saveSettings({
        ...settings.value,
        global: {
          ...settings.value.global,
          voice_input_tool: voiceInputTool.value || null,
          audio_endpoint_id: audioEndpointId.value || null,
          rc003_capture_enabled: rc003Capture.value,
        },
      });
      statusStore.setVoiceInputTool(voiceInputTool.value);
    } catch (cause) {
      error.value = errorMessage(cause);
      saving.value = false;
      return;
    }
    saving.value = false;
  }
  emit("complete");
}

onMounted(async () => {
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  window.addEventListener("keydown", onKeydown, true);
  await nextTick();
  focusableElements()[0]?.focus();
  try {
    settings.value = await getSettings();
    voiceInputTool.value = String(settings.value.global.voice_input_tool ?? "");
    audioEndpointId.value = String(settings.value.global.audio_endpoint_id ?? "");
    rc003Capture.value = settings.value.global.rc003_capture_enabled ?? false;
    endpoints.value = await listAudioEndpoints().catch(() => []);
    rc003HelperInstalled.value = await getRc003HelperInstalled().catch(() => false);
    settingsReady.value = true;
  } catch (cause) {
    error.value = errorMessage(cause);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  window.clearInterval(helperPoll);
  previousFocus?.focus();
  previousFocus = null;
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

        <div class="voice-options">
          <Field :label="tr('语音输入工具')">
            <SelectInput v-model="voiceInputTool">
              <option value="">{{ tr("未选择") }}</option>
              <option value="wechat">{{ tr("微信输入法") }}</option>
              <option value="doubao">{{ tr("豆包输入法") }}</option>
              <option value="vokie">{{ tr("Vokie 桌面版") }}</option>
              <option value="other">{{ tr("其他工具") }}</option>
            </SelectInput>
          </Field>
          <Field :label="tr('音频输入设备')">
            <SelectInput v-model="audioEndpointId">
              <option value="">{{ tr("未设置（连接时自动选 CABLE）") }}</option>
              <option v-for="endpoint in endpoints" :key="endpoint.id" :value="endpoint.id">
                {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
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

        <div class="wizard-panel stack-sm button-check-panel">
          <div class="button-check-head">
            <span class="muted"
              >{{ tr("已检测") }} {{ detectedCount }}/{{ PROFILE_BUTTONS.length }}</span
            >
            <span v-if="detectedCount === PROFILE_BUTTONS.length" class="muted">
              {{ tr("全部按键可用") }}
            </span>
          </div>

          <ul class="button-check-list">
            <li
              v-for="button in buttonChecklist"
              :key="button.id"
              class="button-check-item"
              :class="{ detected: button.detected }"
            >
              <span class="button-check-name">{{ trDynamic(button.label) }}</span>
              <StatusPill
                :label="trDynamic(button.detected ? '可用' : '待检测')"
                :tone="button.detected ? 'success' : 'neutral'"
              />
            </li>
          </ul>

          <div v-if="isRc003" class="capture-setting">
            <div class="capture-heading">
              <Checkbox v-model="rc003Capture">{{ tr("启用 RC003 增强捕获") }}</Checkbox>
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
            <p class="muted capture-hint">
              {{ tr("返回键、音量键需要启用 RC003 增强捕获并安装服务后才能检测。") }}
            </p>
            <Notice v-if="helperMessage" :kind="rc003HelperInstalled ? 'success' : 'info'">
              {{ helperMessage }}
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
        <span class="footer-spacer" />
        <Button v-if="step < 5" variant="primary" @click="next">
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

.button-check-panel {
  align-items: stretch;
  text-align: left;
}

.button-check-head {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  font-size: var(--text-sm);
}

.button-check-list {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.button-check-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-2) var(--space-3);
}

.button-check-item.detected {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.button-check-name {
  font-size: var(--text-sm);
  font-weight: 600;
  overflow-wrap: anywhere;
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

.footer-spacer {
  flex: 1;
}

@media (max-width: 1260px) {
  .button-check-list {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }

  .wizard {
    padding: var(--space-5);
  }

  .stepper-label {
    display: none;
  }
}
</style>
