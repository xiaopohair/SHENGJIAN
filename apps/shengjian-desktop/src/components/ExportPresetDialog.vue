<script setup lang="ts">
import { computed, nextTick, ref, useTemplateRef, watch } from "vue";
import { FileDown, X } from "lucide-vue-next";
import { tr } from "../lib/i18n";
import Button from "./ui/Button.vue";
import Field from "./ui/Field.vue";
import TextInput from "./ui/TextInput.vue";

export interface PresetExportRequest {
  presetId: string;
  compatibleModels: string[];
}

const props = defineProps<{
  open: boolean;
  /** 当前方案名，用来预填推荐方案 id 与提示文案。 */
  profileName: string;
  /** 当前设备型号；用于预填 id 前缀与适配型号。 */
  deviceModel: string | null;
  busy?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  submit: [request: PresetExportRequest];
}>();

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const presetId = ref("");
const models = ref("");
let previousFocus: HTMLElement | null = null;

/**
 * 方案名 → id 片段：保留中英文与数字，其余折叠成单个连字符。
 *
 * 保留中文是为了默认值可读且不撞车（去掉中文会让所有中文名都退化成 `custom`，
 * 第二次导出就重名）。用户可以改成英文/拼音。
 */
function slugify(value: string): string {
  const slug = value
    .trim()
    .toLowerCase()
    .replace(/[\s_/\\]+/g, "-")
    .replace(/[^a-z0-9\u4e00-\u9fa5-]/g, "")
    .replace(/-{2,}/g, "-")
    .replace(/^-|-$/g, "");
  return slug || "custom";
}

function defaultPresetId(): string {
  const model = (props.deviceModel ?? "").trim().toLowerCase();
  return `preset.${model || "generic"}.${slugify(props.profileName)}`;
}

const trimmedId = computed(() => presetId.value.trim());
const idError = computed(() => {
  if (!trimmedId.value.startsWith("preset.")) return tr("id 必须以 preset. 开头");
  if (trimmedId.value.length === "preset.".length) return tr("id 不能只有前缀");
  if (/\s/.test(presetId.value)) return tr("id 不能包含空格");
  return "";
});
const modelList = computed(() =>
  models.value
    .split(/[,，\s]+/)
    .map((model) => model.trim().toUpperCase())
    .filter(Boolean),
);
const canSubmit = computed(() => !idError.value && !props.busy);

watch(
  () => props.open,
  async (openState) => {
    if (openState) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      presetId.value = defaultPresetId();
      models.value = props.deviceModel ?? "";
      await nextTick();
      dialogRef.value?.querySelector<HTMLInputElement>(".preset-id-input")?.focus();
    } else {
      previousFocus?.focus();
      previousFocus = null;
    }
  },
);

function close() {
  emit("close");
}

function submit() {
  if (!canSubmit.value) return;
  emit("submit", { presetId: trimmedId.value, compatibleModels: modelList.value });
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
  }
  if (event.key === "Enter" && event.target instanceof HTMLInputElement) {
    event.preventDefault();
    submit();
  }
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="preset-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="preset-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="preset-export-title"
        @keydown="onKeydown"
      >
        <header class="preset-head">
          <div>
            <h2 id="preset-export-title">
              <FileDown :size="17" aria-hidden="true" />
              {{ tr("导出为推荐方案") }}
            </h2>
          </div>
          <button type="button" class="preset-close" :aria-label="tr('关闭')" @click="close">
            <X :size="16" />
          </button>
        </header>

        <div class="preset-body">
          <Field label="推荐方案 id">
            <TextInput v-model="presetId" class="preset-id-input" :maxlength="120" />
            <span v-if="idError" class="preset-error">{{ idError }}</span>
          </Field>

          <Field label="适配型号">
            <TextInput v-model="models" :placeholder="tr('留空表示对所有型号可见')" />
            <span class="preset-help">
              {{ tr("多个型号用逗号分隔，例如 RC003,ARN9。") }}
            </span>
          </Field>
        </div>

        <footer class="preset-foot">
          <span class="preset-model-hint">
            {{
              props.deviceModel
                ? tr("当前型号：{model}", { model: props.deviceModel })
                : tr("当前未连接设备")
            }}
          </span>
          <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
          <Button size="sm" variant="primary" :disabled="!canSubmit" @click="submit">
            {{ tr("选择位置并导出") }}
          </Button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.preset-backdrop {
  position: fixed;
  inset: 0;
  z-index: 260;
  display: grid;
  place-items: center;
  padding: var(--space-4);
  background: var(--overlay);
  backdrop-filter: blur(4px);
}

.preset-dialog {
  display: flex;
  flex-direction: column;
  width: min(460px, calc(100vw - 32px));
  max-height: calc(100vh - 48px);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.preset-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-5);
}

.preset-head h2 {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  margin: 0;
  font-size: var(--text-lg);
  letter-spacing: -0.02em;
}

.preset-close {
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

.preset-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.preset-body {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  overflow: auto;
  padding: var(--space-5);
}

/* 对话框只有 ~460px 宽：字段标签压到输入框上方，避免 140px 标签列把输入框挤窄。 */
.preset-body :deep(.ui-field) {
  grid-template-columns: minmax(0, 1fr);
  align-items: stretch;
  gap: 5px;
}

.preset-body :deep(.ui-field-control) {
  width: 100%;
}

.preset-help {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  overflow-wrap: anywhere;
}

.preset-error {
  color: var(--danger);
  font-size: var(--text-2xs);
}

.preset-foot {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.preset-model-hint {
  margin-right: auto;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}
</style>
