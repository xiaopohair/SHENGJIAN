<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, useTemplateRef, watch } from "vue";
import { X } from "lucide-vue-next";
import { mergeAppTargetsText, splitAppTargets } from "../lib/appTargets";
import {
  appTargetFriendlyName,
  cachedAppTarget,
  describeAppTargetsCached,
} from "../composables/useAppPicker";
import type { ProfileDraft } from "../lib/buttons";
import { tr, trDynamic } from "../lib/i18n";
import { voiceToolOptions } from "../lib/voiceTools";
import { useAppPicker } from "../composables/useAppPicker";
import { useVoiceToolCatalog } from "../composables/useVoiceToolCatalog";
import AppPickerDialog from "./app-picker/AppPickerDialog.vue";
import AppTargetField from "./app-picker/AppTargetField.vue";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";
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
const drawerRef = useTemplateRef<HTMLElement>("drawerRef");
const {
  open: appPickerOpen,
  openPicker: openAppPicker,
  closePicker: closeAppPicker,
} = useAppPicker();
const recentMatchTargets = computed(() => splitAppTargets(draft.value.appTargets));

/** 方案可能匹配多个程序：只读展示友好名，选择/更换一律走选择器。 */
const programTargets = computed(() => splitAppTargets(draft.value.appTargets));
const programLabel = computed(() =>
  programTargets.value.map((target) => appTargetFriendlyName(target, target)).join("、"),
);

// 先把已知目标丢进描述缓存，友好名才能在下一帧变成产品名。
watch(
  programTargets,
  (targets) => {
    const missing = targets.filter((target) => !cachedAppTarget(target));
    if (missing.length > 0) void describeAppTargetsCached(missing);
  },
  { immediate: true },
);
const { catalog: voiceToolCatalog, load: loadVoiceToolCatalog } = useVoiceToolCatalog();
let previousFocus: HTMLElement | null = null;

function close() {
  emit("close");
}

function applyPickedTarget(target: string) {
  draft.value.appTargets = mergeAppTargetsText(draft.value.appTargets, target);
  closeAppPicker();
}

/** 清掉程序选择（连带关掉自动应用——没有程序就无从匹配）。 */
function clearProgram() {
  draft.value.appTargets = "";
  draft.value.autoApply = false;
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  // The centered app picker sits above the drawer and owns Escape while open.
  if (appPickerOpen.value) return;
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

onMounted(() => {
  window.addEventListener("keydown", onKeydown, true);
  void loadVoiceToolCatalog();
});
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
            <h2>{{ tr("程序与行为") }}</h2>
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
          <Field class="settings-field" label="程序">
            <div class="settings-program-row">
              <AppTargetField
                :target="programTargets[0] ?? ''"
                :choose-label="tr('选择程序')"
                :disabled="busy"
                @choose="openAppPicker"
              />
              <IconButton
                v-if="programTargets.length > 0"
                size="sm"
                :label="tr('清除程序')"
                :disabled="busy"
                @click="clearProgram"
              >
                <X :size="14" />
              </IconButton>
            </div>
            <span v-if="programTargets.length > 1" class="settings-help">
              {{ tr("另外还匹配：{names}", { names: programLabel }) }}
            </span>
          </Field>

          <div class="settings-action-row">
            <Checkbox v-model="draft.autoApply" :disabled="programTargets.length === 0">
              {{ tr("前台匹配时自动应用") }}
            </Checkbox>
          </div>

          <Field class="settings-field" label="语音输入工具">
            <SelectInput v-model="draft.voiceInputTool">
              <option value="">{{ tr("使用设备覆盖设置或全局设置") }}</option>
              <option
                v-for="tool in voiceToolOptions(voiceToolCatalog, draft.voiceInputTool)"
                :key="tool.id"
                :value="tool.id"
              >
                {{ tool.label }}
              </option>
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

    <AppPickerDialog
      :open="appPickerOpen"
      mode="match"
      :title="tr('选择这个方案要匹配的应用')"
      :description="tr('选择后方案的键位会在它切到前台时生效。')"
      :foreground-app="foregroundApp"
      :recent-targets="recentMatchTargets"
      @select="applyPickedTarget"
      @close="closeAppPicker"
    />
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

.settings-program-row {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.settings-program-row :deep(.app-target-field) {
  flex: 1 1 auto;
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
