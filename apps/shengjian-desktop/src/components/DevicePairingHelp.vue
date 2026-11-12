<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { CheckCircle2, CircleHelp, Mic, Radio, Tv, X } from "lucide-vue-next";
import { tr, trDynamic } from "../lib/i18n";
import { normalizePairingModel, type PairingModel } from "../stores/pairing";
import rc001Photo from "../assets/devices/xiaomi-rc001.png";
import rc003Photo from "../assets/devices/xiaomi-rc003.png";
import arn9Photo from "../assets/devices/xiaomi-arn9.png";

const props = defineProps<{
  initialModel?: string | null;
}>();

const emit = defineEmits<{
  close: [];
}>();

interface ButtonHotspot {
  label: string;
  x: number;
  y: number;
  side: "left" | "right";
}

interface ModelInfo {
  nameKey: string;
  bluetoothName: string;
  image: string;
  width: number;
  height: number;
  /**
   * 产品主体归一化系数。
   *
   * 三张产品图里，产品主体占画布高度的比例不同（RC003 95.4%、RC001 93.6%、
   * ARN9 95.6%），而这里图片高度固定 260px，于是切型号时主体大小会跳：
   * 实测 RC001 比 RC003 矮 4.71px（-1.9%），从 2 Pro 切到 2 会有明显的缩小感。
   * 以 RC003 为基准统一放大/缩小，让三款的主体显示高度一致（247.97px）。
   */
  bodyScale: number;
  /** 该型号是否有 TV 键：决定「先按 TV 完成初始化」这条提示是否显示。 */
  hasTvKey: boolean;
  hotspots: ButtonHotspot[];
}

/**
 * 三个型号的正式名称。
 *
 * `nameKey` 是产品名（型号标签/标题用），`bluetoothName` 是「在 Windows 蓝牙列表里
 * 按什么名字找」——两者都用厂商正式名，不再写英文代号：中文 Windows 的列表里显示的
 * 就是中文名，写 "Xiaomi Bluetooth Remote 2" 会让用户找不到。
 */
const MODELS: Record<PairingModel, ModelInfo> = {
  RC003: {
    nameKey: "小米蓝牙语音遥控器 2 Pro",
    bluetoothName: "小米蓝牙语音遥控器 2 Pro",
    image: rc003Photo,
    width: 335,
    height: 1081,
    bodyScale: 1,
    hasTvKey: true,
    hotspots: [
      { label: "主页键", x: 0.334, y: 0.483, side: "left" },
      { label: "菜单键", x: 0.334, y: 0.574, side: "left" },
      { label: "TV", x: 0.657, y: 0.574, side: "right" },
    ],
  },
  RC001: {
    nameKey: "小米蓝牙语音遥控器 2",
    bluetoothName: "小米蓝牙语音遥控器 2",
    image: rc001Photo,
    width: 341,
    height: 1103,
    bodyScale: 1.0194,
    hasTvKey: true,
    hotspots: [
      { label: "主页键", x: 0.336, y: 0.489, side: "left" },
      { label: "菜单键", x: 0.336, y: 0.576, side: "left" },
      { label: "TV", x: 0.652, y: 0.576, side: "right" },
    ],
  },
  // ARN9 普通款：514 x 1640 实拍图；没有 TV 键，Home / 返回 / 菜单 是同一行的三颗独立键。
  ARN9: {
    nameKey: "小米蓝牙语音遥控器",
    bluetoothName: "小米蓝牙语音遥控器",
    image: arn9Photo,
    width: 514,
    height: 1640,
    bodyScale: 0.9975,
    hasTvKey: false,
    hotspots: [
      { label: "主页键", x: 0.268, y: 0.447, side: "left" },
      { label: "菜单键", x: 0.743, y: 0.447, side: "right" },
    ],
  },
};

const activeModel = ref<PairingModel>(normalizePairingModel(props.initialModel));
const model = computed(() => MODELS[activeModel.value]);
const title = computed(() => tr("如何配对{name}", { name: trDynamic(model.value.nameKey) }));

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const closeButton = useTemplateRef<HTMLElement>("closeButton");
let previousFocus: HTMLElement | null = null;

function close() {
  emit("close");
}

function selectModel(next: PairingModel) {
  activeModel.value = next;
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
  if (event.key === "Escape") {
    event.preventDefault();
    close();
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

watch(
  () => props.initialModel,
  (value) => {
    activeModel.value = normalizePairingModel(value);
  },
);

onMounted(() => {
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  window.addEventListener("keydown", onKeydown, true);
  void nextTick(() => (closeButton.value as HTMLButtonElement | null)?.focus());
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  previousFocus?.focus();
  previousFocus = null;
});
</script>

<template>
  <Teleport to="body">
    <div class="pairing-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="pairing-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="pairing-title"
      >
        <header class="pairing-head">
          <div class="pairing-heading">
            <p class="pairing-eyebrow">
              <Radio :size="15" />
              <span>Xiaomi Remote</span>
              <span class="mono">· {{ activeModel }}</span>
            </p>
            <h2 id="pairing-title">{{ title }}</h2>
            <p class="pairing-lead">{{ tr("选择型号，按步骤完成配对。") }}</p>
          </div>
          <button
            ref="closeButton"
            type="button"
            class="pairing-close"
            :aria-label="trDynamic('关闭')"
            @click="close"
          >
            <X :size="18" />
          </button>
        </header>

        <div class="model-tabs" role="tablist" :aria-label="trDynamic('遥控器型号')">
          <button
            v-for="(info, key) in MODELS"
            :key="key"
            type="button"
            role="tab"
            class="model-tab"
            :class="{ active: activeModel === key }"
            :aria-selected="activeModel === key"
            :aria-label="trDynamic(info.nameKey)"
            @click="selectModel(key)"
          >
            <span class="model-tab-name">{{ trDynamic(info.nameKey) }}</span>
          </button>
        </div>

        <div class="pairing-body">
          <section class="pairing-visual">
            <div class="pairing-step-head">
              <span class="pairing-step-index">1</span>
              <div>
                <h3>{{ tr("让遥控器进入配对状态") }}</h3>
                <p>{{ tr("首次连接或重新配对时") }}</p>
              </div>
            </div>

            <div class="remote-stage">
              <div
                class="remote-figure"
                :style="{ aspectRatio: `${model.width} / ${model.height}` }"
              >
                <!-- 图片与热区同处一层并一起缩放：热区坐标是相对这层算的，
                     所以整体 scale 不会让圈和按键错位。 -->
                <div class="remote-inner" :style="{ transform: `scale(${model.bodyScale})` }">
                  <img :src="model.image" :alt="trDynamic(model.nameKey)" class="remote-photo" />
                  <span
                    v-for="spot in model.hotspots"
                    :key="spot.label"
                    class="button-hotspot"
                    :class="`side-${spot.side}`"
                    :style="{ left: `${spot.x * 100}%`, top: `${spot.y * 100}%` }"
                  >
                    <span class="hotspot-ring" aria-hidden="true" />
                    <span class="hotspot-label">{{ trDynamic(spot.label) }}</span>
                  </span>
                </div>
              </div>
            </div>

            <p class="visual-caption">
              {{ tr("同时按住「主页键」和「菜单键」约 3 秒，直到指示灯闪烁。") }}
            </p>
          </section>

          <section class="pairing-connect">
            <div class="pairing-step-head">
              <span class="pairing-step-index">2</span>
              <div>
                <h3>{{ tr("在电脑上完成连接") }}</h3>
                <p>{{ tr("打开 Windows 蓝牙设置") }}</p>
              </div>
            </div>

            <ol class="connect-steps">
              <li>{{ tr("打开 Windows「设置 > 蓝牙和其他设备」。") }}</li>
              <li>{{ tr("选择「添加设备 > 蓝牙」。") }}</li>
              <li>
                {{
                  tr("在列表中选择 {device}。", {
                    device: model.bluetoothName,
                  })
                }}
              </li>
            </ol>

            <!-- ARN9 没有 TV 键，这条初始化提示只对本型号不适用。 -->
            <p v-if="model.hasTvKey" class="init-note">
              <Tv :size="16" />
              <span>{{ tr("新遥控器请先按一次「TV」键完成初始化，再进入配对。") }}</span>
            </p>
          </section>
        </div>

        <div class="pairing-tips">
          <div class="pairing-tip">
            <span class="tip-icon"><Mic :size="17" /></span>
            <div>
              <strong>{{ tr("语音键") }}</strong>
              <p>{{ tr("按住说话，松开结束。") }}</p>
            </div>
          </div>
          <div class="pairing-tip">
            <span class="tip-icon"><CheckCircle2 :size="17" /></span>
            <div>
              <strong>{{ tr("确认键") }}</strong>
              <p>{{ tr("发送当前输入内容。") }}</p>
            </div>
          </div>
          <div class="pairing-tip">
            <span class="tip-icon"><CircleHelp :size="17" /></span>
            <div>
              <strong>{{ tr("配对遇到问题？") }}</strong>
              <p>{{ tr("让遥控器靠近电脑，换一节电池，或回到连接页重新扫描。") }}</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.pairing-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  overflow-y: auto;
  background: var(--overlay);
  padding: var(--space-3);
  backdrop-filter: blur(6px);
}

.pairing-dialog {
  width: min(960px, 100%);
  max-height: calc(100vh - 2 * var(--space-3));
  overflow-y: auto;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.pairing-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-5) var(--space-6) var(--space-3);
}

.pairing-eyebrow {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.pairing-heading h2 {
  margin-top: var(--space-2);
  font-size: var(--text-2xl);
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.2;
}

.pairing-lead {
  margin-top: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-sm);
  line-height: 1.5;
}

.pairing-close {
  display: grid;
  flex: 0 0 32px;
  place-items: center;
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.pairing-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.pairing-close:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.model-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  border-bottom: 1px solid var(--border);
  padding: var(--space-2) var(--space-6);
}

.model-tab {
  display: inline-flex;
  align-items: center;
  border: 1px solid transparent;
  border-radius: var(--radius-pill);
  background: transparent;
  padding: 6px 11px;
  color: var(--text-muted);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
}

.model-tab:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.model-tab.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

.model-tab-name {
  font-weight: 600;
}

.pairing-body {
  display: grid;
  grid-template-columns: minmax(0, 1.02fr) minmax(0, 1fr);
}

.pairing-visual {
  padding: var(--space-4) var(--space-6);
  border-right: 1px solid var(--border);
}

.pairing-step-head {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
}

.pairing-step-index {
  display: grid;
  flex: 0 0 28px;
  place-items: center;
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: var(--text);
  color: var(--bg);
  font-family: var(--font-mono);
  font-size: var(--text-xs);
  font-weight: 700;
}

.pairing-step-head h3 {
  font-size: var(--text-lg);
  font-weight: 650;
  line-height: 1.3;
}

.pairing-step-head p {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.remote-stage {
  position: relative;
  display: flex;
  flex-direction: column;
  align-items: center;
  min-height: 284px;
  margin-top: var(--space-4);
  border-radius: var(--radius-lg);
  background:
    radial-gradient(circle at 50% 36%, rgba(90, 200, 170, 0.13), transparent 58%), #0b0f14;
  padding: var(--space-3) var(--space-3) 0;
  overflow: hidden;
}

.remote-figure {
  position: relative;
  flex: 0 0 auto;
  height: 260px;
  margin-top: var(--space-2);
}

.remote-inner {
  position: absolute;
  inset: 0;
  transform-origin: center center;
}

.remote-photo {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: contain;
  filter: drop-shadow(0 14px 22px rgba(0, 0, 0, 0.45));
}

.button-hotspot {
  position: absolute;
  z-index: 2;
  width: 20px;
  height: 20px;
  transform: translate(-50%, -50%);
}

.hotspot-ring {
  position: absolute;
  inset: 0;
  border: 2px solid #ffcf5c;
  border-radius: 50%;
  background: rgba(255, 207, 92, 0.16);
  box-shadow:
    0 0 0 3px rgba(255, 207, 92, 0.16),
    0 0 12px rgba(255, 207, 92, 0.35);
}

.hotspot-label {
  position: absolute;
  top: 50%;
  border: 1px solid rgba(255, 207, 92, 0.65);
  border-radius: var(--radius-pill);
  background: #0b0f14;
  padding: 3px 7px;
  color: #ffe6a8;
  font-size: 11px;
  font-weight: 600;
  line-height: 1.2;
  white-space: nowrap;
  transform: translateY(-50%);
}

.hotspot-label::after {
  position: absolute;
  top: 50%;
  width: 14px;
  border-top: 1px dashed rgba(255, 207, 92, 0.8);
  content: "";
}

.side-left .hotspot-label {
  right: calc(100% + 14px);
}

.side-left .hotspot-label::after {
  left: 100%;
}

.side-right .hotspot-label {
  left: calc(100% + 14px);
}

.side-right .hotspot-label::after {
  right: 100%;
}

.visual-caption {
  margin-top: var(--space-3);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.pairing-connect {
  padding: var(--space-4) var(--space-6);
}

.connect-steps {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  margin: var(--space-4) 0 0;
  padding-left: 1.2em;
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.6;
}

.connect-steps li::marker {
  color: var(--accent);
  font-family: var(--font-mono);
  font-weight: 650;
}

.init-note {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  margin-top: var(--space-5);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-3);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.55;
}

.init-note svg {
  margin-top: 2px;
  color: var(--accent);
}

.pairing-tips {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: var(--space-4);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-6);
}

.pairing-tip {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  min-width: 0;
}

.tip-icon {
  display: grid;
  flex: 0 0 32px;
  place-items: center;
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  background: var(--accent-soft);
  color: var(--accent);
}

.pairing-tip strong {
  display: block;
  font-size: var(--text-sm);
  font-weight: 650;
}

.pairing-tip p {
  margin-top: 2px;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

/* Layout stays multi-column down to the app minimum window width (1200px);
   the content area is ~960px there once the 208px sidebar is subtracted.
   These rules only apply below the supported minimum, as a safety net. */
@media (max-width: 1100px) {
  .pairing-body {
    grid-template-columns: 1fr;
  }

  .pairing-visual {
    border-right: 0;
    border-bottom: 1px solid var(--border);
  }

  .pairing-tips {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 1100px) {
  .pairing-backdrop {
    padding: var(--space-2);
  }

  .pairing-head,
  .model-tabs,
  .pairing-visual,
  .pairing-connect,
  .pairing-tips {
    padding-inline: var(--space-4);
  }
}
</style>
