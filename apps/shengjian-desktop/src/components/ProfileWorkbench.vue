<script setup lang="ts">
import { computed, ref, shallowRef, watch } from "vue";
import { Search } from "lucide-vue-next";
import rc003Image from "../assets/devices/xiaomi-rc003.png";
import rc001Image from "../assets/devices/xiaomi-rc001.png";
import arn9Image from "../assets/devices/xiaomi-arn9.png";
import {
  PROFILE_BUTTONS,
  PROFILE_TRIGGERS,
  profileButtonsForModel,
  type ProfileDraft,
  type ProfileTrigger,
} from "../lib/buttons";
import { statusLabel, tr, trDynamic } from "../lib/i18n";
import { formatShortcut } from "../lib/shortcuts";
import { systemKeyLabel } from "../lib/systemActions";
import {
  appTargetFriendlyName,
  cachedAppTarget,
  describeAppTargetsCached,
} from "../composables/useAppPicker";
import type { LogicalAction, LogicalButton, MacroDefinition } from "../lib/types";
import ProfileBindingEditor from "./ProfileBindingEditor.vue";
import StatusPill from "./ui/StatusPill.vue";
import Switch from "./ui/Switch.vue";

const props = defineProps<{
  view: "workbench" | "table";
  busy: boolean;
  dirty: boolean;
  baselineJson: string;
  foregroundApp: string;
  commonPhrasesAvailable: boolean;
  latestMacros: MacroDefinition[];
  /** Whether a remote is actually connected right now. */
  remoteConnected: boolean;
  /** Connected remote model, when known (e.g. "RC003"). */
  remoteModel: string | null;
  /** 设备上报的蓝牙名（型号识别不出来时的兜底显示）。 */
  remoteName: string | null;
  /** 按键增强（RC003 增强捕获）当前是否开启。 */
  enhancedCapture: boolean;
  /** 机型是否支持按键增强（仅 RC003）。 */
  enhancedCaptureEnabled: boolean;
  /** 当前方案是否自动应用。 */
  autoApply: boolean;
  /** 默认方案不能自动应用。 */
  autoApplyEnabled: boolean;
  /** 当前方案是否已激活。 */
  activationActive: boolean;
  /** 默认方案始终生效，不需要也不能手动激活。 */
  activationEnabled: boolean;
  /** 开关是否不可操作（无已保存方案或正忙）。 */
  toggleDisabled: boolean;
}>();

const emit = defineEmits<{
  error: [message: string];
  toggleEnhancedCapture: [value: boolean];
  toggleAutoApply: [value: boolean];
  toggleActivation: [value: boolean];
}>();

const draft = defineModel<ProfileDraft>("draft", { required: true });
const selectedButton = ref<LogicalButton>("home");

const selectedTrigger = ref<ProfileTrigger>("single");
const query = shallowRef("");
const onlyConfigured = shallowRef(false);

const CAPS: Record<string, string> = {
  power: "⏻",
  up: "↑",
  down: "↓",
  left: "←",
  right: "→",
  ok: "OK",
  back: "↩",
  home: "⌂",
  tv: "TV",
  menu: "☰",
  "volume-up": "+",
  "volume-down": "−",
};

// 触发总数必须跟着型号走：ARN9 无 TV 键，是 11×3 而不是 12×3。
const totalTriggers = computed(
  () => profileButtonsForModel(props.remoteModel).length * PROFILE_TRIGGERS.length,
);

interface RemoteHotspot {
  id: LogicalButton;
  x: number;
  y: number;
  w: number;
  h: number;
  shape: "circle" | "zone";
}

// x/y 是热区**中心**（CSS 用 translate(-50%,-50%)），w/h 是尺寸，均为图片宽高的百分比。
// 只标逻辑按键；硬件麦克风在产品图上可见但不是方案目标。
// 百分比是按各自产品图单独量的——两张图尺寸不同，不能共用一套坐标。
const RC003_HOTSPOTS: RemoteHotspot[] = [
  { id: "power", x: 29.55, y: 9.3, w: 17.6, h: 5.55, shape: "circle" },
  { id: "up", x: 49.85, y: 17.5, w: 17, h: 4.8, shape: "circle" },
  { id: "down", x: 49.85, y: 31.2, w: 17, h: 4.8, shape: "circle" },
  { id: "left", x: 27.76, y: 24.33, w: 17, h: 5.3, shape: "circle" },
  { id: "right", x: 71.94, y: 24.33, w: 17, h: 5.3, shape: "circle" },
  { id: "ok", x: 49.85, y: 24.33, w: 26, h: 8.05, shape: "circle" },
  { id: "back", x: 33.4, y: 39.21, w: 25.4, h: 7.86, shape: "circle" },
  { id: "home", x: 33.4, y: 48.33, w: 25.4, h: 7.86, shape: "circle" },
  { id: "menu", x: 33.4, y: 57.42, w: 25.4, h: 7.86, shape: "circle" },
  { id: "volume-up", x: 65.67, y: 39.21, w: 25.4, h: 7.86, shape: "circle" },
  { id: "volume-down", x: 65.67, y: 48.33, w: 25.4, h: 7.86, shape: "circle" },
  { id: "tv", x: 65.67, y: 57.42, w: 25.4, h: 7.86, shape: "circle" },
];

/**
 * ARN9 普通款：514 x 1640 产品图。
 *
 * 坐标与直径都来自实测，不是目测：
 * - D-pad 用径向亮度剖面（64 个角度取平均以抑制机身纹理）定位两级圆盘：
 *   圆心 (260, 492)、外盘 r=160、内盘 r=65。方向键取环带中径 (160+65)/2 = 112.5
 *   上四个正方向的点
 * - 小按键的圆盘直径用径向剖面找盘缘的暗环：电源 92px，主页/返回/菜单 80px
 * - 热区直径**等于按键圆盘直径**（与 RC003/RC001 的做法一致：那些型号的热区
 *   25.4%×7.86% 正好等于其 84px 盘径），这样圈与按键严丝合缝
 * - 四方向键区取 80px：环带厚度 95px，两侧各留 7.5px 余量，不顶到内外边界
 *
 * 与 RC003 的布局差异：没有 TV 键；Home / 返回 / 菜单 是同一行的三颗独立键；
 * 音量是一根上下摇杆（+ 上半、− 下半）。
 */
const ARN9_HOTSPOTS: RemoteHotspot[] = [
  { id: "power", x: 50.6, y: 7.47, w: 17.9, h: 5.61, shape: "circle" },
  { id: "up", x: 50.58, y: 23.14, w: 15.56, h: 4.88, shape: "circle" },
  { id: "down", x: 50.58, y: 36.86, w: 15.56, h: 4.88, shape: "circle" },
  { id: "left", x: 28.7, y: 30, w: 15.56, h: 4.88, shape: "circle" },
  { id: "right", x: 72.47, y: 30, w: 15.56, h: 4.88, shape: "circle" },
  { id: "ok", x: 50.58, y: 30, w: 24.9, h: 7.8, shape: "circle" },
  { id: "home", x: 26.77, y: 44.7, w: 15.56, h: 4.88, shape: "circle" },
  { id: "back", x: 50.73, y: 44.7, w: 15.56, h: 4.88, shape: "circle" },
  { id: "menu", x: 74.27, y: 44.7, w: 15.56, h: 4.88, shape: "circle" },
  { id: "volume-up", x: 50.61, y: 51.96, w: 15.56, h: 4.88, shape: "circle" },
  { id: "volume-down", x: 50.4, y: 60.27, w: 15.56, h: 4.88, shape: "circle" },
];

const modelKey = computed(() => (props.remoteModel ?? "").trim().toUpperCase());
const remoteImage = computed(() => {
  if (modelKey.value === "ARN9") return arn9Image;
  if (modelKey.value === "RC001") return rc001Image;
  return rc003Image;
});
/**
 * RC001 与 RC003 共用同一套热区。
 *
 * 两张产品图已统一到同一画布与机身尺寸（335x1081，机身 bbox 同为
 * (35,30)-(297,1059)），实测按键位置偏差 <=2px（home/menu/back/tv/ok 均为 0.1~0.2px），
 * 所以不单独维护一份，避免两边坐标悄悄漂移。
 */
const REMOTE_HOTSPOTS = computed(() =>
  modelKey.value === "ARN9" ? ARN9_HOTSPOTS : RC003_HOTSPOTS,
);
const remoteAlt = computed(() => {
  if (modelKey.value === "ARN9") return tr("小米蓝牙语音遥控器");
  if (modelKey.value === "RC001") return tr("小米蓝牙语音遥控器 2");
  return tr("小米蓝牙语音遥控器 2 Pro");
});
/** 产品图说明：每张图各自对位，不要写成同一句。 */
const productNote = computed(() => {
  if (modelKey.value === "ARN9") return tr("按 ARN9 产品图对位；语音键为硬件键。");
  if (modelKey.value === "RC001") return tr("按 RC001 产品图对位；语音键为硬件键。");
  return tr("按 RC003 产品图对位；语音键为硬件键。");
});

function hotspotStyle(hotspot: RemoteHotspot) {
  return {
    left: `${hotspot.x}%`,
    top: `${hotspot.y}%`,
    width: `${hotspot.w}%`,
    height: `${hotspot.h}%`,
  };
}

const labelFor = (id: LogicalButton) =>
  PROFILE_BUTTONS.find((button) => button.id === id)?.label ?? id;

function actionFor(button: LogicalButton, trigger: ProfileTrigger): LogicalAction | undefined {
  return draft.value.bindings[button]?.[trigger];
}

/** 当前草稿里所有 open-app 目标（去重）：用于主动查询友好名，摘要优先显示中文名。 */
const openAppTargets = computed<string[]>(() => {
  const targets: string[] = [];
  const seen = new Set<string>();
  for (const triggers of Object.values(draft.value.bindings ?? {})) {
    for (const action of Object.values(triggers ?? {})) {
      if (action?.type !== "open-app" || typeof action.app !== "string") continue;
      const target = action.app.trim();
      if (!target) continue;
      const key = target.toLowerCase();
      if (seen.has(key)) continue;
      seen.add(key);
      targets.push(target);
    }
  }
  return targets;
});

watch(
  openAppTargets,
  (targets) => {
    if (targets.length > 0) void describeAppTargetsCached(targets);
  },
  { immediate: true },
);

/** 搜索索引：摘要之外再匹配原始 target 与解析出的 exe 名。 */
function searchTextFor(button: LogicalButton, trigger: ProfileTrigger): string {
  const action = actionFor(button, trigger);
  if (!action) return "";
  const parts = [summary(button, trigger)];
  if (action.type === "open-app" && typeof action.app === "string") {
    const target = action.app.trim();
    if (target) {
      parts.push(target);
      const info = cachedAppTarget(target);
      if (info?.displayName) parts.push(info.displayName);
      if (info?.executableName) parts.push(info.executableName);
    }
  }
  return parts.join(" ");
}

function summary(button: LogicalButton, trigger: ProfileTrigger): string {
  const action = actionFor(button, trigger);
  if (!action) return "";
  switch (action.type) {
    case "shortcut":
      return Array.isArray(action.keys) && action.keys.length
        ? formatShortcut(action.keys)
        : "快捷键";
    case "open-app": {
      const target = typeof action.app === "string" ? action.app.trim() : "";
      // 描述命中前先回退原串；缓存是响应式的，describe 回来后会重算。
      return target ? appTargetFriendlyName(target, target) : trDynamic("打开应用");
    }
    case "run-macro":
      return (
        props.latestMacros.find((macro) => macro.macroId === action.macro_id)?.name ?? "运行宏"
      );
    case "system-action": {
      const key = typeof action.key === "string" ? action.key.trim() : "";
      return key ? systemKeyLabel(key) : trDynamic("系统动作");
    }
    case "focus-input":
      return trDynamic("聚焦输入");
    case "disabled":
      return trDynamic("禁用");
    case "common-phrase":
      return trDynamic("常用语");
    case "mouse":
      return trDynamic("鼠标");
    default:
      return trDynamic("其他（保留）");
  }
}

function bindingCount(): number {
  return Object.values(draft.value.bindings ?? {}).reduce(
    (total, triggers) => total + Object.keys(triggers ?? {}).length,
    0,
  );
}

function hasBinding(button: LogicalButton): boolean {
  return PROFILE_TRIGGERS.some((trigger) => Boolean(actionFor(button, trigger)));
}

function isButtonModified(button: LogicalButton): boolean {
  if (!props.baselineJson) return false;
  try {
    const baseline = JSON.parse(props.baselineJson) as ProfileDraft;
    return (
      JSON.stringify(draft.value.bindings[button] ?? null) !==
      JSON.stringify(baseline.bindings?.[button] ?? null)
    );
  } catch {
    return false;
  }
}

const visibleButtons = computed(() => {
  const value = query.value.trim().toLowerCase();
  // 该型号没有的按键（ARN9 无 TV 键）不列出来，避免让人去配一个按不到的键。
  return profileButtonsForModel(props.remoteModel).filter((button) => {
    if (
      onlyConfigured.value &&
      !PROFILE_TRIGGERS.some((trigger) => actionFor(button.id, trigger))
    ) {
      return false;
    }
    if (!value) return true;
    const actionText = PROFILE_TRIGGERS.map((trigger) => searchTextFor(button.id, trigger)).join(
      " ",
    );
    return `${button.label} ${button.id} ${actionText}`.toLowerCase().includes(value);
  });
});

function selectButton(button: LogicalButton) {
  selectedButton.value = button;
}

function selectTrigger(trigger: ProfileTrigger) {
  selectedTrigger.value = trigger;
}

function selectCell(button: LogicalButton, trigger: ProfileTrigger) {
  selectedButton.value = button;
  selectedTrigger.value = trigger;
}
</script>

<template>
  <div class="profile-workbench">
    <template v-if="view === 'workbench'">
      <aside class="remote-panel panel">
        <!-- 不再单列「当前遥控器」标签：型号识别得出来时胶囊里是产品正式名，
             名称本身就说明了这是当前遥控器，省下的宽度让长名称能单行放下。 -->
        <!-- 产品图上方工具栏：按键增强 / 自动应用 / 方案激活，三个开关。 -->
        <div class="panel-head remote-toolbar">
          <span class="toolbar-toggle" :title="tr('仅 RC003 支持按键增强')">
            <span class="toolbar-toggle-label">{{ tr("按键增强") }}</span>
            <Switch
              size="sm"
              :model-value="enhancedCapture"
              :disabled="!enhancedCaptureEnabled || toggleDisabled"
              :label="tr('按键增强')"
              @update:model-value="emit('toggleEnhancedCapture', $event)"
            />
          </span>
          <span class="toolbar-toggle" :title="tr('前台应用匹配时自动切换到本方案')">
            <span class="toolbar-toggle-label">{{ tr("自动应用") }}</span>
            <Switch
              size="sm"
              :model-value="autoApply"
              :disabled="!autoApplyEnabled || toggleDisabled"
              :label="tr('自动应用')"
              @update:model-value="emit('toggleAutoApply', $event)"
            />
          </span>
          <span class="toolbar-toggle" :title="tr('手动切换到本方案')">
            <span class="toolbar-toggle-label">{{ tr("方案激活") }}</span>
            <Switch
              size="sm"
              :model-value="activationActive"
              :disabled="!activationEnabled || toggleDisabled"
              :label="tr('方案激活')"
              @update:model-value="emit('toggleActivation', $event)"
            />
          </span>
        </div>
        <div class="product-stage">
          <div class="product-remote">
            <img class="remote-photo" :src="remoteImage" :alt="remoteAlt" draggable="false" />
            <button
              v-for="hotspot in REMOTE_HOTSPOTS"
              :key="hotspot.id"
              type="button"
              class="remote-hotspot"
              :class="[hotspot.shape, { selected: selectedButton === hotspot.id }]"
              :style="hotspotStyle(hotspot)"
              :aria-label="trDynamic(labelFor(hotspot.id))"
              :aria-pressed="selectedButton === hotspot.id"
              :title="trDynamic(labelFor(hotspot.id))"
              @click="selectButton(hotspot.id)"
            />
          </div>
          <p class="product-note">{{ productNote }}</p>
        </div>
        <div class="remote-footer">
          <span>{{ tr("点击按键查看绑定") }}</span>
          <span class="legend">
            <span><i class="single" />{{ statusLabel("single") }}</span>
            <span><i class="double" />{{ statusLabel("double") }}</span>
            <span><i class="long" />{{ statusLabel("long") }}</span>
          </span>
        </div>
      </aside>

      <section class="binding-panel panel">
        <div class="binding-head">
          <div>
            <span class="eyebrow">{{ tr("正在编辑") }}</span>
            <h2>
              {{ trDynamic(labelFor(selectedButton)) }}
              <span class="mono">{{ selectedButton }}</span>
            </h2>
            <p>{{ tr("先选按键，再选手势，最后指定动作。当前选中键会显示金色描边。") }}</p>
          </div>
          <div class="head-stat">
            <strong
              >{{ bindingCount() }}<span> / {{ totalTriggers }}</span></strong
            >
            <small>{{ tr("已配置触发") }}</small>
            <StatusPill v-if="dirty" :label="trDynamic('未保存')" tone="warning" dot />
          </div>
        </div>

        <div class="binding-triggers">
          <button
            v-for="trigger in PROFILE_TRIGGERS"
            :key="trigger"
            type="button"
            class="trigger-tab"
            :class="{ active: selectedTrigger === trigger }"
            @click="selectTrigger(trigger)"
          >
            <i :class="trigger" />
            {{ statusLabel(trigger) }}
          </button>
        </div>

        <ProfileBindingEditor
          v-model:draft="draft"
          :button-id="selectedButton"
          :trigger="selectedTrigger"
          :busy="busy"
          :common-phrases-available="commonPhrasesAvailable"
          :latest-macros="latestMacros"
          @error="emit('error', $event)"
        />
      </section>
    </template>

    <template v-else>
      <section class="matrix-panel panel">
        <div class="matrix-toolbar">
          <label class="search-wrap">
            <Search :size="14" />
            <input v-model="query" class="search-input" :placeholder="tr('搜索按键或动作…')" />
          </label>
          <label class="filter-toggle">
            <input v-model="onlyConfigured" type="checkbox" />
            {{ tr("只看已配置") }}
          </label>
          <span class="matrix-count">{{ bindingCount() }} / {{ totalTriggers }}</span>
        </div>
        <div class="matrix-scroll">
          <table class="matrix-table">
            <thead>
              <tr>
                <th>{{ tr("按键") }}</th>
                <th v-for="trigger in PROFILE_TRIGGERS" :key="trigger">
                  <i class="tone-dot" :class="trigger" />
                  {{ statusLabel(trigger) }}
                </th>
              </tr>
            </thead>
            <tbody>
              <tr
                v-for="button in visibleButtons"
                :key="button.id"
                :class="{ selected: selectedButton === button.id }"
              >
                <td>
                  <button
                    type="button"
                    class="matrix-key"
                    :class="{ configured: hasBinding(button.id) }"
                    @click="selectButton(button.id)"
                  >
                    <span class="key-cap">{{ CAPS[button.id] }}</span>
                    <span>
                      <strong>{{ trDynamic(button.label) }}</strong>
                      <small>{{ button.id }}</small>
                    </span>
                    <span v-if="isButtonModified(button.id)" class="modified-dot" />
                  </button>
                </td>
                <td v-for="trigger in PROFILE_TRIGGERS" :key="trigger">
                  <button
                    type="button"
                    class="matrix-cell"
                    :class="{
                      selected: selectedButton === button.id && selectedTrigger === trigger,
                      empty: !actionFor(button.id, trigger),
                    }"
                    @click="selectCell(button.id, trigger)"
                  >
                    <template v-if="actionFor(button.id, trigger)">
                      <i class="tone-dot" :class="trigger" />
                      <span class="cell-copy">
                        <strong>{{ summary(button.id, trigger) }}</strong>
                      </span>
                    </template>
                    <span v-else class="cell-add">＋</span>
                  </button>
                </td>
              </tr>
              <tr v-if="visibleButtons.length === 0">
                <td colspan="4" class="empty-note">{{ tr("没有匹配的按键。") }}</td>
              </tr>
            </tbody>
          </table>
        </div>
        <div class="matrix-foot">
          <span>{{ tr("单击单元格即可在右侧修改") }}</span>
          <span>{{ bindingCount() }} / {{ totalTriggers }}</span>
        </div>
      </section>

      <aside class="inspector panel">
        <div class="inspector-head">
          <span class="eyebrow">{{ tr("正在编辑") }}</span>
          <h2>
            {{ trDynamic(labelFor(selectedButton)) }}<span class="mono">{{ selectedButton }}</span>
          </h2>
          <div class="inspector-triggers">
            <button
              v-for="trigger in PROFILE_TRIGGERS"
              :key="trigger"
              type="button"
              class="trigger-tab"
              :class="{ active: selectedTrigger === trigger }"
              @click="selectTrigger(trigger)"
            >
              {{ statusLabel(trigger) }}
            </button>
          </div>
        </div>
        <div class="inspector-body">
          <ProfileBindingEditor
            v-model:draft="draft"
            :button-id="selectedButton"
            :trigger="selectedTrigger"
            compact
            :busy="busy"
            :common-phrases-available="commonPhrasesAvailable"
            :latest-macros="latestMacros"
            @error="emit('error', $event)"
          />
        </div>
      </aside>
    </template>
  </div>
</template>

<style scoped>
.profile-workbench {
  display: grid;
  gap: var(--space-4);
  min-width: 0;
}

.profile-workbench:has(.remote-panel) {
  grid-template-columns: clamp(240px, 26%, 280px) minmax(0, 1fr);
  align-items: start;
}

.profile-workbench:has(.matrix-panel) {
  grid-template-columns: minmax(0, 1fr) clamp(280px, 28%, 330px);
  align-items: start;
}

.panel {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-sm);
}

.remote-panel {
  overflow: hidden;
}
.panel-head {
  display: flex;
  align-items: center;
  justify-content: flex-start;
  gap: var(--space-2);
  /* Match .binding-panel's padding so both columns start on the same line. */
  padding: var(--space-5) var(--space-4) var(--space-2);
}

/* 产品图上方工具栏：三个开关横向铺开，文字在上、开关在下，窄面板也放得下。 */
.panel-head.remote-toolbar {
  justify-content: space-between;
  flex-wrap: nowrap;
  min-width: 0;
  /* 组间距比组内间距大一点，三组不至于糊在一起。 */
  gap: var(--space-3);
  padding-bottom: 0;
}

/* 左右结构：标签在左、开关在右；标签与开关都用紧凑尺寸，250px 宽也放得下三组。 */
.toolbar-toggle {
  display: flex;
  flex: 1 1 0;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
  min-width: 0;
}

.toolbar-toggle-label {
  color: var(--text-muted);
  font-size: 10px;
  line-height: 1.2;
  white-space: nowrap;
}

.eyebrow {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.device-pill {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 1px solid var(--border);
  border-radius: var(--radius-pill);
  background: var(--bg-subtle);
  padding: 3px 7px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 9px;
  white-space: nowrap;
}

.device-pill::before {
  width: 4px;
  height: 4px;
  border-radius: 50%;
  background: var(--success);
  content: "";
}

/* Not connected: the dot must not read as "online". */
.device-pill.offline::before {
  background: var(--text-muted);
}

.product-stage {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-2);
  padding: var(--space-2) var(--space-4) var(--space-3);
  background:
    radial-gradient(
      circle at 50% 36%,
      color-mix(in srgb, var(--accent) 10%, transparent),
      transparent 52%
    ),
    linear-gradient(180deg, var(--bg-subtle), var(--bg));
}

.product-remote {
  position: relative;
  flex: 0 0 auto;
  width: clamp(120px, 54%, 150px);
}

.remote-photo {
  display: block;
  width: 100%;
  height: auto;
  filter: drop-shadow(0 22px 24px rgba(24, 48, 42, 0.2));
  user-select: none;
}

.remote-hotspot {
  position: absolute;
  z-index: 1;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: transparent;
  cursor: pointer;
  transform: translate(-50%, -50%);
}

.remote-hotspot.zone {
  border-radius: 999px;
}

.remote-hotspot:hover {
  background: rgba(255, 255, 255, 0.12);
  box-shadow:
    0 0 0 2px rgba(255, 255, 255, 0.7),
    0 0 0 4px rgba(15, 122, 99, 0.24);
}

.remote-hotspot:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 3px;
}

.remote-hotspot.selected::after {
  position: absolute;
  inset: -3px;
  border: 2px solid #e8b45c;
  border-radius: inherit;
  box-shadow:
    0 0 0 3px rgba(232, 180, 92, 0.24),
    0 0 16px rgba(15, 122, 99, 0.45);
  content: "";
  pointer-events: none;
}

.product-note {
  margin: 0;
  color: var(--text-muted);
  font-size: 9px;
  line-height: 1.45;
  text-align: center;
}
.remote-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: 10px var(--space-4) 12px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}
.legend {
  display: flex;
  gap: 9px;
}
.legend span {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.legend i {
  width: 4px;
  height: 4px;
  border-radius: 50%;
}
.legend i.single {
  background: var(--accent);
}
.legend i.double {
  background: var(--warning);
}
.legend i.long {
  background: var(--info);
}

.binding-panel {
  overflow: hidden;
  padding: var(--space-5);
}
.binding-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
}
.binding-head h2 {
  margin: 4px 0 0;
  font-size: var(--text-2xl);
  letter-spacing: -0.04em;
}
.binding-head h2 span {
  margin-left: 6px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  font-weight: 500;
  letter-spacing: 0;
}
.binding-head p {
  margin: 6px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.55;
}
.head-stat {
  flex: 0 0 auto;
  text-align: right;
}
.head-stat strong {
  display: block;
  font-size: var(--text-xl);
  letter-spacing: -0.03em;
}
.head-stat strong span {
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 500;
}
.head-stat small {
  display: block;
  margin: 2px 0 5px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.binding-triggers,
.inspector-triggers {
  display: flex;
  gap: 4px;
  margin: var(--space-4) 0 var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: 3px;
  width: fit-content;
}
.trigger-tab {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  border: 0;
  border-radius: var(--radius-xs);
  background: transparent;
  padding: 5px 9px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 650;
  cursor: pointer;
}
.trigger-tab:hover {
  color: var(--text);
}
.trigger-tab.active {
  background: var(--bg-raised);
  color: var(--text);
  box-shadow: 0 2px 8px rgba(20, 40, 35, 0.08);
}
.trigger-tab i {
  width: 5px;
  height: 5px;
  border-radius: 50%;
}
.trigger-tab i.single {
  background: var(--accent);
}
.trigger-tab i.double {
  background: var(--warning);
}
.trigger-tab i.long {
  background: var(--info);
}

.matrix-panel {
  overflow: hidden;
}
.matrix-toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  border-bottom: 1px solid var(--border);
  padding: var(--space-3);
}
.search-wrap {
  display: flex;
  flex: 1;
  align-items: center;
  gap: 6px;
  min-width: 100px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  padding: 0 8px;
  color: var(--text-muted);
}
.search-input {
  flex: 1;
  min-width: 0;
  border: 0;
  background: transparent;
  padding: 7px 0;
  color: var(--text);
  font-size: var(--text-xs);
  outline: 0;
}
.filter-toggle {
  display: inline-flex;
  align-items: center;
  gap: 5px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  white-space: nowrap;
}
.filter-toggle input {
  accent-color: var(--accent);
}
.matrix-count {
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
}

.matrix-scroll {
  overflow: visible;
  max-height: none;
}
.matrix-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
}
.matrix-table th {
  border-bottom: 0;
  background: var(--bg-subtle);
  padding: 9px 12px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-align: left;
  text-transform: uppercase;
}
.matrix-table th:not(:first-child) {
  text-align: center;
}
.matrix-table td {
  border-top: 1px solid var(--border);
  padding: 5px;
  vertical-align: middle;
}
.matrix-table tr.selected td {
  background: color-mix(in srgb, var(--accent-soft) 52%, transparent);
}
.matrix-table tr:hover td {
  background: var(--bg-hover);
}
.matrix-key {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  border: 0;
  background: transparent;
  padding: 6px;
  text-align: left;
  cursor: pointer;
}
.key-cap {
  display: grid;
  flex: 0 0 30px;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-raised);
  font-family: var(--font-mono);
  font-size: 10px;
}
.matrix-key.configured .key-cap {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent-ink);
  font-weight: 700;
}
.matrix-key.configured strong {
  color: var(--accent-ink);
}
.matrix-key strong {
  display: block;
  font-size: var(--text-xs);
}
.matrix-key small {
  display: block;
  margin-top: 1px;
  color: var(--text-muted);
  font-size: 8px;
}
.modified-dot {
  width: 5px;
  height: 5px;
  margin-left: auto;
  border-radius: 50%;
  background: var(--accent);
}
.matrix-cell {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 6px;
  width: 100%;
  min-height: 36px;
  border: 1px solid transparent;
  border-radius: var(--radius-xs);
  background: transparent;
  padding: 5px 7px;
  text-align: center;
  cursor: pointer;
}
.matrix-cell:hover {
  border-color: var(--border);
  background: var(--bg-raised);
}
.matrix-cell.selected {
  border-color: var(--accent);
  background: var(--accent-soft);
}
.matrix-cell.empty {
  justify-content: center;
  color: var(--text-muted);
  font-size: var(--text-lg);
}
.cell-copy {
  display: flex;
  flex-direction: column;
  align-items: center;
  min-width: 0;
  text-align: center;
}
.cell-copy strong {
  overflow: hidden;
  font-size: var(--text-2xs);
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.cell-add {
  color: var(--text-muted);
  font-size: var(--text-base);
}
.tone-dot {
  flex: 0 0 4px;
  width: 4px;
  height: 4px;
  border-radius: 50%;
}
.tone-dot.single {
  background: var(--accent);
}
.tone-dot.double {
  background: var(--warning);
}
.tone-dot.long {
  background: var(--info);
}
.matrix-foot {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: 10px 14px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}
.empty-note {
  padding: var(--space-6);
  color: var(--text-muted);
  text-align: center;
}

.inspector {
  position: sticky;
  top: var(--space-4);
  overflow: hidden;
}
.inspector-head {
  padding: var(--space-4) var(--space-4) 0;
}
.inspector-head h2 {
  margin: 4px 0 0;
  font-size: var(--text-xl);
  letter-spacing: -0.03em;
}
.inspector-head h2 span {
  margin-left: 5px;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
  font-weight: 500;
  letter-spacing: 0;
}
.inspector-head .inspector-triggers {
  margin: var(--space-3) 0 0;
}
.inspector-body {
  padding: var(--space-4);
}

/* Layout stays multi-column down to the app minimum window width (1200px);
   the content area is ~960px there once the 208px sidebar is subtracted.
   These rules only apply below the supported minimum, as a safety net. */
@media (max-width: 1100px) {
  .profile-workbench:has(.matrix-panel) {
    grid-template-columns: 1fr;
  }
  .inspector {
    position: static;
  }
}

@media (max-width: 1100px) {
  .profile-workbench:has(.remote-panel),
  .profile-workbench:has(.matrix-panel) {
    grid-template-columns: 1fr;
  }
  .inspector {
    position: static;
  }
  .remote-stage {
    min-height: 490px;
  }
  .binding-panel {
    padding: var(--space-4);
  }
  .matrix-table th,
  .matrix-table td {
    min-width: 120px;
  }
  .remote-shell {
    transform: scale(0.92);
  }
}
</style>
