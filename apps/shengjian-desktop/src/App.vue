<script setup lang="ts">
import { computed, defineAsyncComponent, nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { getVersion } from "@tauri-apps/api/app";
import {
  BatteryLow,
  BatteryMedium,
  Command,
  History as HistoryIcon,
  Info,
  Keyboard,
  Mic,
  PanelLeftClose,
  PanelLeftOpen,
  Radio,
  RefreshCw,
  Settings as SettingsIcon,
} from "lucide-vue-next";
import ConnectionPage from "./pages/ConnectionPage.vue";
import OnboardingWizard from "./components/OnboardingWizard.vue";
import ToastHost from "./components/ToastHost.vue";
import TitleBar from "./components/TitleBar.vue";
import TopbarSearch from "./components/TopbarSearch.vue";
import ConfirmDialog from "./components/ui/ConfirmDialog.vue";
import LoadingState from "./components/ui/LoadingState.vue";
import StatusPill from "./components/ui/StatusPill.vue";
import { getSettings, setTrayLanguage } from "./lib/api";
import { setLanguagePreference, statusLabel, tr, trDynamic, uiLocale } from "./lib/i18n";
import { canLeavePage } from "./lib/navigation";
import {
  applyAccent,
  applyTheme,
  ACCENT_KEY,
  ONBOARDING_KEY,
  RESTART_ONBOARDING_EVENT,
  SIDEBAR_KEY,
  getThemePreference,
} from "./lib/preferences";
import { finishStartup } from "./lib/startup";
import { useEngineStore } from "./stores/engine";
import { useStatusStore } from "./stores/status";
import type { PaletteCommand } from "./lib/palette";
import type { EventEnvelope } from "./lib/types";
import BrandMark from "./components/BrandMark.vue";

type PageId = "connection" | "voice" | "buttons" | "macros" | "history" | "settings" | "about";

const VoicePage = defineAsyncComponent(() => import("./pages/VoicePage.vue"));
const ButtonsPage = defineAsyncComponent(() => import("./pages/ButtonsPage.vue"));
const MacrosPage = defineAsyncComponent(() => import("./pages/MacrosPage.vue"));
const SettingsPage = defineAsyncComponent(() => import("./pages/SettingsPage.vue"));
const HistoryPage = defineAsyncComponent(() => import("./pages/HistoryPage.vue"));
const AboutPage = defineAsyncComponent(() => import("./pages/AboutPage.vue"));

const store = useEngineStore();
const status = useStatusStore();
const active = ref<PageId>("connection");
const appVersion = ref("");
const showOnboarding = ref(localStorage.getItem(ONBOARDING_KEY) !== "1");
const scrollArea = ref<HTMLElement | null>(null);
const scrollbarVisible = ref(false);
const themePreference = ref("system");
const sidebarCollapsed = ref(localStorage.getItem(SIDEBAR_KEY) === "1");
const topbarSearch = ref<InstanceType<typeof TopbarSearch> | null>(null);
let unlisten: UnlistenFn | undefined;
let unlistenNavigate: UnlistenFn | undefined;
let mediaQuery: MediaQueryList | undefined;
let statusTimer: number | undefined;
let scrollbarTimer: number | undefined;

const SCROLLBAR_IDLE_MS = 800;
const SCROLLBAR_HINT_PX = 18;

function showMainScrollbar() {
  scrollbarVisible.value = true;
  if (scrollbarTimer !== undefined) {
    window.clearTimeout(scrollbarTimer);
    scrollbarTimer = undefined;
  }
}

function scheduleMainScrollbarHide(delay = SCROLLBAR_IDLE_MS) {
  if (scrollbarTimer !== undefined) window.clearTimeout(scrollbarTimer);
  scrollbarTimer = window.setTimeout(() => {
    scrollbarVisible.value = false;
    scrollbarTimer = undefined;
  }, delay);
}

function onMainScroll() {
  showMainScrollbar();
  scheduleMainScrollbarHide();
}

function onMainPointerMove(event: PointerEvent) {
  const element = scrollArea.value;
  if (!element) return;
  const rect = element.getBoundingClientRect();
  if (event.clientX >= rect.right - SCROLLBAR_HINT_PX) {
    showMainScrollbar();
  } else if (scrollbarVisible.value) {
    scheduleMainScrollbarHide(350);
  }
}

function onMainPointerLeave() {
  scheduleMainScrollbarHide(150);
}

const INPUT_TOOL_LABELS: Record<string, string> = {
  wechat: "微信输入法",
  doubao: "豆包输入法",
  vokie: "Vokie 桌面版",
  other: "其他工具",
};

const pages = [
  { id: "connection" as const, label: "连接", icon: Radio, component: ConnectionPage },
  { id: "voice" as const, label: "语音", icon: Mic, component: VoicePage },
  { id: "buttons" as const, label: "按键", icon: Keyboard, component: ButtonsPage },
  { id: "macros" as const, label: "宏", icon: Command, component: MacrosPage },
  { id: "history" as const, label: "历史", icon: HistoryIcon, component: HistoryPage },
  { id: "settings" as const, label: "设置", icon: SettingsIcon, component: SettingsPage },
  { id: "about" as const, label: "关于", icon: Info, component: AboutPage },
];

const navGroups = [
  { label: "设备", ids: ["connection", "voice"] as PageId[] },
  { label: "自动化", ids: ["buttons", "macros"] as PageId[] },
  { label: "记录", ids: ["history"] as PageId[] },
  { label: "系统", ids: ["settings", "about"] as PageId[] },
].map((group) => ({
  ...group,
  items: group.ids
    .map((id) => pages.find((page) => page.id === id))
    .filter((page): page is (typeof pages)[number] => Boolean(page)),
}));

const activePage = computed(() => pages.find((page) => page.id === active.value) ?? pages[0]);
const phase = computed(() => store.phase);
const phaseTone = computed(() => {
  const value = String(phase.value);
  if (value === "ready") return "success" as const;
  if (["connecting", "scanning", "disconnecting", "reconnecting"].includes(value)) {
    return "warning" as const;
  }
  if (value === "failed" || value === "error") return "danger" as const;
  return "neutral" as const;
});
const remoteName = computed(() => store.snapshot?.connection.remote?.name ?? tr("未连接"));
const batteryLow = computed(() => status.batteryLevel !== null && status.batteryLevel <= 20);
const batteryIcon = computed(() => (batteryLow.value ? BatteryLow : BatteryMedium));
const inputToolLabel = computed(() =>
  status.voiceInputTool
    ? trDynamic(INPUT_TOOL_LABELS[status.voiceInputTool] ?? status.voiceInputTool)
    : tr("未选择"),
);
const voiceActive = computed(() => String(store.voiceState) !== "idle");
const voiceTone = computed(() => {
  const state = String(store.voiceState);
  if (state === "streaming") return "success" as const;
  if (state === "draining") return "warning" as const;
  return "neutral" as const;
});
const audioProblem = computed(() => {
  const state = String(store.audioState);
  return state === "error" || state === "failed";
});

const paletteCommands = computed<PaletteCommand[]>(() => [
  ...pages.map((page) => ({
    id: `nav:${page.id}`,
    label: page.label,
    group: "导航",
    icon: page.icon,
    keywords: `${page.id} ${page.label} navigation`,
  })),
  {
    id: "action:scan",
    label: "扫描遥控器",
    group: "操作",
    icon: RefreshCw,
    keywords: "scan bluetooth 扫描",
  },
  {
    id: "action:voice",
    label: "打开语音测试",
    group: "操作",
    icon: Mic,
    keywords: "voice test 语音 测试",
  },
]);

function syncTheme() {
  applyTheme(themePreference.value);
}

function syncSystemTheme() {
  applyTheme(getThemePreference());
}

async function loadTheme() {
  try {
    const settings = await getSettings();
    themePreference.value = String(settings.global.theme_preference ?? "system");
    status.setVoiceInputTool(settings.global.voice_input_tool);
    setLanguagePreference(String(settings.global.application_language ?? "system"));
    await setTrayLanguage(uiLocale.value).catch(() => undefined);
  } catch {
    // Keep system defaults when settings are unavailable.
  }
  syncTheme();
}

function refreshStatusIfVisible() {
  if (document.visibilityState === "visible") {
    void status.refreshDeviceStatus();
  }
}

function finishOnboarding() {
  localStorage.setItem(ONBOARDING_KEY, "1");
  showOnboarding.value = false;
}

function restartOnboarding() {
  localStorage.removeItem(ONBOARDING_KEY);
  showOnboarding.value = true;
}

async function goTo(pageId: PageId) {
  if (pageId === active.value) return;
  if (!(await canLeavePage())) return;
  active.value = pageId;
}

function toggleSidebar() {
  sidebarCollapsed.value = !sidebarCollapsed.value;
  localStorage.setItem(SIDEBAR_KEY, sidebarCollapsed.value ? "1" : "0");
}

function runPaletteCommand(command: PaletteCommand) {
  if (command.id.startsWith("nav:")) {
    void goTo(command.id.slice(4) as PageId);
  } else if (command.id === "action:scan") {
    void store.scan();
  } else if (command.id === "action:voice") {
    void goTo("voice");
  }
}

function onGlobalKeydown(event: KeyboardEvent) {
  if (!(event.ctrlKey || event.metaKey) || event.key.toLowerCase() !== "k") return;
  event.preventDefault();
  if (showOnboarding.value) return;
  topbarSearch.value?.open();
}

watch(active, async () => {
  await nextTick();
  scrollArea.value?.scrollTo({ top: 0 });
});

onMounted(async () => {
  unlisten = await listen<EventEnvelope>("core-event", (event) => {
    store.applyEvent(event.payload);
  });
  unlistenNavigate = await listen<string>("navigate", (event) => {
    if (pages.some((page) => page.id === event.payload)) {
      void goTo(event.payload as PageId);
    }
  });
  await store.hydrate();
  await loadTheme();
  appVersion.value = await getVersion().catch(() => "");
  applyAccent(localStorage.getItem(ACCENT_KEY) ?? "default");
  await finishStartup();
  await status.refreshDeviceStatus();
  statusTimer = window.setInterval(refreshStatusIfVisible, 3000);
  document.addEventListener("visibilitychange", refreshStatusIfVisible);
  window.addEventListener("keydown", onGlobalKeydown, true);
  window.addEventListener(RESTART_ONBOARDING_EVENT, restartOnboarding);
  mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
  mediaQuery.addEventListener("change", syncSystemTheme);
});

onUnmounted(() => {
  unlisten?.();
  unlistenNavigate?.();
  if (statusTimer !== undefined) window.clearInterval(statusTimer);
  if (scrollbarTimer !== undefined) window.clearTimeout(scrollbarTimer);
  document.removeEventListener("visibilitychange", refreshStatusIfVisible);
  window.removeEventListener("keydown", onGlobalKeydown, true);
  window.removeEventListener(RESTART_ONBOARDING_EVENT, restartOnboarding);
  mediaQuery?.removeEventListener("change", syncSystemTheme);
});
</script>

<template>
  <div class="window">
    <TitleBar />
    <div class="shell">
      <a class="skip-link" href="#main-content">{{ tr("跳到主内容") }}</a>
      <aside
        class="sidebar"
        :class="{ collapsed: sidebarCollapsed }"
        :aria-label="trDynamic('应用侧栏')"
      >
        <div class="sidebar-brand">
          <BrandMark :height="24" />
          <div v-if="!sidebarCollapsed" class="sidebar-brand-copy">
            <span class="sidebar-brand-name">{{ tr("声键") }}</span>
            <span v-if="appVersion" class="sidebar-brand-version mono">v{{ appVersion }}</span>
          </div>
          <button
            type="button"
            class="sidebar-collapse"
            :aria-label="trDynamic(sidebarCollapsed ? '展开侧栏' : '折叠侧栏')"
            :title="trDynamic(sidebarCollapsed ? '展开侧栏' : '折叠侧栏')"
            :aria-expanded="!sidebarCollapsed"
            aria-controls="sidebar-nav"
            @click="toggleSidebar"
          >
            <PanelLeftOpen v-if="sidebarCollapsed" :size="16" />
            <PanelLeftClose v-else :size="16" />
          </button>
        </div>
        <nav id="sidebar-nav" class="nav" :aria-label="trDynamic('主导航')">
          <div v-for="group in navGroups" :key="group.label" class="nav-group">
            <p class="nav-group-label">{{ trDynamic(group.label) }}</p>
            <button
              v-for="page in group.items"
              :key="page.id"
              class="nav-item"
              :class="{ active: page.id === active }"
              :aria-current="page.id === active ? 'page' : undefined"
              :aria-label="trDynamic(page.label)"
              :title="trDynamic(page.label)"
              type="button"
              @click="goTo(page.id)"
            >
              <component :is="page.icon" :size="17" :stroke-width="1.8" />
              <span class="nav-label">{{ trDynamic(page.label) }}</span>
            </button>
          </div>
        </nav>
      </aside>

      <main class="main">
        <header class="topbar">
          <div class="topbar-left">
            <div class="topbar-status">
              <span role="status" aria-live="polite">
                <StatusPill :label="statusLabel(phase)" :tone="phaseTone" dot />
              </span>
              <span
                v-if="store.isConnected"
                class="topbar-device text-truncate"
                :title="remoteName"
                >{{ remoteName }}</span
              >
              <span
                v-if="status.batteryLevel !== null"
                class="topbar-chip"
                :class="{ low: batteryLow }"
                :title="trDynamic('遥控器电量')"
              >
                <component :is="batteryIcon" :size="15" />
                <span class="sr-only">{{ tr("遥控器电量") }}</span>
                {{ status.batteryLevel }}%
              </span>
              <span class="topbar-chip" :title="trDynamic('当前输入法')">
                <Mic :size="14" />
                <span class="sr-only">{{ tr("当前输入法") }}</span>
                {{ inputToolLabel }}
              </span>
              <StatusPill
                v-if="voiceActive"
                :label="`${trDynamic('语音')} · ${statusLabel(store.voiceState)}`"
                :tone="voiceTone"
                dot
              />
              <StatusPill
                v-if="audioProblem"
                :label="`${trDynamic('音频')} · ${statusLabel(store.audioState)}`"
                tone="danger"
                dot
              />
            </div>
          </div>
          <div class="topbar-right">
            <TopbarSearch ref="topbarSearch" :commands="paletteCommands" @run="runPaletteCommand" />
          </div>
        </header>
        <div
          ref="scrollArea"
          id="main-content"
          class="main-scroll"
          :class="{ 'scrollbar-visible': scrollbarVisible }"
          tabindex="-1"
          @scroll.passive="onMainScroll"
          @pointermove.passive="onMainPointerMove"
          @pointerleave="onMainPointerLeave"
        >
          <Suspense>
            <component :is="activePage.component" />
            <template #fallback>
              <div class="page">
                <LoadingState label="正在加载…" description="正在加载页面…" />
              </div>
            </template>
          </Suspense>
        </div>
      </main>
    </div>

    <ToastHost />
    <ConfirmDialog />

    <OnboardingWizard v-if="showOnboarding" @complete="finishOnboarding" />
  </div>
</template>

<style scoped>
.window {
  display: flex;
  flex-direction: column;
  height: 100vh;
  overflow: hidden;
  background: var(--bg);
}

.shell {
  display: flex;
  flex: 1;
  gap: var(--space-2);
  min-height: 0;
  overflow: hidden;
  background: var(--bg-sidebar);
  padding: 0 var(--space-2) var(--space-2);
}

.skip-link {
  position: absolute;
  left: 12px;
  top: -60px;
  z-index: 220;
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  padding: 8px 12px;
  color: var(--text);
  text-decoration: none;
}

.skip-link:focus {
  top: calc(var(--titlebar-height) + 8px);
}

.sidebar {
  display: flex;
  flex: 0 0 208px;
  flex-direction: column;
  width: 208px;
  background: transparent;
  padding: var(--space-2);
  transition:
    width var(--duration-base) var(--ease-out),
    flex-basis var(--duration-base) var(--ease-out),
    padding var(--duration-base) var(--ease-out);
}

.sidebar.collapsed {
  flex-basis: 64px;
  width: 64px;
  padding-inline: var(--space-2);
}

.sidebar-brand {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-1) 0 var(--space-4) var(--space-2);
}

.sidebar-brand-copy {
  display: flex;
  min-width: 0;
  flex-direction: column;
  gap: 1px;
}

.sidebar-brand-name {
  overflow: hidden;
  color: var(--text);
  font-size: var(--text-base);
  font-weight: 650;
  letter-spacing: 0.02em;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.sidebar-brand-version {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.sidebar.collapsed .sidebar-brand {
  flex-direction: column;
  justify-content: center;
  gap: 6px;
  padding-inline: 0;
}

.sidebar-collapse {
  display: grid;
  flex: 0 0 26px;
  place-items: center;
  width: 26px;
  height: 26px;
  margin-left: auto;
  padding: 0;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.sidebar-collapse:hover {
  border-color: var(--border);
  background: var(--bg-hover);
  color: var(--text);
}

.sidebar-collapse:focus-visible {
  border-color: var(--accent);
}

.sidebar.collapsed .sidebar-collapse {
  margin-left: 0;
}

.sidebar.collapsed .sidebar-brand-copy,
.sidebar.collapsed .nav-group-label,
.sidebar.collapsed .nav-label {
  display: none;
}

.sidebar.collapsed .nav-item {
  justify-content: center;
  padding: 9px;
}

.nav {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  min-height: 0;
  overflow-y: auto;
}

.nav-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.nav-group-label {
  padding: 0 10px var(--space-1);
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  letter-spacing: 0.08em;
}

.nav-item {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  width: 100%;
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  padding: 8px 10px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-sm);
  text-align: left;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.nav-item:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.nav-item.active {
  background: var(--accent-soft);
  color: var(--text);
  font-weight: 600;
}

.main {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
  border: 1px solid var(--border);
  border-top: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-panel);
  box-shadow: var(--shadow-sm);
}

.topbar {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-4);
  min-height: 56px;
  border-bottom: 1px solid var(--border);
  background: color-mix(in srgb, var(--bg-panel) 92%, transparent);
  padding-block: var(--space-2);
  padding-inline: max(var(--space-6), calc((100% - var(--page-max)) / 2 + var(--space-6)));
  backdrop-filter: blur(10px);
}

.topbar-left {
  display: flex;
  flex: 1;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.topbar-right {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
}

.topbar-status {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
  overflow: hidden;
}

.topbar-device {
  max-width: 180px;
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.topbar-chip {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-subtle);
  padding: 3px 8px;
  color: var(--text-secondary);
  font-size: var(--text-xs);
  white-space: nowrap;
}

.topbar-chip.low {
  border-color: color-mix(in srgb, var(--warning) 35%, var(--border));
  background: color-mix(in srgb, var(--warning) 8%, transparent);
  color: var(--warning-text);
}

.main-scroll {
  /* Be the containing block for absolutely positioned page content so it is
     clipped by this scroller instead of extending the document scroll height.
     Otherwise keyboard focus can scroll the whole shell and leave a blank gap. */
  position: relative;
  flex: 1;
  min-height: 0;
  overflow-y: auto;
  scrollbar-gutter: stable;
  scrollbar-color: transparent transparent;
}

.main-scroll::-webkit-scrollbar-thumb {
  background: transparent;
}

.main-scroll.scrollbar-visible {
  scrollbar-color: var(--border-strong) transparent;
}

.main-scroll.scrollbar-visible::-webkit-scrollbar-thumb {
  background: var(--border-strong);
}

.main-scroll.scrollbar-visible::-webkit-scrollbar-thumb:hover {
  background: var(--text-disabled);
}

.main-scroll:focus-visible {
  outline: none;
}

@media (max-width: 1100px) {
  .topbar-device {
    max-width: 110px;
  }
}

@media (max-width: 900px) {
  .topbar {
    gap: var(--space-2);
    padding-inline: var(--space-5);
  }

  .topbar-device {
    max-width: 88px;
  }
}
</style>
