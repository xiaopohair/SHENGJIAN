<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, shallowRef, watch } from "vue";
import { Check, Keyboard, RotateCcw } from "lucide-vue-next";
import { errorMessage, getSettings, getVoiceLevel, saveSettings } from "../lib/api";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { voiceLevelPercent } from "../lib/voice";
import PageHeader from "../components/PageHeader.vue";
import Button from "../components/ui/Button.vue";
import Card from "../components/ui/Card.vue";
import Field from "../components/ui/Field.vue";
import IconButton from "../components/ui/IconButton.vue";
import Notice from "../components/ui/Notice.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import SelectInput from "../components/ui/SelectInput.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import Switch from "../components/ui/Switch.vue";
import { useEngineStore } from "../stores/engine";
import { useStatusStore } from "../stores/status";
import type { SettingsDocument } from "../lib/types";

type PillTone = "neutral" | "success" | "warning" | "danger" | "info" | "accent";

const store = useEngineStore();
const statusStore = useStatusStore();
const error = shallowRef<string | null>(null);
const voiceInputTool = shallowRef("");
const transcriptHistory = shallowRef(false);
const voiceHotkey = ref<string[] | null>(null);
const recordingHotkey = shallowRef(false);
const DEFAULT_HOTKEYS: Record<string, string[]> = {
  wechat: ["ControlLeft", "MetaLeft"],
  doubao: ["AltRight"],
};
const MODIFIER_CODES = new Set([
  "ControlLeft",
  "ControlRight",
  "ShiftLeft",
  "ShiftRight",
  "AltLeft",
  "AltRight",
  "MetaLeft",
  "MetaRight",
]);
const MAIN_HOTKEY_CODES = new Set([
  "Space",
  "Enter",
  "Tab",
  "Backspace",
  "Delete",
  "Insert",
  "Home",
  "End",
  "PageUp",
  "PageDown",
  "ArrowLeft",
  "ArrowRight",
  "ArrowUp",
  "ArrowDown",
  "Backquote",
  "Minus",
  "Equal",
  "BracketLeft",
  "BracketRight",
  "Backslash",
  "Semicolon",
  "Quote",
  "Comma",
  "Period",
  "Slash",
  "NumpadAdd",
  "NumpadSubtract",
  "NumpadMultiply",
  "NumpadDivide",
  "NumpadDecimal",
  "NumpadEnter",
]);
const HOTKEY_LABELS: Record<string, string> = {
  ControlLeft: "Ctrl",
  ControlRight: "右 Ctrl",
  ShiftLeft: "Shift",
  ShiftRight: "右 Shift",
  AltLeft: "Alt",
  AltRight: "右 Alt",
  MetaLeft: "Win",
  MetaRight: "右 Win",
  Space: "Space",
  Enter: "Enter",
  Escape: "Esc",
  Tab: "Tab",
  Backspace: "Backspace",
  Delete: "Delete",
  Insert: "Insert",
  Home: "Home",
  End: "End",
  PageUp: "PageUp",
  PageDown: "PageDown",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Backquote: "`",
  Minus: "-",
  Equal: "=",
  BracketLeft: "[",
  BracketRight: "]",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Comma: ",",
  Period: ".",
  Slash: "/",
  NumpadAdd: "Num +",
  NumpadSubtract: "Num -",
  NumpadMultiply: "Num *",
  NumpadDivide: "Num /",
  NumpadDecimal: "Num .",
  NumpadEnter: "Num Enter",
};
const settingsDocument = ref<SettingsDocument | null>(null);
let saveTimer: number | undefined;
let loadingSettings = false;
let pressedHotkeyCodes = new Set<string>();
let capturedHotkeyCodes: string[] = [];
const testPhase = ref<"idle" | "waiting" | "streaming" | "done">("idle");
const TEST_STEPS = ["等待按下", "收到语音流", "会话结束"] as const;
const voiceLevelDbfs = shallowRef(-120);
const levelPercent = computed(() => voiceLevelPercent(voiceLevelDbfs.value));
const testStatusText = computed(() => {
  switch (testPhase.value) {
    case "waiting":
      return tr("等待语音键…");
    case "streaming":
      return tr("已收到语音流，继续说话，松开结束。");
    case "done":
      return tr("测试完成，语音链路正常。");
    default:
      return tr("点击开始测试，然后按住语音键说一句话。本测试不读取语音内容。");
  }
});
const testNoticeKind = computed<"info" | "success">(() =>
  testPhase.value === "done" ? "success" : "info",
);
const voiceTone = computed<PillTone>(() => {
  switch (store.voiceState) {
    case "streaming":
      return "accent";
    case "draining":
      return "warning";
    case "idle":
      return "neutral";
    default:
      return "info";
  }
});
const audioTone = computed<PillTone>(() => {
  switch (store.audioState) {
    case "ready":
      return "success";
    case "error":
      return "danger";
    case "unconfigured":
      return "neutral";
    default:
      return "warning";
  }
});
const effectiveHotkey = computed(
  () => voiceHotkey.value ?? DEFAULT_HOTKEYS[voiceInputTool.value] ?? [],
);
const hasCustomHotkey = computed(() => (voiceHotkey.value?.length ?? 0) > 0);
const hotkeyText = computed(() =>
  recordingHotkey.value ? tr("请按下组合键…") : formatHotkey(effectiveHotkey.value),
);

function formatHotkey(keys: string[]): string {
  if (keys.length === 0) return tr("不注入热键");
  return keys.map(hotkeyKeyLabel).join(" + ");
}

function hotkeyKeyLabel(key: string): string {
  if (HOTKEY_LABELS[key]) return trDynamic(HOTKEY_LABELS[key]);
  if (key.startsWith("Key")) return key.slice(3);
  if (key.startsWith("Digit")) return key.slice(5);
  if (/^Numpad[0-9]$/.test(key)) return `Num ${key.slice(6)}`;
  return key;
}

function isSupportedHotkeyCode(code: string): boolean {
  return (
    MODIFIER_CODES.has(code) ||
    MAIN_HOTKEY_CODES.has(code) ||
    /^Key[A-Z]$/.test(code) ||
    /^Digit[0-9]$/.test(code) ||
    /^F([1-9]|1[0-2])$/.test(code) ||
    /^Numpad[0-9]$/.test(code)
  );
}

function startHotkeyRecording() {
  recordingHotkey.value = true;
  pressedHotkeyCodes = new Set();
  capturedHotkeyCodes = [];
  window.addEventListener("keydown", onHotkeyKeyDown, true);
  window.addEventListener("keyup", onHotkeyKeyUp, true);
}

function stopHotkeyRecording() {
  recordingHotkey.value = false;
  pressedHotkeyCodes = new Set();
  capturedHotkeyCodes = [];
  window.removeEventListener("keydown", onHotkeyKeyDown, true);
  window.removeEventListener("keyup", onHotkeyKeyUp, true);
}

function finishHotkeyRecording() {
  if (capturedHotkeyCodes.length > 0) {
    voiceHotkey.value = capturedHotkeyCodes.slice(0, 8);
  }
  stopHotkeyRecording();
}

function onHotkeyKeyDown(event: KeyboardEvent) {
  if (!recordingHotkey.value) return;
  event.preventDefault();
  event.stopPropagation();
  if (event.code === "Escape") {
    stopHotkeyRecording();
    return;
  }
  if (event.repeat || !isSupportedHotkeyCode(event.code)) return;
  pressedHotkeyCodes.add(event.code);
  if (!capturedHotkeyCodes.includes(event.code)) {
    capturedHotkeyCodes.push(event.code);
  }
  if (!MODIFIER_CODES.has(event.code)) {
    finishHotkeyRecording();
  }
}

function onHotkeyKeyUp(event: KeyboardEvent) {
  if (!recordingHotkey.value) return;
  event.preventDefault();
  event.stopPropagation();
  pressedHotkeyCodes.delete(event.code);
  if (
    pressedHotkeyCodes.size === 0 &&
    capturedHotkeyCodes.length > 0 &&
    capturedHotkeyCodes.every((code) => MODIFIER_CODES.has(code))
  ) {
    finishHotkeyRecording();
  }
}

function toggleHotkeyRecording() {
  if (recordingHotkey.value) {
    stopHotkeyRecording();
  } else {
    startHotkeyRecording();
  }
}

function resetHotkey() {
  stopHotkeyRecording();
  voiceHotkey.value = null;
}
let levelTimer: number | undefined;

function testStepState(index: number): "done" | "active" | "" {
  if (testPhase.value === "done") return "done";
  if (testPhase.value === "waiting" && index === 0) return "active";
  if (testPhase.value === "streaming") {
    if (index === 0) return "done";
    if (index === 1) return "active";
  }
  return "";
}

async function pollVoiceLevel() {
  try {
    voiceLevelDbfs.value = await getVoiceLevel();
  } catch {
    // 电平是辅助显示，读取失败不影响语音链路。
  }
}

function startLevelPolling() {
  if (levelTimer !== undefined) return;
  void pollVoiceLevel();
  levelTimer = window.setInterval(pollVoiceLevel, 100);
}

function stopLevelPolling() {
  if (levelTimer !== undefined) {
    window.clearInterval(levelTimer);
    levelTimer = undefined;
  }
  voiceLevelDbfs.value = -120;
}

function startVoiceTest() {
  testPhase.value = store.voiceState === "streaming" ? "streaming" : "waiting";
  if (store.voiceState === "streaming") startLevelPolling();
}

watch(
  () => store.voiceState,
  (state) => {
    if (state === "streaming") {
      startLevelPolling();
    } else {
      stopLevelPolling();
    }
    if (testPhase.value === "waiting" && state === "streaming") {
      testPhase.value = "streaming";
    } else if (testPhase.value === "streaming" && (state === "draining" || state === "idle")) {
      testPhase.value = "done";
    }
  },
);

async function load() {
  error.value = null;
  try {
    await store.refresh();
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function loadSettings() {
  loadingSettings = true;
  try {
    const settings = await getSettings();
    settingsDocument.value = settings;
    voiceInputTool.value = String(settings.global.voice_input_tool ?? "");
    statusStore.setVoiceInputTool(settings.global.voice_input_tool);
    const storedHotkey = settings.global.voice_hotkey;
    voiceHotkey.value =
      Array.isArray(storedHotkey) && storedHotkey.length > 0 ? storedHotkey : null;
    transcriptHistory.value = settings.global.local_transcript_history_enabled ?? false;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loadingSettings = false;
  }
}

async function saveVoiceSettings() {
  try {
    const latest = await getSettings();
    const next: SettingsDocument = {
      ...latest,
      global: {
        ...latest.global,
        voice_input_tool: voiceInputTool.value || null,
        voice_hotkey: voiceHotkey.value,
        local_transcript_history_enabled: transcriptHistory.value,
      },
    };
    await saveSettings(next);
    statusStore.setVoiceInputTool(next.global.voice_input_tool);
    settingsDocument.value = next;
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

function scheduleSave() {
  if (loadingSettings || !settingsDocument.value) return;
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    void saveVoiceSettings();
  }, 250);
}

watch([voiceInputTool, transcriptHistory, voiceHotkey], scheduleSave);

onMounted(async () => {
  await Promise.all([load(), loadSettings()]);
  if (store.voiceState === "streaming") startLevelPolling();
});

onUnmounted(() => {
  stopLevelPolling();
  stopHotkeyRecording();
  window.clearTimeout(saveTimer);
});
</script>

<template>
  <section class="page">
    <PageHeader
      eyebrow="Voice"
      title="语音输入与测试"
      description="查看语音与音频状态，配置输入工具、热键与转写，并测试语音链路。"
    >
      <template #meta>
        <div class="status-row">
          <StatusPill
            :label="`${trDynamic('语音')} · ${statusLabel(store.voiceState)}`"
            :tone="voiceTone"
            dot
          />
          <StatusPill
            :label="`${trDynamic('音频')} · ${statusLabel(store.audioState)}`"
            :tone="audioTone"
            dot
          />
        </div>
      </template>
    </PageHeader>

    <Notice v-if="error" kind="error">{{ error }}</Notice>

    <div class="voice-layout">
      <Card class="stack">
        <SectionHeader title="语音输入工具" />

        <Field label="输入法">
          <SelectInput v-model="voiceInputTool">
            <option value="">{{ tr("未选择") }}</option>
            <option value="wechat">{{ tr("微信输入法") }}</option>
            <option value="doubao">{{ tr("豆包输入法") }}</option>
            <option value="vokie">{{ tr("Vokie 桌面版") }}</option>
            <option value="other">{{ tr("其他工具") }}</option>
          </SelectInput>
        </Field>

        <div class="setting-row">
          <span class="setting-label">{{ tr("热键") }}</span>
          <div class="setting-control">
            <div class="hotkey-inline">
              <kbd class="hotkey-value" :class="{ recording: recordingHotkey }" aria-live="polite">
                <span v-if="recordingHotkey" class="hotkey-recording-dot" aria-hidden="true" />
                {{ hotkeyText }}
              </kbd>
              <IconButton
                :label="trDynamic(recordingHotkey ? '取消录制' : '录制热键')"
                @click="toggleHotkeyRecording"
              >
                <Keyboard :size="15" />
              </IconButton>
              <IconButton
                v-if="hasCustomHotkey"
                :label="trDynamic('恢复默认')"
                @click="resetHotkey"
              >
                <RotateCcw :size="15" />
              </IconButton>
            </div>
          </div>
        </div>

        <div class="setting-row">
          <span class="setting-label">{{ tr("本机转写历史") }}</span>
          <div class="setting-control">
            <div class="switch-row">
              <Switch v-model="transcriptHistory" :label="trDynamic('本机转写历史')" />
              <StatusPill
                :label="trDynamic(transcriptHistory ? '启用' : '禁用')"
                :tone="transcriptHistory ? 'success' : 'neutral'"
              />
            </div>
            <span class="setting-hint">
              {{ tr("只保存转写文字，不保存音频。") }}
            </span>
          </div>
        </div>
      </Card>

      <Card class="stack">
        <SectionHeader title="语音测试">
          <template #actions>
            <Button variant="primary" @click="startVoiceTest">
              {{ trDynamic(testPhase === "idle" ? "开始测试" : "重新测试") }}
            </Button>
          </template>
        </SectionHeader>

        <ol class="test-progress" aria-live="polite">
          <li
            v-for="(step, index) in TEST_STEPS"
            :key="step"
            :class="testStepState(index)"
            :aria-current="testStepState(index) === 'active' ? 'step' : undefined"
          >
            <span class="step-index">
              <Check v-if="testStepState(index) === 'done'" :size="13" />
              <template v-else>{{ index + 1 }}</template>
            </span>
            <span class="step-label">{{ trDynamic(step) }}</span>
          </li>
        </ol>

        <div v-if="testPhase !== 'idle'" class="level-row">
          <span class="level-label">{{ tr("实时电平") }}</span>
          <div
            class="level-meter"
            role="meter"
            aria-valuemin="0"
            aria-valuemax="100"
            :aria-valuenow="Math.round(levelPercent)"
            :aria-label="trDynamic('实时语音电平')"
          >
            <div class="level-fill" :style="{ width: `${levelPercent}%` }" />
          </div>
          <span class="level-value mono muted">
            {{ store.voiceState === "streaming" ? `${Math.round(voiceLevelDbfs)} dBFS` : "—" }}
          </span>
        </div>

        <Notice :kind="testNoticeKind">{{ testStatusText }}</Notice>
      </Card>
    </div>
  </section>
</template>

<style scoped>
.voice-layout {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  align-items: start;
  gap: var(--space-5);
}

.status-row {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.setting-row {
  display: grid;
  grid-template-columns: minmax(140px, 200px) minmax(0, 1fr);
  align-items: start;
  gap: var(--space-2) var(--space-3);
  min-width: 0;
}

.setting-label {
  padding-top: 7px;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.4;
}

.setting-control {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  min-width: 0;
}

.setting-hint {
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.4;
}

.switch-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-height: 34px;
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

.test-progress {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-2);
  margin: 0;
  padding: 0;
  list-style: none;
  font-size: var(--text-xs);
}

.test-progress li {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 8px 10px;
  color: var(--text-muted);
}

.test-progress li.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--text);
}

.test-progress li.done {
  border-color: color-mix(in srgb, var(--success) 45%, var(--border));
  color: var(--text);
}

.step-index {
  display: inline-grid;
  flex: 0 0 20px;
  place-items: center;
  width: 20px;
  height: 20px;
  border-radius: 50%;
  background: var(--bg-hover);
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.test-progress li.active .step-index {
  background: var(--accent);
  color: var(--text-on-accent);
}

.test-progress li.done .step-index {
  background: var(--success);
  color: var(--text-on-accent);
}

.step-label {
  min-width: 0;
  overflow-wrap: anywhere;
}

.level-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) 74px;
  align-items: center;
  gap: var(--space-3);
  font-size: var(--text-xs);
}

.level-label {
  color: var(--text-muted);
}

.level-value {
  text-align: right;
}

.level-meter {
  height: 12px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-hover);
}

.level-fill {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, var(--accent), var(--success));
  transition: width 80ms linear;
}

@media (max-width: 1320px) {
  .voice-layout {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 1260px) {
  .setting-row {
    grid-template-columns: 1fr;
  }

  .setting-label {
    padding-top: 0;
  }

  .test-progress {
    grid-template-columns: 1fr;
  }
}
</style>
