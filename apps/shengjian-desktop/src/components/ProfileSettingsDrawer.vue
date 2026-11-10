<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { FolderOpen, X } from "lucide-vue-next";
import { errorMessage, resolveAppTarget } from "../lib/api";
import type { ProfileDraft } from "../lib/buttons";
import { tr, trDynamic } from "../lib/i18n";
import Button from "./ui/Button.vue";
import Checkbox from "./ui/Checkbox.vue";
import Field from "./ui/Field.vue";
import SelectInput from "./ui/SelectInput.vue";
import TextInput from "./ui/TextInput.vue";

const props = defineProps<{
  open: boolean;
  busy: boolean;
  foregroundApp: string;
}>();

const emit = defineEmits<{
  close: [];
  error: [message: string];
}>();

const draft = defineModel<ProfileDraft>("draft", { required: true });
const drawerRef = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;

const RUNNABLE_EXTENSIONS = [
  "exe",
  "lnk",
  "bat",
  "cmd",
  "com",
  "scr",
  "msi",
  "ps1",
  "vbs",
  "js",
  "url",
];

function close() {
  emit("close");
}

async function chooseAppTarget() {
  try {
    const selected = await openDialog({
      title: tr("选择程序"),
      multiple: false,
      filters: [{ name: tr("可运行文件"), extensions: RUNNABLE_EXTENSIONS }],
    });
    if (!selected || Array.isArray(selected)) return;
    const resolved = await resolveAppTarget(selected).catch(() => null);
    const name = resolved || selected.split(/[\\/]/).pop() || selected;
    const targets = draft.value.appTargets
      .split(",")
      .map((target) => target.trim())
      .filter(Boolean);
    if (!targets.some((target) => target.toLowerCase() === name.toLowerCase())) {
      targets.push(name);
    }
    draft.value.appTargets = targets.join(", ");
  } catch (cause) {
    emit("error", errorMessage(cause));
  }
}

function useForegroundTarget() {
  if (!props.foregroundApp) return;
  const targets = draft.value.appTargets
    .split(",")
    .map((target) => target.trim())
    .filter(Boolean);
  if (!targets.includes(props.foregroundApp)) targets.push(props.foregroundApp);
  draft.value.appTargets = targets.join(", ");
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
      drawerRef.value
        ?.querySelector<HTMLElement>(".settings-body input, .settings-body select")
        ?.focus();
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
    <div v-if="props.open" class="settings-backdrop" @click.self="close">
      <aside
        ref="drawerRef"
        class="settings-drawer"
        role="dialog"
        aria-modal="true"
        aria-label="方案设置"
      >
        <header class="settings-head">
          <div>
            <span class="eyebrow">{{ tr("方案设置") }}</span>
            <h2>{{ tr("应用目标与行为") }}</h2>
            <p>{{ tr("这些设置随方案一起保存。") }}</p>
          </div>
          <button
            type="button"
            class="settings-close"
            :aria-label="trDynamic('关闭')"
            @click="close"
          >
            <X :size="18" />
          </button>
        </header>

        <div class="settings-body">
          <Field class="settings-field" label="名称">
            <TextInput v-model="draft.name" :maxlength="80" />
          </Field>
          <Field class="settings-field" label="应用目标">
            <TextInput v-model="draft.appTargets" :placeholder="trDynamic('选择或填写应用')" />
          </Field>

          <div class="settings-action-row">
            <Checkbox v-model="draft.autoApply">{{ tr("前台匹配时自动应用") }}</Checkbox>
            <div class="settings-buttons">
              <Button size="sm" :disabled="busy" @click="chooseAppTarget">
                <FolderOpen :size="14" />
                {{ tr("选择程序…") }}
              </Button>
              <Button size="sm" :disabled="!foregroundApp" @click="useForegroundTarget">
                {{ tr("使用当前程序") }}
              </Button>
            </div>
          </div>

          <Field class="settings-field" label="语音输入工具">
            <SelectInput v-model="draft.voiceInputTool">
              <option value="">{{ tr("使用设备覆盖设置或全局设置") }}</option>
              <option value="wechat">{{ tr("微信输入法") }}</option>
              <option value="doubao">{{ tr("豆包输入法") }}</option>
              <option value="vokie">{{ tr("Vokie 桌面版") }}</option>
              <option value="other">{{ tr("其他工具") }}</option>
            </SelectInput>
            <span class="settings-help">
              {{ tr("设备有单独覆盖设置时，以设备设置为准。") }}
            </span>
          </Field>
        </div>

        <footer class="settings-foot">
          <Button variant="primary" @click="close">{{ tr("完成") }}</Button>
        </footer>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.settings-backdrop {
  position: fixed;
  inset: 0;
  z-index: 240;
  display: flex;
  justify-content: flex-end;
  background: var(--overlay);
  backdrop-filter: blur(4px);
}

.settings-drawer {
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

.settings-head {
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

.settings-head h2 {
  margin: 4px 0 0;
  font-size: var(--text-xl);
  letter-spacing: -0.03em;
}

.settings-head p {
  margin: 5px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.settings-close {
  display: grid;
  flex: 0 0 30px;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.settings-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.settings-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
  padding: var(--space-5);
}

.settings-body :deep(.ui-field) {
  grid-template-columns: minmax(0, 1fr);
  align-items: stretch;
  gap: 5px;
}

.settings-body :deep(.ui-field-control) {
  width: 100%;
}

.settings-action-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
}

.settings-buttons {
  display: flex;
  flex-wrap: wrap;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-left: auto;
}

.settings-help {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.45;
}

.settings-foot {
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-5);
}
</style>
