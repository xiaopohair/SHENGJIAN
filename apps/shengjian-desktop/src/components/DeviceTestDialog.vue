<script setup lang="ts">
/**
 * 设备测试面板：语音链路与按键。
 *
 * 只读：全部结论都来自 core 推上来的事件（`voiceHistory` / `buttonHistory`），
 * 不写任何设置，也不改设备状态，所以放在顶栏命令面板里随时开关都安全。
 * 外壳（遮罩/标题/关闭/焦点圈定）统一用 ui/DialogShell，内容和底部走插槽。
 */
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
import { Keyboard, Mic } from "lucide-vue-next";
import {
  closeVoiceTestPad,
  openVoiceTestPad,
  raiseVoiceTestPad,
  voiceTestPadForeground,
  voiceTestPadText,
} from "../lib/api";
import { tr, trDynamic } from "../lib/i18n";
import { profileButtonsForModel } from "../lib/buttons";
import { useEngineStore } from "../stores/engine";
import ButtonCheckList from "./ButtonCheckList.vue";
import Button from "./ui/Button.vue";
import DialogShell from "./ui/DialogShell.vue";

type TestTab = "voice" | "buttons";

const props = withDefaults(defineProps<{ initialTab?: TestTab }>(), { initialTab: "voice" });
const emit = defineEmits<{ close: [] }>();

const store = useEngineStore();
const tab = ref<TestTab>(props.initialTab);

function close() {
  emit("close");
}

// ---------- 原生测试（开关） ----------

/** 识别内容区是否显示（点过「原生测试」就一直留着，方便回看刚才的内容）。 */
const transcriptVisible = shallowRef(false);
/** 原生录入窗口是否开着（一次测试：点按钮打开，读完内容就注销）。 */
const padOpen = shallowRef(false);
const transcriptField = useTemplateRef<HTMLTextAreaElement>("transcriptField");
const transcript = shallowRef("");

/** `rgb(a)`/`rgba(...)`/`#rrggbb` → 0xRRGGBB；解析不了就用兜底色。 */
function parseCssColor(value: string, fallback: number): number {
  const match = value.match(/rgba?\(([^)]+)\)/i);
  if (match) {
    const parts = match[1].split(",").map((part) => Number.parseFloat(part.trim()));
    if (parts.length >= 3 && parts.every((part) => Number.isFinite(part))) {
      const [r, g, b] = parts;
      return ((r & 0xff) << 16) | ((g & 0xff) << 8) | (b & 0xff);
    }
  }
  const hex = value.trim().match(/^#([0-9a-f]{6})$/i);
  if (hex) return Number.parseInt(hex[1], 16);
  return fallback;
}

/** 把原生录入窗口贴到"识别内容"框的位置（物理像素 = CSS 像素 × DPR）。 */
async function openPad() {
  const field = transcriptField.value;
  const measured = field?.getBoundingClientRect();
  const box =
    measured && measured.width > 0
      ? { left: measured.left, top: measured.top, width: measured.width, height: measured.height }
      : { left: 360, top: 420, width: 600, height: 120 };
  const dpr = window.devicePixelRatio || 1;
  const computedStyle = getComputedStyle(field ?? document.documentElement);
  await openVoiceTestPad({
    x: Math.round(box.left * dpr),
    y: Math.round(box.top * dpr),
    width: Math.round(box.width * dpr),
    height: Math.round(box.height * dpr),
    background: parseCssColor(computedStyle.backgroundColor, 0xffffff),
    border: parseCssColor(computedStyle.borderTopColor, 0xd8d8d8),
  }).catch(() => undefined);
}

/** 点「原生测试」：显示出识别内容区，再把原生窗口贴上去，等这次说话的识别结果。 */
async function startNativeTest() {
  transcriptVisible.value = true;
  resetVoiceTest();
  // 先让识别内容区渲染出来，再按它的位置摆原生窗口。
  await nextTick();
  await openPad();
  padOpen.value = true;
  await refreshTranscript();
}

/** 收起原生窗口（一次测试结束）：文字留在识别内容区里。 */
async function closePad() {
  if (!padOpen.value) return;
  padOpen.value = false;
  await closeVoiceTestPad().catch(() => undefined);
}

/** 会话结束/切回窗口时取一次录入内容；拿到就收起原生窗口，把文字留在区域里。 */
async function refreshTranscript() {
  let captured: string | null = null;
  try {
    captured = await voiceTestPadText();
  } catch {
    captured = null;
  }
  if (captured && captured.trim()) {
    // 这次说完了：文字留在识别内容区，原生窗口注销；要再测就再点一次按钮。
    transcript.value = captured.trim();
    void closePad();
    return;
  }
  if (padOpen.value) void raiseVoiceTestPad().catch(() => undefined);
}

// ---------- 语音测试 ----------

function latestVoiceSeq(): number {
  const events = store.voiceHistory;
  return events.length > 0 ? events[events.length - 1].seq : 0;
}

const voiceBaseline = shallowRef(0);
const voicePassed = shallowRef(false);
const voiceFailed = shallowRef(false);

watch(
  () => store.voiceHistory,
  (events) => {
    for (const event of events) {
      if (event.seq <= voiceBaseline.value) continue;
      if (event.state === "streaming") voicePassed.value = true;
    }
  },
);

watch(
  () => store.snapshot?.voice.lastError,
  (error) => {
    if (error) voiceFailed.value = true;
  },
);

const voiceStreaming = computed(() => store.voiceState === "streaming");
const voiceResult = computed<"success" | "failure" | null>(() => {
  if (voicePassed.value) return "success";
  if (voiceFailed.value) return "failure";
  return null;
});

const voiceHeadline = computed(() => {
  if (voiceResult.value === "success") return "语音链路正常";
  if (voiceResult.value === "failure") return "语音链路异常";
  if (voiceStreaming.value) return "正在接收语音";
  return "等待语音输入";
});

const voiceCaption = computed(() => {
  if (voiceResult.value === "success") return "";
  if (voiceResult.value === "failure") return "检查语音输入设备后再试一次";
  if (voiceStreaming.value) return "说完松开即可";
  return "按住遥控器语音键说一句话";
});

const voiceTone = computed(() => ({
  live: voiceStreaming.value && voiceResult.value === null,
  ok: voiceResult.value === "success",
  fail: voiceResult.value === "failure",
}));

function resetVoiceTest() {
  voicePassed.value = false;
  voiceFailed.value = false;
  voiceBaseline.value = latestVoiceSeq();
}

/** 底部「重新测试」：清状态；原生测试开着就把原生窗口重新摆好。 */
function retest() {
  if (tab.value === "buttons") {
    resetButtonTest();
    return;
  }
  resetVoiceTest();
  if (padOpen.value) {
    void nextTick().then(() => openPad());
  }
}

// ---------- 按键测试 ----------

const detectedButtons = ref<string[]>([]);
let buttonBaseline = 0;
let blurTimer: number | undefined;
let transcriptTimers: number[] = [];

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

const remoteModel = computed(() => store.snapshot?.connection.remote?.model ?? null);
// 与首次配置同一个口径：ARN9 没有 TV 键，只列它真正有的键。
const checkButtons = computed(() => profileButtonsForModel(remoteModel.value));
const buttonChecklist = computed(() =>
  checkButtons.value.map((button) => ({
    ...button,
    detected: detectedButtons.value.includes(button.id),
  })),
);

const allButtonsDetected = computed(
  () => checkButtons.value.length > 0 && detectedButtons.value.length === checkButtons.value.length,
);

function resetButtonTest() {
  buttonBaseline = store.buttonHistory[store.buttonHistory.length - 1]?.seq ?? 0;
  detectedButtons.value = [];
}

onMounted(() => {
  // 打开面板后才开始计数：之前的历史事件不算本次测试。
  voiceBaseline.value = latestVoiceSeq();
  buttonBaseline = store.buttonHistory[store.buttonHistory.length - 1]?.seq ?? 0;
  window.addEventListener("focus", onWindowFocus);
  window.addEventListener("blur", onWindowBlur);
  document.addEventListener("pointerdown", onPointerDown, true);
});

function onWindowFocus() {
  if (padOpen.value) void refreshTranscript();
}

/** 点到面板里任何地方（不是原生窗口）：这次测试就结束了，注销原生窗口，文字留着。 */
function onPointerDown(event: PointerEvent) {
  if (!padOpen.value) return;
  const target = event.target as HTMLElement | null;
  // 点"原生测试"按钮本身不算"点到别处"。
  if (target?.closest(".native-test-button")) return;
  void closePad();
}

/** 点到别的应用：延迟一下确认焦点确实不在原生窗口上，再收起。 */
function onWindowBlur() {
  if (!padOpen.value) return;
  if (blurTimer !== undefined) window.clearTimeout(blurTimer);
  blurTimer = window.setTimeout(async () => {
    blurTimer = undefined;
    if (!padOpen.value || document.hasFocus()) return;
    const stillPad = await voiceTestPadForeground().catch(() => false);
    if (!stillPad) void closePad();
  }, 200);
}

onUnmounted(() => {
  window.removeEventListener("focus", onWindowFocus);
  window.removeEventListener("blur", onWindowBlur);
  document.removeEventListener("pointerdown", onPointerDown, true);
  if (blurTimer !== undefined) window.clearTimeout(blurTimer);
  for (const timer of transcriptTimers) window.clearTimeout(timer);
  transcriptTimers = [];
  void closeVoiceTestPad().catch(() => undefined);
});

// 切到按键页时收起原生测试：区域隐藏，原生窗口注销。
watch(tab, (value) => {
  if (value === "voice") return;
  void closePad();
  transcriptVisible.value = false;
});

// 会话从 streaming 回到 idle 后取结果（0.8s 首选，2s 兜底）。
watch(
  () => store.voiceState,
  (state, previous) => {
    if (!padOpen.value) return;
    // 会话结束可能是 streaming → draining → idle，所以别只认直接回 idle。
    if (state === "idle" && previous !== "idle") {
      for (const timer of transcriptTimers) window.clearTimeout(timer);
      transcriptTimers = [
        window.setTimeout(() => void refreshTranscript(), 800),
        window.setTimeout(() => void refreshTranscript(), 2000),
      ];
    }
  },
);
</script>

<template>
  <DialogShell
    title="设备测试"
    title-id="device-test-title"
    :subtitle="tr('只用于测试，不会修改任何设置')"
    width="min(660px, 100%)"
    content-height="360px"
    @close="close"
  >
    <div class="test-tabs" role="tablist" :aria-label="trDynamic('设备测试')">
      <button
        type="button"
        role="tab"
        class="test-tab"
        :class="{ active: tab === 'voice' }"
        :aria-selected="tab === 'voice'"
        @click="tab = 'voice'"
      >
        <Mic :size="15" />
        {{ tr("语音测试") }}
      </button>
      <button
        type="button"
        role="tab"
        class="test-tab"
        :class="{ active: tab === 'buttons' }"
        :aria-selected="tab === 'buttons'"
        @click="tab = 'buttons'"
      >
        <Keyboard :size="15" />
        {{ tr("按键测试") }}
      </button>
    </div>

    <div v-if="tab === 'voice'" class="test-pane">
      <div class="voice-hero" :class="voiceTone">
        <span class="voice-orb"><Mic :size="20" /></span>
        <div class="voice-copy">
          <p class="voice-title">{{ tr(voiceHeadline) }}</p>
          <p v-if="voiceCaption" class="voice-sub">{{ tr(voiceCaption) }}</p>
        </div>
        <div class="voice-pills">
          <Button
            class="native-test-button"
            size="sm"
            variant="secondary"
            :title="tr('点一下打开原生窗口，说完话松开即可')"
            @click="startNativeTest"
          >
            <Mic :size="14" />
            {{ tr("原生测试") }}
          </Button>

        </div>
      </div>

      <div v-if="transcriptVisible" class="transcript-card">
        <div class="transcript-head">
          <label class="transcript-title" for="device-test-transcript">{{ tr("识别内容") }}</label>
          <span class="transcript-hint">{{ tr("会话结束后自动填入") }}</span>
        </div>
        <textarea
          id="device-test-transcript"
          ref="transcriptField"
          class="transcript-input"
          :value="transcript"
          :placeholder="tr('说完话后这里会显示识别内容。')"
          rows="3"
          readonly
        />
      </div>
    </div>

    <div v-else class="test-pane">
      <ButtonCheckList :buttons="buttonChecklist" :show-complete-label="false">
        <template #hint>
          <span class="key-hint" :class="{ done: allButtonsDetected }">
            {{
              allButtonsDetected
                ? tr("全部按键可用")
                : tr("请依次按下遥控器上的各个键，检测按键是否可用。")
            }}
          </span>
        </template>
      </ButtonCheckList>
    </div>

    <template #footer>
      <Button variant="ghost" @click="retest">{{ tr("重新测试") }}</Button>
      <span class="test-spacer" />
      <Button variant="secondary" @click="close">{{ tr("完成") }}</Button>
    </template>
  </DialogShell>
</template>

<style scoped>
.test-tabs {
  display: flex;
  flex: 0 0 auto;
  gap: var(--space-2);
  /* 与配对指南一致：标题区与导航区之间要有分割线。 */
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) var(--space-6);
}

.test-tab {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: none;
  padding: 4px 12px;
  color: var(--text-muted);
  font-size: var(--text-sm);
  cursor: pointer;
}

.test-tab.active {
  border-color: var(--accent);
  color: var(--accent);
  background: var(--accent-soft);
}

.test-pane {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  padding: var(--space-4) var(--space-6) var(--space-5);
}

.test-lead {
  margin: 0;
  font-size: var(--text-sm);
  color: var(--text-muted);
}

.key-hint {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.key-hint.done {
  color: var(--accent);
  font-weight: 600;
}

/* ---- 语音状态卡 ---- */

.voice-hero {
  position: relative;
  display: flex;
  align-items: center;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-hover);
  padding: var(--space-3) var(--space-4);
  overflow: hidden;
}

.voice-hero::before {
  content: "";
  position: absolute;
  inset: 0 auto 0 0;
  width: 3px;
  background: var(--border-strong);
}

.voice-hero.live::before,
.voice-hero.ok::before {
  background: var(--accent);
}

.voice-hero.fail::before {
  background: var(--danger, #d64545);
}

.voice-hero.ok {
  background: var(--accent-soft);
  border-color: var(--accent);
}

.voice-orb {
  display: grid;
  flex: 0 0 40px;
  place-items: center;
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: var(--bg-raised);
  color: var(--accent);
  box-shadow: inset 0 0 0 1px var(--border);
}

.voice-hero.live .voice-orb {
  color: var(--bg-raised);
  background: var(--accent);
  animation: voice-pulse 1.4s ease-in-out infinite;
}

.voice-hero.fail .voice-orb {
  color: var(--danger, #d64545);
}

@keyframes voice-pulse {
  0%,
  100% {
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--accent) 45%, transparent);
  }
  60% {
    box-shadow: 0 0 0 8px color-mix(in srgb, var(--accent) 0%, transparent);
  }
}

.voice-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.voice-title {
  margin: 0;
  font-size: var(--text-md, var(--text-base));
  font-weight: 650;
  line-height: 1.3;
}

.voice-sub {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.voice-pills {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: var(--space-2);
}

/* ---- 原生测试开关 ---- */

.native-test-button {
  white-space: nowrap;
}

/* ---- 识别内容卡 ---- */

.transcript-card {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  padding: var(--space-3) var(--space-4);
}

.transcript-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: var(--space-3);
}

.transcript-title {
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 650;
  letter-spacing: 0.02em;
}

.transcript-hint {
  color: var(--text-muted);
  font-size: var(--text-2xs, var(--text-xs));
  opacity: 0.85;
}

.transcript-input {
  width: 100%;
  min-height: 84px;
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  padding: 0;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  line-height: 1.65;
  resize: none;
}

.transcript-input:read-only {
  cursor: text;
}

.transcript-input::placeholder {
  color: var(--text-muted);
}

.transcript-input:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.test-note {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.test-spacer {
  flex: 1;
}
</style>
