<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { Command, FileText, Keyboard, Mic, Trash2 } from "lucide-vue-next";
import {
  clearMacroRunHistory,
  clearTranscriptHistory,
  errorMessage,
  getMacroRunHistory,
  getTranscriptHistory,
} from "../lib/api";
import { PROFILE_BUTTONS } from "../lib/buttons";
import { confirmAction } from "../lib/confirm";
import { statusLabel, tr, trDynamic, uiLocale } from "../lib/i18n";
import PageHeader from "../components/PageHeader.vue";
import Card from "../components/ui/Card.vue";
import EmptyState from "../components/ui/EmptyState.vue";
import IconButton from "../components/ui/IconButton.vue";
import Notice from "../components/ui/Notice.vue";
import SectionHeader from "../components/ui/SectionHeader.vue";
import StatusPill from "../components/ui/StatusPill.vue";
import Tabs from "../components/ui/Tabs.vue";
import { useEngineStore } from "../stores/engine";
import type { MacroHistoryEntry, TranscriptRecord } from "../lib/types";

type StatusTone = "neutral" | "success" | "warning" | "danger" | "info" | "accent";

interface MacroHistoryEntryView {
  macroId: string;
  name: string;
  result: string;
  elapsedMs: number;
  atUnix: number;
}

const store = useEngineStore();
const activeTab = ref<string>("voice");
const macroHistory = ref<MacroHistoryEntryView[]>([]);
const transcripts = ref<TranscriptRecord[]>([]);
const error = ref<string | null>(null);

const tabs = computed(() => [
  { id: "voice", label: "语音", count: store.voiceHistory.length },
  { id: "buttons", label: "按键", count: store.buttonHistory.length },
  { id: "macros", label: "宏运行", count: macroHistory.value.length },
  { id: "transcripts", label: "转写", count: transcripts.value.length },
]);

function formatTime(unixSeconds: number): string {
  return new Intl.DateTimeFormat(uiLocale.value, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(new Date(unixSeconds * 1000));
}

function buttonLabel(button: string): string {
  const label = PROFILE_BUTTONS.find((item) => item.id === button)?.label ?? button;
  return trDynamic(label);
}

function buttonAction(event: (typeof store.buttonHistory)[number]): string {
  if (event.type === "gesture") return statusLabel(event.trigger ?? "");
  return trDynamic(event.pressed ? "按下" : "松开");
}

function voiceTone(state: string): StatusTone {
  if (state === "streaming") return "success";
  if (state === "draining") return "warning";
  return "neutral";
}

function macroTone(result: string): StatusTone {
  if (result === "completed" || result === "done") return "success";
  if (result === "cancelled" || result === "timed_out") return "warning";
  if (result === "failed" || result === "step_failed" || result === "invalid_macro")
    return "danger";
  return "neutral";
}

async function loadMacroHistory() {
  try {
    const entries = await getMacroRunHistory();
    macroHistory.value = entries.map((entry: MacroHistoryEntry) => ({
      macroId: entry.macroId,
      name: entry.name,
      result: entry.result,
      elapsedMs: entry.elapsedMs,
      atUnix: entry.atUnix,
    }));
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function loadTranscripts() {
  try {
    transcripts.value = await getTranscriptHistory();
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function loadAll() {
  error.value = null;
  await Promise.all([loadMacroHistory(), loadTranscripts()]);
}

async function clearMacroHistory() {
  const confirmed = await confirmAction({
    title: "清空全部宏运行历史？",
    message: "清空后无法恢复。",
    confirmLabel: "清空",
    danger: true,
  });
  if (!confirmed) return;
  try {
    await clearMacroRunHistory();
    macroHistory.value = [];
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function clearTranscripts() {
  const confirmed = await confirmAction({
    title: "清除全部本机转写历史？此操作不可恢复。",
    confirmLabel: "清除全部",
    danger: true,
  });
  if (!confirmed) return;
  try {
    await clearTranscriptHistory();
    transcripts.value = [];
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

onMounted(async () => {
  await loadAll();
});

watch(activeTab, (tab) => {
  if (tab === "macros") void loadMacroHistory();
  if (tab === "transcripts") void loadTranscripts();
});
</script>

<template>
  <section class="page">
    <PageHeader
      eyebrow="History"
      title="历史"
      description="集中查看最近的语音、按键、宏运行和转写记录。"
    />

    <Notice v-if="error" kind="error">{{ error }}</Notice>

    <Tabs v-model="activeTab" :items="tabs" />

    <div
      v-if="activeTab === 'voice'"
      id="panel-voice"
      class="tab-panel"
      role="tabpanel"
      aria-labelledby="tab-voice"
    >
      <Card class="stack">
        <SectionHeader title="最近语音事件" hint="本次运行内的语音状态变化。">
          <template #actions>
            <span class="muted">{{ store.voiceHistory.length }} {{ tr("条") }}</span>
          </template>
        </SectionHeader>
        <EmptyState
          v-if="store.voiceHistory.length === 0"
          title="暂无语音事件"
          description="按住语音键说话，再松开。"
        >
          <template #icon><Mic :size="20" /></template>
        </EmptyState>
        <ul v-else class="event-list">
          <li v-for="event in store.voiceHistory" :key="event.seq" class="event-row">
            <span class="mono muted">#{{ event.seq }}</span>
            <StatusPill :label="statusLabel(event.state)" :tone="voiceTone(event.state)" dot />
          </li>
        </ul>
      </Card>
    </div>

    <div
      v-else-if="activeTab === 'buttons'"
      id="panel-buttons"
      class="tab-panel"
      role="tabpanel"
      aria-labelledby="tab-buttons"
    >
      <Card class="stack">
        <SectionHeader title="最近按键" hint="本次运行内的按键与手势事件。">
          <template #actions>
            <span class="muted">{{ store.buttonHistory.length }} {{ tr("条") }}</span>
          </template>
        </SectionHeader>
        <EmptyState
          v-if="store.buttonHistory.length === 0"
          title="暂无按键事件"
          description="配置并激活 profile 后按 OK。"
        >
          <template #icon><Keyboard :size="20" /></template>
        </EmptyState>
        <ul v-else class="event-list">
          <li v-for="event in store.buttonHistory" :key="event.seq" class="event-row">
            <span class="mono muted">#{{ event.seq }}</span>
            <span class="event-name">{{ buttonLabel(event.button) }}</span>
            <StatusPill
              :label="buttonAction(event)"
              :tone="event.type === 'gesture' ? 'accent' : 'neutral'"
            />
          </li>
        </ul>
      </Card>
    </div>

    <div
      v-else-if="activeTab === 'macros'"
      id="panel-macros"
      class="tab-panel"
      role="tabpanel"
      aria-labelledby="tab-macros"
    >
      <Card class="stack">
        <SectionHeader title="最近运行" hint="最近 50 次运行记录，持久保存。">
          <template #actions>
            <span class="muted">{{ macroHistory.length }} {{ tr("条") }}</span>
            <IconButton
              :label="trDynamic('清空宏运行历史')"
              variant="danger"
              :disabled="!macroHistory.length"
              @click="clearMacroHistory"
            >
              <Trash2 :size="15" />
            </IconButton>
          </template>
        </SectionHeader>
        <EmptyState
          v-if="macroHistory.length === 0"
          title="暂无宏运行记录"
          description="测试宏或通过按键触发后会记录在这里。"
        >
          <template #icon><Command :size="20" /></template>
        </EmptyState>
        <ul v-else class="history-list">
          <li
            v-for="(entry, index) in macroHistory"
            :key="`${entry.macroId}-${entry.atUnix}-${index}`"
            class="history-row"
          >
            <div class="history-main">
              <span class="history-name">{{ entry.name }}</span>
              <span class="history-meta">{{ formatTime(entry.atUnix) }}</span>
            </div>
            <StatusPill :label="statusLabel(entry.result)" :tone="macroTone(entry.result)" dot />
            <span class="history-duration mono">{{ entry.elapsedMs }} ms</span>
          </li>
        </ul>
      </Card>
    </div>

    <div
      v-else
      id="panel-transcripts"
      class="tab-panel"
      role="tabpanel"
      aria-labelledby="tab-transcripts"
    >
      <Card class="stack">
        <SectionHeader title="本机转写历史" hint="只保存转写文字，不保存音频。">
          <template #actions>
            <span class="muted">{{ transcripts.length }} {{ tr("条") }}</span>
            <IconButton
              :label="trDynamic('清除全部本机转写历史')"
              variant="danger"
              :disabled="!transcripts.length"
              @click="clearTranscripts"
            >
              <Trash2 :size="15" />
            </IconButton>
          </template>
        </SectionHeader>
        <EmptyState
          v-if="transcripts.length === 0"
          title="暂无转写记录"
          description="在语音页开启转写历史后，使用语音键。"
        >
          <template #icon><FileText :size="20" /></template>
        </EmptyState>
        <ul v-else class="transcript-list">
          <li v-for="record in transcripts" :key="record.sessionId" class="transcript-row">
            <div class="transcript-meta">
              <span class="transcript-app">{{ record.applicationName }}</span>
              <span class="mono muted">{{ formatTime(record.startedAtMs / 1000) }}</span>
            </div>
            <p class="transcript-text">{{ record.text }}</p>
          </li>
        </ul>
      </Card>
    </div>
  </section>
</template>

<style scoped>
.tab-panel {
  min-width: 0;
}

.event-list,
.history-list,
.transcript-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  margin: 0;
  padding: 0;
  list-style: none;
}

.event-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) 0;
}

.event-name {
  flex: 1;
  min-width: 0;
  font-size: var(--text-sm);
}

.history-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto 80px;
  align-items: center;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) 0;
}

.history-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.history-name {
  font-size: var(--text-sm);
  font-weight: 600;
}

.history-meta {
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.history-duration {
  color: var(--text-muted);
  text-align: right;
}

.transcript-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
  border-bottom: 1px solid var(--border);
  padding: var(--space-3) 0;
}

.transcript-meta {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.transcript-app {
  color: var(--text-secondary);
  font-weight: 600;
}

.transcript-text {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.6;
  overflow-wrap: anywhere;
  white-space: pre-wrap;
}

@media (max-width: 1260px) {
  .history-row {
    grid-template-columns: minmax(0, 1fr) auto;
  }

  .history-duration {
    grid-column: 1 / -1;
    text-align: left;
  }
}
</style>
