<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { X } from "lucide-vue-next";
import { tr } from "../lib/i18n";
import { voiceToolOptions } from "../lib/voiceTools";
import type { AudioEndpoint, VoiceToolInfo } from "../lib/types";
import Button from "./ui/Button.vue";
import Checkbox from "./ui/Checkbox.vue";
import RangeInput from "./ui/RangeInput.vue";
import SelectInput from "./ui/SelectInput.vue";
import TextInput from "./ui/TextInput.vue";

/**
 * 单台遥控器的设备设置抽屉（连接页「设置」打开）。
 *
 * 原来这些字段平铺在页面上的「当前设备覆盖」卡片里，和连接状态等内容混在一起；
 * 收进抽屉后页面只留设备概览，设置按需展开。
 */
const props = defineProps<{
  open: boolean;
  /** 广播名，作为名称输入框的占位提示。 */
  fallbackName: string;
  endpoints: AudioEndpoint[];
  voiceTools: VoiceToolInfo[];
  /** 当前设备是否支持独立音频设置（不支持时隐藏音频相关字段）。 */
  canOverrideAudio: boolean;
}>();

const emit = defineEmits<{ close: [] }>();

const nickname = defineModel<string>("nickname", { required: true });
const useOverride = defineModel<boolean>("useOverride", { required: true });
const gain = defineModel<number>("gain", { required: true });
const endpointId = defineModel<string>("endpointId", { required: true });
const voiceInputTool = defineModel<string>("voiceInputTool", { required: true });

const drawerRef = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;

function close() {
  emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    close();
  }
}

watch(
  () => props.open,
  async (openState) => {
    if (openState) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      await nextTick();
      drawerRef.value?.querySelector<HTMLElement>(".device-drawer-body input")?.focus();
    } else {
      previousFocus?.focus();
      previousFocus = null;
    }
  },
);

onMounted(() => window.addEventListener("keydown", onKeydown, true));
onUnmounted(() => window.removeEventListener("keydown", onKeydown, true));
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="device-drawer-backdrop" @click.self="close">
      <aside
        ref="drawerRef"
        class="device-drawer"
        role="dialog"
        aria-modal="true"
        :aria-label="tr('设备设置')"
      >
        <header class="device-drawer-head">
          <div>
            <span class="eyebrow">{{ tr("设置") }}</span>
            <h2>{{ tr("设备设置") }}</h2>
            <p>{{ tr("只对这台遥控器生效；音频设置不勾选时沿用全局。") }}</p>
          </div>
          <button type="button" class="drawer-close" :aria-label="tr('关闭')" @click="close">
            <X :size="16" />
          </button>
        </header>

        <div class="device-drawer-body">
          <label class="drawer-field">
            <span class="drawer-field-label">{{ tr("设备名称") }}</span>
            <TextInput
              v-model="nickname"
              :maxlength="24"
              :placeholder="tr('留空则用遥控器的广播名')"
            />
          </label>

          <template v-if="canOverrideAudio">
            <Checkbox v-model="useOverride">
              {{ tr("为此设备使用独立音频设置") }}
            </Checkbox>

            <template v-if="useOverride">
              <div class="drawer-field">
                <span class="drawer-field-label">{{ tr("设备增益") }}</span>
                <div class="gain-row">
                  <RangeInput v-model="gain" :min="0" :max="24" :step="1" />
                  <span class="gain-value mono muted">{{ gain }} dB</span>
                </div>
              </div>
              <label class="drawer-field">
                <span class="drawer-field-label">{{ tr("语音输出设备") }}</span>
                <SelectInput v-model="endpointId">
                  <option value="">{{ tr("继承全局端点") }}</option>
                  <option v-for="endpoint in endpoints" :key="endpoint.id" :value="endpoint.id">
                    {{ endpoint.name }}{{ endpoint.recommended ? tr("（推荐）") : "" }}
                  </option>
                </SelectInput>
              </label>
              <label class="drawer-field">
                <span class="drawer-field-label">{{ tr("语音输入工具") }}</span>
                <SelectInput v-model="voiceInputTool">
                  <option value="">{{ tr("继承全局工具") }}</option>
                  <option
                    v-for="tool in voiceToolOptions(voiceTools, voiceInputTool)"
                    :key="tool.id"
                    :value="tool.id"
                  >
                    {{ tool.label }}
                  </option>
                </SelectInput>
              </label>
            </template>
          </template>
        </div>

        <footer class="device-drawer-foot">
          <Button size="sm" variant="primary" @click="close">{{ tr("完成") }}</Button>
        </footer>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.device-drawer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 240;
  display: flex;
  justify-content: flex-end;
  background: var(--overlay);
  backdrop-filter: blur(4px);
}

.device-drawer {
  display: flex;
  flex-direction: column;
  width: min(440px, calc(100vw - 24px));
  height: 100%;
  border-left: 1px solid var(--border-strong);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  animation: drawer-in var(--duration-base) var(--ease-out);
}

@keyframes drawer-in {
  from {
    transform: translateX(14px);
    opacity: 0.6;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.device-drawer-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-5);
}

.eyebrow {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.device-drawer-head h2 {
  margin: 4px 0 0;
  font-size: var(--text-xl);
  letter-spacing: -0.03em;
}

.device-drawer-head p {
  margin: 5px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.drawer-close {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.drawer-close:hover {
  border-color: var(--border);
  color: var(--text);
}

.device-drawer-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
  padding: var(--space-5);
}

.drawer-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.drawer-field-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.4;
}

.gain-row {
  display: flex;
  align-items: center;
  gap: var(--space-3);
}

.gain-value {
  flex: 0 0 auto;
  font-size: var(--text-xs);
}

.device-drawer-foot {
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}
</style>
