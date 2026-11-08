<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref } from "vue";
import { Check, CheckCircle2, Mic, Radio, ShieldCheck } from "lucide-vue-next";
import { errorMessage, getSettings, saveSettings } from "../lib/api";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import type { SettingsDocument } from "../lib/types";
import { useEngineStore } from "../stores/engine";
import { useStatusStore } from "../stores/status";
import Button from "./ui/Button.vue";
import Field from "./ui/Field.vue";
import Notice from "./ui/Notice.vue";
import SelectInput from "./ui/SelectInput.vue";
import StatusPill from "./ui/StatusPill.vue";

type StatusTone = "neutral" | "success" | "warning" | "danger" | "info" | "accent";

const emit = defineEmits<{ complete: [] }>();
const store = useEngineStore();
const statusStore = useStatusStore();
const step = ref(1);
const connecting = ref(false);
const settings = ref<SettingsDocument | null>(null);
const voiceInputTool = ref("");
const saving = ref(false);
const error = ref<string | null>(null);
const wizardRef = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;

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

const steps = ["连接遥控器", "扫描设备", "语音测试", "完成"] as const;

const hasVoiceEvent = computed(() =>
  store.voiceHistory.some((event) => event.state === "streaming"),
);
const connected = computed(() => store.phase === "ready");

const primaryLabel = computed(() => {
  if (step.value === 1) return "开始配置";
  if (step.value === 2) return connected.value ? "下一步" : "稍后连接";
  if (step.value === 3) return hasVoiceEvent.value ? "下一步" : "跳过语音测试";
  return saving.value ? "保存中" : "进入应用";
});

function phaseTone(phase: string): StatusTone {
  if (phase === "ready") return "success";
  if (phase === "connecting" || phase === "scanning" || phase === "reconnecting") {
    return "warning";
  }
  if (phase === "error" || phase === "disconnected") return "danger";
  return "neutral";
}

function voiceTone(state: string): StatusTone {
  if (state === "streaming") return "success";
  if (state === "draining") return "warning";
  return "neutral";
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
  if (step.value < 4) step.value += 1;
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
  } catch (cause) {
    error.value = errorMessage(cause);
  }
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
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
        <h2>{{ tr("连接你的小米蓝牙语音遥控器") }}</h2>
        <p class="step-lead">
          {{ tr("本向导会检查遥控器连接、语音链路和基础设置。所有配置都由本机 core 保存。") }}
        </p>
      </div>

      <div v-else-if="step === 2" class="wizard-body">
        <span class="step-icon"><Radio :size="26" /></span>
        <h2>{{ tr("扫描并连接") }}</h2>
        <p class="step-lead">{{ tr("按下遥控器任意键唤醒，然后扫描已配对设备。") }}</p>

        <div class="wizard-panel stack-sm">
          <div class="toolbar">
            <Button :disabled="store.scanning" @click="scan">
              {{ trDynamic(store.scanning ? "扫描中" : "扫描设备") }}
            </Button>
            <StatusPill :label="statusLabel(store.phase)" :tone="phaseTone(store.phase)" dot />
          </div>

          <ul v-if="store.remotes.length" class="remote-list">
            <li v-for="remote in store.remotes" :key="remote.remoteId" class="remote-row">
              <span class="remote-name text-truncate">{{ remote.name }}</span>
              <Button
                size="sm"
                :disabled="connecting || connected"
                @click="connect(remote.remoteId)"
              >
                {{ trDynamic(connected ? "已连接" : "连接") }}
              </Button>
            </li>
          </ul>

          <Notice v-if="connected" kind="success">{{ tr("设备已连接，可以继续。") }}</Notice>
        </div>
      </div>

      <div v-else-if="step === 3" class="wizard-body">
        <span class="step-icon"><Mic :size="26" /></span>
        <h2>{{ tr("测试语音键") }}</h2>
        <p class="step-lead">{{ tr("按住遥控器语音键说一句话，再松开。") }}</p>

        <div class="wizard-panel stack-sm">
          <div class="voice-state">
            <span class="voice-state-item">
              <span class="muted">{{ tr("语音") }}</span>
              <StatusPill
                :label="statusLabel(store.voiceState)"
                :tone="voiceTone(store.voiceState)"
                dot
              />
            </span>
            <span class="voice-state-item">
              <span class="muted">{{ tr("音频") }}</span>
              <StatusPill :label="statusLabel(store.audioState)" tone="neutral" />
            </span>
          </div>
          <p class="wizard-hint">{{ trDynamic(hasVoiceEvent ? "已收到语音流" : "等待语音流") }}</p>

          <ul v-if="store.voiceHistory.length" class="event-list">
            <li v-for="event in store.voiceHistory" :key="event.seq" class="event-row">
              <span class="mono muted">#{{ event.seq }}</span>
              <StatusPill :label="statusLabel(event.state)" :tone="voiceTone(event.state)" />
            </li>
          </ul>

          <Field label="语音输入工具">
            <SelectInput v-model="voiceInputTool">
              <option value="">{{ tr("未选择") }}</option>
              <option value="wechat">{{ tr("微信输入法") }}</option>
              <option value="doubao">{{ tr("豆包输入法") }}</option>
              <option value="vokie">{{ tr("Vokie 桌面版") }}</option>
              <option value="other">{{ tr("其他工具") }}</option>
            </SelectInput>
          </Field>
        </div>
      </div>

      <div v-else class="wizard-body">
        <span class="step-icon"><CheckCircle2 :size="26" /></span>
        <h2>{{ tr("配置完成") }}</h2>
        <p class="step-lead">
          {{ tr("你可以在按键页管理 profile、在设置页调整音频和启动行为。向导可以随时重新运行。") }}
        </p>
      </div>

      <footer class="wizard-footer">
        <Button v-if="step > 1" variant="ghost" @click="previous">{{ tr("上一步") }}</Button>
        <span class="footer-spacer" />
        <Button v-if="step < 4" variant="primary" @click="next">
          {{ trDynamic(primaryLabel) }}
        </Button>
        <Button v-else variant="primary" :disabled="saving" @click="finish">
          {{ trDynamic(primaryLabel) }}
        </Button>
      </footer>
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

.wizard-hint {
  color: var(--text-muted);
  font-size: var(--text-sm);
}

.voice-state {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-3);
}

.voice-state-item {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  font-size: var(--text-sm);
}

.remote-list,
.event-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  margin: 0;
  padding: 0;
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

.event-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-1) 0;
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

@media (max-width: 900px) {
  .wizard {
    padding: var(--space-5);
  }

  .stepper-label {
    display: none;
  }
}
</style>
