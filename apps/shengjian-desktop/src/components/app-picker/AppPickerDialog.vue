<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import { AppWindow, RefreshCw, Search, X } from "lucide-vue-next";
import { canonicalizeAppTarget, errorMessage } from "../../lib/api";
import {
  appTargetStatus,
  displayNameOf,
  executableNameFromPath,
  subtitleOf,
  type AppTargetMode,
} from "../../lib/appTargets";
import { tr, trDynamic } from "../../lib/i18n";
import { RUNNABLE_EXTENSIONS } from "../../lib/runnable";
import { pushToast } from "../../lib/toast";
import type { AppTargetInfo, LaunchableApp } from "../../lib/types";
import {
  appIconFor,
  cachedAppTarget,
  describeAppTargetsCached,
  loadAppIcons,
  useAppPicker,
} from "../../composables/useAppPicker";
import Button from "../ui/Button.vue";
import IconButton from "../ui/IconButton.vue";
import TextInput from "../ui/TextInput.vue";
import ToggleGroup from "../ui/ToggleGroup.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    mode?: AppTargetMode;
    currentTarget?: string;
    foregroundApp?: string;
    recentTargets?: string[];
    title?: string;
    /** 覆盖默认副标题；不同入口的用途不同（自动匹配 / 新建方案）。 */
    description?: string;
  }>(),
  {
    mode: "launch",
    currentTarget: "",
    foregroundApp: "",
    recentTargets: () => [],
    title: "",
    description: "",
  },
);

const emit = defineEmits<{
  /** 选中目标；第二个参数是显示名（新方案默认拿它当名字）。 */
  select: [target: string, displayName?: string];
  close: [];
}>();

type BadgeTone = "neutral" | "warning" | "danger";

interface PickerBadge {
  label: string;
  tone: BadgeTone;
  title?: string;
}

interface PickerOptionSeed {
  key: string;
  displayName: string;
  subtitle: string;
  description: string;
  target: string;
  iconTarget: string;
  matchTarget: string | null;
  disabled: boolean;
  badges: PickerBadge[];
  search: string;
}

interface PickerOption extends PickerOptionSeed {
  id: string;
  flatIndex: number;
}

const picker = useAppPicker();
const query = ref("");
const activeIndex = ref(0);
const manualOpen = ref(false);
const manualValue = ref("");
const manualInfo = shallowRef<AppTargetInfo | null>(null);
const manualError = shallowRef<string | null>(null);
const manualChecking = ref(false);
/** 列表分类：全部 / 推荐（预设）/ 已安装应用。 */
const category = ref("all");
const CATEGORY_ITEMS = [
  { value: "all", label: "全部" },
  { value: "suggested", label: "推荐" },
  { value: "installed", label: "已安装应用" },
] as const;

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const searchRef = useTemplateRef<HTMLInputElement>("searchRef");
const optionRefs = ref<HTMLElement[]>([]);
let previousFocus: HTMLElement | null = null;
let manualTimer: number | undefined;
let manualSeq = 0;

const isMatchMode = computed(() => props.mode === "match");
const resolvedTitle = computed(
  () => props.title.trim() || (isMatchMode.value ? tr("选择用于自动匹配的应用") : tr("选择应用")),
);

function badgeTitle(status: ReturnType<typeof appTargetStatus>): string | undefined {
  if (!status.detailKey) return undefined;
  return tr(status.detailKey, status.reasonKey ? { reason: tr(status.reasonKey) } : undefined);
}

function appSeed(app: LaunchableApp): PickerOptionSeed {
  const match = isMatchMode.value;
  const executable = app.executableName?.trim() || "";
  const matchTarget = executable || null;
  const badges: PickerBadge[] = [];
  if (!app.installed) {
    badges.push({ label: tr("未安装"), tone: "warning" });
  }
  if (match) {
    if (!app.matchable || !matchTarget) {
      badges.push({
        label: tr("不支持自动匹配"),
        tone: "neutral",
        title: tr("该应用不支持自动匹配"),
      });
    }
  } else if (app.installed && !app.matchable) {
    badges.push({
      label: tr("仅启动"),
      tone: "neutral",
      title: tr("仅支持启动，不会自动切换方案"),
    });
  }
  const subtitle =
    executable && executable !== app.displayName
      ? executable
      : app.target !== app.displayName
        ? app.target
        : "";
  return {
    key: `${match ? "match" : "launch"}:${(match ? (matchTarget ?? app.target) : app.target).toLowerCase()}`,
    displayName: app.displayName?.trim() || matchTarget || app.target,
    subtitle,
    description: app.target,
    target: app.target,
    iconTarget: app.target,
    matchTarget,
    disabled: match ? !app.installed || !app.matchable || !matchTarget : !app.installed,
    badges,
    search: [app.displayName, executable, app.target, app.appId].join(" ").toLowerCase(),
  };
}

function targetSeed(raw: string): PickerOptionSeed {
  const target = raw.trim();
  const info = cachedAppTarget(target);
  const match = isMatchMode.value;
  const executable = info?.executableName?.trim() || "";
  const display = displayNameOf(info) || target;
  const subtitle = subtitleOf(info) || (target !== display ? target : "");
  const matchTarget = match ? executable || executableNameFromPath(target) || null : null;
  const status = appTargetStatus(info, match ? "match" : "launch");
  const badges: PickerBadge[] = [];
  if (status.labelKey) {
    badges.push({
      label: tr(status.labelKey),
      tone: status.tone === "danger" ? "danger" : status.tone === "warning" ? "warning" : "neutral",
      title: badgeTitle(status),
    });
  }
  // Launch 模式必须等 describe 返回规范 target 才允许确认：描述未返回时点选会
  // 把旧的裸 exe 名原样写回，正是本次要修的问题。Match 模式的匹配键本来就是
  // exe 名，未描述时也能安全选中。
  const disabled = match ? !matchTarget || info?.matchable === false : !info || !info.launchable;
  return {
    key: `${match ? "match" : "launch"}:${(match ? (matchTarget ?? target) : target).toLowerCase()}`,
    displayName: display,
    subtitle,
    description: info?.target?.trim() || target,
    target: info?.target?.trim() || target,
    iconTarget: info?.target?.trim() || target,
    matchTarget,
    disabled,
    badges,
    search: [display, executable, target, info?.target ?? ""].join(" ").toLowerCase(),
  };
}

/** 当前分类 + 搜索过滤后的统一列表：分类切换就是分组，不再单独渲染标题。 */
const flatItems = computed<PickerOption[]>(() => {
  const needle = query.value.trim().toLowerCase();
  const used = new Set<string>();
  const collected: PickerOptionSeed[] = [];

  function add(seeds: PickerOptionSeed[], dedupe = true) {
    for (const seed of seeds) {
      if (dedupe && used.has(seed.key)) continue;
      if (needle && !seed.search.includes(needle)) continue;
      used.add(seed.key);
      collected.push(seed);
    }
  }

  const presets = picker.apps.value.filter((app) => app.source === "preset").map(appSeed);
  const installed = picker.apps.value
    .filter((app) => app.source !== "preset" && app.installed)
    .map(appSeed);

  if (category.value === "suggested") {
    add(presets);
  } else if (category.value === "installed") {
    add(installed);
  } else {
    // 全部：当前前台 → 推荐（预设）→ 已安装 → 最近使用，去重后合成一个列表。
    const foreground = props.foregroundApp.trim();
    if (foreground) add([targetSeed(foreground)], false);
    add(presets);
    add(installed);
    const recentSeeds: PickerOptionSeed[] = [];
    const recentSeen = new Set<string>();
    for (const target of props.recentTargets) {
      const seed = targetSeed(target);
      if (recentSeen.has(seed.key)) continue;
      recentSeen.add(seed.key);
      recentSeeds.push(seed);
    }
    add(recentSeeds);
  }

  return collected.map((seed, index) => ({
    ...seed,
    id: `app-picker-option-${index}`,
    flatIndex: index,
  }));
});
const activeItem = computed(() => flatItems.value[activeIndex.value]);
const activeDescendant = computed(() => activeItem.value?.id);

const manualStatus = computed(() => appTargetStatus(manualInfo.value, props.mode));
const manualDetail = computed(() => badgeTitle(manualStatus.value) ?? "");
const manualConfirmDisabled = computed(() => {
  if (manualChecking.value) return true;
  const info = manualInfo.value;
  if (!info) return true;
  if (isMatchMode.value) return !info.matchable || !info.executableName?.trim();
  return !info.launchable;
});
const manualMessage = computed(() => {
  if (manualError.value) return "";
  const status = manualStatus.value;
  if (status.labelKey) return manualDetail.value;
  return manualInfo.value ? tr("目标有效，可以保存") : "";
});
const manualHasProblem = computed(
  () =>
    Boolean(manualError.value) ||
    manualStatus.value.tone === "danger" ||
    manualStatus.value.tone === "warning",
);

watch(flatItems, () => {
  const items = flatItems.value;
  if (items.length === 0) {
    activeIndex.value = 0;
    return;
  }
  const current = items[activeIndex.value];
  if (current && !current.disabled) return;
  const firstEnabled = items.findIndex((item) => !item.disabled);
  activeIndex.value = firstEnabled >= 0 ? firstEnabled : 0;
});

watch(
  flatItems,
  (items) => {
    void loadAppIcons(items.map((item) => item.iconTarget).filter(Boolean));
  },
  { immediate: true },
);

watch(query, () => {
  activeIndex.value = 0;
});

watch(category, () => {
  activeIndex.value = 0;
});

function ensureActiveVisible() {
  const item = activeItem.value;
  if (!item) return;
  void nextTick(() => optionRefs.value[item.flatIndex]?.scrollIntoView?.({ block: "nearest" }));
}

function move(delta: number) {
  const items = flatItems.value;
  if (items.length === 0) return;
  let index = activeIndex.value;
  for (let step = 0; step < items.length; step += 1) {
    index = (index + delta + items.length) % items.length;
    if (!items[index].disabled) break;
  }
  activeIndex.value = index;
  ensureActiveVisible();
}

function requestClose() {
  emit("close");
}

function choose(option: PickerOption | undefined) {
  if (!option || option.disabled) return;
  const value = isMatchMode.value ? option.matchTarget : option.target;
  if (!value) return;
  emit("select", value, option.displayName);
  requestClose();
}

function onSearchKeydown(event: KeyboardEvent) {
  if (event.key === "ArrowDown") {
    event.preventDefault();
    move(1);
  } else if (event.key === "ArrowUp") {
    event.preventDefault();
    move(-1);
  } else if (event.key === "Enter") {
    event.preventDefault();
    choose(activeItem.value);
  }
}

function focusableElements(): HTMLElement[] {
  if (!dialogRef.value) return [];
  return Array.from(
    dialogRef.value.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((element) => !element.hasAttribute("disabled") && element.offsetParent !== null);
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    event.stopPropagation();
    requestClose();
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

function primeDescriptions() {
  const targets = [props.currentTarget, props.foregroundApp, ...props.recentTargets]
    .map((target) => target?.trim() ?? "")
    .filter(Boolean);
  if (targets.length > 0) void describeAppTargetsCached(targets);
}

function resetForOpen() {
  query.value = "";
  category.value = "all";
  activeIndex.value = 0;
  manualOpen.value = false;
  manualValue.value = "";
  manualInfo.value = null;
  manualError.value = null;
  manualChecking.value = false;
  manualSeq += 1;
  window.clearTimeout(manualTimer);
}

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      window.addEventListener("keydown", onKeydown, true);
      resetForOpen();
      await nextTick();
      searchRef.value?.focus();
      if (!picker.appsLoaded.value && !picker.appsLoading.value) void picker.loadApps();
      primeDescriptions();
    } else {
      window.removeEventListener("keydown", onKeydown, true);
      window.clearTimeout(manualTimer);
      previousFocus?.focus();
      previousFocus = null;
    }
  },
  { immediate: true },
);

function refreshApps() {
  void picker.refreshApps();
}

async function browse() {
  try {
    const selected = await openFileDialog({
      title: tr("选择程序"),
      multiple: false,
      filters: [{ name: tr("可运行文件"), extensions: RUNNABLE_EXTENSIONS }],
    });
    if (!selected || Array.isArray(selected)) return;
    if (isMatchMode.value) {
      const info = await canonicalizeAppTarget(selected).catch(() => null);
      const name = info?.executableName?.trim() || executableNameFromPath(selected);
      if (!name) {
        pushUnmatchable();
        return;
      }
      emit("select", name, info?.displayName ?? name);
      requestClose();
      return;
    }
    let info: AppTargetInfo;
    try {
      info = await canonicalizeAppTarget(selected);
    } catch (cause) {
      pushUnavailable(errorMessage(cause));
      return;
    }
    if (!info.launchable) {
      pushUnavailable(infoDetail(info));
      return;
    }
    emit("select", info.target, info.displayName ?? undefined);
    requestClose();
  } catch (cause) {
    pushUnavailable(errorMessage(cause));
  }
}

function pushUnmatchable() {
  pushToast(tr("该应用不支持自动匹配"), "warning");
}

function pushUnavailable(reason: string) {
  pushToast(tr("应用不可用：{reason}", { reason }), "error");
}

function infoDetail(info: AppTargetInfo): string {
  return badgeTitle(appTargetStatus(info, isMatchMode.value ? "match" : "launch")) ?? info.target;
}

function toggleManual() {
  manualOpen.value = !manualOpen.value;
  if (!manualOpen.value) return;
  if (!manualValue.value && props.currentTarget.trim()) {
    manualValue.value = props.currentTarget.trim();
  }
  scheduleManualCheck(true);
}

function scheduleManualCheck(immediate = false) {
  window.clearTimeout(manualTimer);
  manualTimer = window.setTimeout(
    () => {
      void runManualCheck();
    },
    immediate ? 0 : 250,
  );
}

async function runManualCheck() {
  const raw = manualValue.value.trim();
  const sequence = ++manualSeq;
  manualError.value = null;
  if (!raw) {
    manualInfo.value = null;
    manualChecking.value = false;
    return;
  }
  manualChecking.value = true;
  try {
    const info = await canonicalizeAppTarget(raw);
    if (sequence === manualSeq) manualInfo.value = info;
  } catch (cause) {
    if (sequence === manualSeq) {
      manualInfo.value = null;
      manualError.value = errorMessage(cause);
    }
  } finally {
    if (sequence === manualSeq) manualChecking.value = false;
  }
}

watch(manualValue, () => {
  if (manualOpen.value) scheduleManualCheck();
});

function confirmManual() {
  const info = manualInfo.value;
  if (!info || manualConfirmDisabled.value) return;
  if (isMatchMode.value) {
    const name = info.executableName?.trim();
    if (!name) return;
    emit("select", name, info.displayName ?? name);
  } else {
    emit("select", info.target, info.displayName ?? undefined);
  }
  requestClose();
}

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  window.clearTimeout(manualTimer);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="app-picker-backdrop" @click.self="requestClose">
      <section
        ref="dialogRef"
        class="app-picker"
        role="dialog"
        aria-modal="true"
        aria-labelledby="app-picker-title"
      >
        <header class="app-picker-head">
          <div class="app-picker-heading">
            <h2 id="app-picker-title">{{ resolvedTitle }}</h2>
            <p>
              {{
                props.description.trim()
                  ? trDynamic(props.description)
                  : isMatchMode
                    ? tr("选择用于自动匹配的应用（按可执行文件名匹配前台）")
                    : tr("选择要启动的应用；旧目标会在这里显示问题并可修复。")
              }}
            </p>
          </div>
          <div class="app-picker-head-actions">
            <IconButton
              size="sm"
              :label="tr('刷新')"
              :disabled="picker.appsLoading.value"
              @click="refreshApps"
            >
              <RefreshCw :size="14" />
            </IconButton>
            <IconButton size="sm" :label="tr('关闭')" @click="requestClose">
              <X :size="16" />
            </IconButton>
          </div>
        </header>

        <div class="app-picker-search">
          <Search :size="14" class="app-picker-search-icon" aria-hidden="true" />
          <input
            ref="searchRef"
            v-model="query"
            type="text"
            role="combobox"
            aria-expanded="true"
            aria-autocomplete="list"
            aria-haspopup="listbox"
            aria-controls="app-picker-listbox"
            :aria-activedescendant="activeDescendant"
            :aria-label="tr('搜索应用…')"
            :placeholder="tr('搜索应用…')"
            @keydown="onSearchKeydown"
          />
          <kbd>Esc</kbd>
        </div>

        <div v-if="picker.appsLoading.value && !picker.appsLoaded.value" class="app-picker-status">
          {{ tr("加载应用列表…") }}
        </div>
        <div
          v-else-if="picker.appsError.value && !picker.appsLoaded.value"
          class="app-picker-status error"
          role="alert"
        >
          <span>{{ tr("无法加载应用列表") }}</span>
          <Button size="sm" @click="refreshApps">{{ tr("重试") }}</Button>
        </div>
        <div v-if="picker.appsScanError.value" class="app-picker-status warning" role="status">
          <span>{{ tr("扫描已安装应用失败，可重试") }}</span>
          <Button size="sm" @click="refreshApps">{{ tr("重试") }}</Button>
        </div>

        <div class="app-picker-panel">
          <div class="app-picker-filters">
            <ToggleGroup v-model="category" :items="CATEGORY_ITEMS" aria-label="应用分类" />
          </div>
          <div
            id="app-picker-listbox"
            class="app-picker-list"
            role="listbox"
            :aria-label="tr('应用列表')"
          >
            <button
              v-for="option in flatItems"
              :id="option.id"
              :key="option.id"
              :ref="
                (element) => {
                  if (element) optionRefs[option.flatIndex] = element as HTMLElement;
                }
              "
              type="button"
              role="option"
              tabindex="-1"
              class="app-picker-option"
              :class="{ active: option.flatIndex === activeIndex, disabled: option.disabled }"
              :aria-selected="option.flatIndex === activeIndex"
              :aria-disabled="option.disabled || undefined"
              :title="option.description"
              @mouseenter="activeIndex = option.flatIndex"
              @click="choose(option)"
            >
              <span class="app-picker-icon" aria-hidden="true">
                <img
                  v-if="appIconFor(option.iconTarget)"
                  :src="appIconFor(option.iconTarget)"
                  alt=""
                />
                <AppWindow v-else :size="16" />
              </span>
              <span class="app-picker-option-copy">
                <strong>{{ option.displayName }}</strong>
                <small v-if="option.subtitle">{{ option.subtitle }}</small>
              </span>
              <span
                v-for="badge in option.badges"
                :key="badge.label"
                class="app-picker-flag"
                :class="`tone-${badge.tone}`"
                :title="badge.title"
              >
                {{ badge.label }}
              </span>
            </button>
            <p
              v-if="
                flatItems.length === 0 && !(picker.appsLoading.value && !picker.appsLoaded.value)
              "
              class="app-picker-empty"
            >
              {{ query.trim() ? tr("没有匹配的应用") : tr("没有可用的应用") }}
            </p>
          </div>
        </div>

        <footer class="app-picker-foot">
          <div class="app-picker-foot-actions">
            <Button size="sm" @click="browse">{{ tr("浏览文件…") }}</Button>
            <Button size="sm" variant="ghost" :aria-expanded="manualOpen" @click="toggleManual">
              {{ tr("手动填写（高级）") }}
            </Button>
          </div>
          <div v-if="manualOpen" class="app-picker-manual">
            <TextInput
              v-model="manualValue"
              :placeholder="tr('输入程序完整路径')"
              :aria-label="tr('手动填写（高级）')"
            />

            <div class="app-picker-manual-foot">
              <span
                class="app-picker-manual-state"
                :class="{ error: manualHasProblem }"
                role="status"
              >
                <template v-if="manualChecking">{{ tr("校验中…") }}</template>
                <template v-else-if="manualError">
                  {{ tr("应用不可用：{reason}", { reason: manualError }) }}
                </template>
                <template v-else-if="manualMessage">{{ manualMessage }}</template>
                <template v-else>{{ tr("输入后会自动校验") }}</template>
              </span>
              <Button
                size="sm"
                variant="primary"
                :disabled="manualConfirmDisabled"
                @click="confirmManual"
              >
                {{ tr("确认") }}
              </Button>
            </div>
          </div>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.app-picker-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  background: var(--overlay);
  padding: var(--space-5);
  backdrop-filter: blur(4px);
}

.app-picker {
  display: flex;
  flex-direction: column;
  width: min(560px, calc(100vw - 48px));
  height: min(640px, calc(100vh - 48px));
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.app-picker-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.app-picker-heading {
  min-width: 0;
}

.app-picker-heading h2 {
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 650;
  letter-spacing: -0.02em;
}

.app-picker-heading p {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.45;
}

.app-picker-head-actions {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: 4px;
}

.app-picker-search {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: var(--space-3) var(--space-5) 0;
  border: 1px solid var(--border-control);
  border-radius: var(--radius-md);
  background: var(--bg);
  padding: 0 9px;
  color: var(--text-muted);
}

.app-picker-search:focus-within {
  border-color: var(--accent);
}

.app-picker-search input {
  flex: 1;
  min-width: 0;
  border: 0;
  outline: none;
  background: transparent;
  padding: 8px 0;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
}

.app-picker-search kbd {
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-subtle);
  padding: 1px 5px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
}

.app-picker-status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin: var(--space-3) var(--space-5) 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-2) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.app-picker-status.error {
  border-color: var(--danger-soft);
  color: var(--danger-text);
}

.app-picker-status.warning {
  border-color: var(--warning);
  color: var(--warning-text);
}

.app-picker-panel {
  display: flex;
  flex: 1 1 0;
  flex-direction: column;
  min-height: 150px;
  overflow: hidden;
  margin: var(--space-3) var(--space-5) 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
}

.app-picker-filters {
  flex: 0 0 auto;
  overflow-x: auto;
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) var(--space-3);
}

.app-picker-list {
  flex: 1 1 0;
  min-height: 0;
  overflow-y: auto;
}

.app-picker-option {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 6px;
  width: 100%;
  border: 0;
  border-top: 1px solid var(--border);
  border-radius: 0;
  background: transparent;
  padding: 9px 10px;
  text-align: left;
  cursor: pointer;
}

.app-picker-option:first-child {
  border-top: 0;
}

.app-picker-option.active {
  background: var(--accent-soft);
  box-shadow: inset 3px 0 0 var(--accent);
}

.app-picker-option.disabled {
  cursor: default;
  opacity: 0.55;
}

.app-picker-option:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.app-picker-icon {
  display: grid;
  flex: 0 0 20px;
  place-items: center;
  width: 20px;
  height: 20px;
  color: var(--text-muted);
}

.app-picker-icon img {
  width: 18px;
  height: 18px;
  border-radius: 4px;
  object-fit: contain;
}

.app-picker-option-copy {
  display: flex;
  flex: 1 1 160px;
  flex-direction: column;
  min-width: 0;
}

.app-picker-option-copy strong {
  overflow: hidden;
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-picker-option.active .app-picker-option-copy strong {
  color: var(--accent-ink);
}

.app-picker-option-copy small {
  overflow: hidden;
  margin-top: 1px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-picker-flag {
  flex: 0 0 auto;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-subtle);
  padding: 1px 7px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  white-space: nowrap;
}

.app-picker-flag.tone-warning {
  border-color: var(--warning);
  color: var(--warning-text);
}

.app-picker-flag.tone-danger {
  border-color: var(--danger-soft);
  color: var(--danger-text);
}

.app-picker-empty {
  margin: 0;
  padding: var(--space-5) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-sm);
  text-align: center;
}

.app-picker-foot {
  margin-top: var(--space-2);
  padding: var(--space-2) var(--space-5) var(--space-3);
}

.app-picker-foot-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: space-between;
  gap: var(--space-2);
}

.app-picker-manual {
  margin-top: var(--space-3);
}

.app-picker-manual-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin-top: var(--space-2);
}

.app-picker-manual-state {
  flex: 1 1 200px;
  min-width: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  overflow-wrap: anywhere;
}

.app-picker-manual-state.error {
  color: var(--warning-text);
}
</style>
