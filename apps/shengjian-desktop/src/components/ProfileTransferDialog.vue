<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { FileDown, FileUp, FolderOpen, X } from "lucide-vue-next";
import { tr, type TranslationKey } from "../lib/i18n";
import type { ButtonProfile } from "../lib/types";
import Button from "./ui/Button.vue";
import Checkbox from "./ui/Checkbox.vue";
import IconButton from "./ui/IconButton.vue";
import ToggleGroup, { type ToggleGroupItem } from "./ui/ToggleGroup.vue";

/** 导出形态：单个方案库文件（可再次导入），或每个方案一个推荐方案文件。 */
export type ExportMode = "document" | "preset";

const props = defineProps<{
  open: boolean;
  profiles: ButtonProfile[];
  busy?: boolean;
}>();

const emit = defineEmits<{
  close: [];
  import: [];
  exportDocument: [ids: string[]];
  exportPreset: [ids: string[]];
}>();

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const mode = ref<"export" | "import">("export");
const exportMode = ref<ExportMode>("document");
const selected = ref<Set<string>>(new Set());
let previousFocus: HTMLElement | null = null;

const MODE_ITEMS: ToggleGroupItem[] = [
  { value: "export", label: "导出" },
  { value: "import", label: "导入" },
];

const FORMAT_ITEMS: Array<{
  value: ExportMode;
  title: TranslationKey;
  detail: TranslationKey;
}> = [
  {
    value: "document",
    title: "方案文件",
    detail: "导出为一个文件，之后可以再导入回来。",
  },
  {
    value: "preset",
    title: "推荐方案",
    detail: "每个方案单独一个文件，用于内置推荐方案。",
  },
];

/** 默认方案是该型号的基线，不能当作推荐方案下发。 */
function isPresetEligible(profile: ButtonProfile): boolean {
  return !profile.isDefault;
}

const selectedCount = computed(() => selected.value.size);
const allSelected = computed(
  () => props.profiles.length > 0 && selected.value.size === props.profiles.length,
);
const canExport = computed(() => selectedCount.value > 0 && !props.busy);
const primaryLabel = computed(() => {
  if (mode.value === "import") return tr("选择文件并导入");
  return exportMode.value === "preset" ? tr("选择文件夹并导出") : tr("选择位置并导出");
});
const primaryDisabled = computed(() =>
  mode.value === "import" ? Boolean(props.busy) : !canExport.value,
);

watch(
  () => props.open,
  async (openState) => {
    if (openState) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      mode.value = "export";
      exportMode.value = "document";
      // 默认全选：常见用法是「把当前这套都导出去备份」。
      selected.value = new Set(props.profiles.map((profile) => profile.id));
      // Esc 走 window 捕获：弹窗打开时焦点可能在触发按钮上，只挂 section 会漏。
      window.addEventListener("keydown", onWindowKeydown, true);
      await nextTick();
      dialogRef.value?.querySelector<HTMLButtonElement>(".transfer-tab.active")?.focus();
    } else {
      window.removeEventListener("keydown", onWindowKeydown, true);
      previousFocus?.focus();
      previousFocus = null;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  window.removeEventListener("keydown", onWindowKeydown, true);
});

// 切到「推荐方案」时自动剔除默认方案，避免用户点了导出才发现导不出。
watch(exportMode, (value) => {
  if (value !== "preset") return;
  const next = new Set(selected.value);
  for (const id of next) {
    if (!props.profiles.some((profile) => profile.id === id && isPresetEligible(profile))) {
      next.delete(id);
    }
  }
  selected.value = next;
});

function toggle(id: string) {
  const next = new Set(selected.value);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  selected.value = next;
}

function toggleAll() {
  selected.value = allSelected.value
    ? new Set()
    : new Set(props.profiles.map((profile) => profile.id));
}

const orderedSelection = computed(() =>
  props.profiles.filter((profile) => selected.value.has(profile.id)).map((profile) => profile.id),
);

function close() {
  emit("close");
}

function submit() {
  if (primaryDisabled.value) return;
  if (mode.value === "import") {
    emit("import");
    return;
  }
  if (exportMode.value === "preset") emit("exportPreset", orderedSelection.value);
  else emit("exportDocument", orderedSelection.value);
}

function onWindowKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  event.preventDefault();
  event.stopPropagation();
  close();
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="transfer-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="transfer-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="transfer-title"
      >
        <header class="transfer-head">
          <div class="transfer-heading">
            <h2 id="transfer-title">{{ tr("导入与导出") }}</h2>
            <p>{{ tr("备份、迁移当前按键方案，或把方案整理成内置推荐方案。") }}</p>
          </div>
          <IconButton size="sm" :label="tr('关闭')" @click="close">
            <X :size="16" />
          </IconButton>
        </header>

        <div class="transfer-switch">
          <ToggleGroup
            v-model="mode"
            :items="MODE_ITEMS"
            aria-label="导入或导出"
            class="transfer-tab-group"
          />
        </div>

        <div class="transfer-body">
          <template v-if="mode === 'export'">
            <section class="transfer-section">
              <p class="transfer-group-label">{{ tr("导出格式") }}</p>
              <div class="transfer-formats" role="radiogroup" :aria-label="tr('导出格式')">
                <button
                  v-for="format in FORMAT_ITEMS"
                  :key="format.value"
                  type="button"
                  class="transfer-format"
                  :class="{ active: exportMode === format.value }"
                  role="radio"
                  :aria-checked="exportMode === format.value"
                  @click="exportMode = format.value"
                >
                  <FileDown :size="16" aria-hidden="true" />
                  <span class="transfer-format-copy">
                    <strong>{{ tr(format.title) }}</strong>
                    <span>{{ tr(format.detail) }}</span>
                  </span>
                </button>
              </div>
            </section>

            <section class="transfer-section transfer-section-grow">
              <div class="transfer-list-head">
                <p class="transfer-group-label">{{ tr("选择方案") }}</p>
                <div class="transfer-list-actions">
                  <span class="transfer-count">
                    {{
                      tr("已选 {picked}/{total}", { picked: selectedCount, total: profiles.length })
                    }}
                  </span>
                  <Button size="sm" variant="ghost" @click="toggleAll">
                    {{ allSelected ? tr("全不选") : tr("全选") }}
                  </Button>
                </div>
              </div>

              <ul v-if="profiles.length" class="transfer-list">
                <li v-for="profile in profiles" :key="profile.id" class="transfer-item">
                  <Checkbox
                    :model-value="selected.has(profile.id)"
                    :disabled="exportMode === 'preset' && !isPresetEligible(profile)"
                    @update:model-value="toggle(profile.id)"
                  >
                    <span class="transfer-item-copy">
                      <span class="transfer-name">{{ profile.name }}</span>
                      <span class="transfer-model">{{ profile.deviceModel ?? tr("通用") }}</span>
                      <span
                        v-if="exportMode === 'preset' && !isPresetEligible(profile)"
                        class="transfer-note"
                      >
                        {{ tr("默认方案不可作为推荐方案") }}
                      </span>
                    </span>
                  </Checkbox>
                </li>
              </ul>
              <p v-else class="transfer-empty">{{ tr("还没有可导出的方案") }}</p>
            </section>
          </template>

          <section v-else class="transfer-import">
            <span class="transfer-import-icon" aria-hidden="true"><FileUp :size="20" /></span>
            <div>
              <strong>{{ tr("从文件导入方案") }}</strong>
              <p>{{ tr("选择之前导出的方案文件，导入会覆盖当前方案列表。") }}</p>
            </div>
          </section>
        </div>

        <footer class="transfer-foot">
          <span v-if="mode === 'export'" class="transfer-foot-summary">
            <FolderOpen :size="14" aria-hidden="true" />
            {{ tr("已选 {count} 个方案", { count: selectedCount }) }}
          </span>
          <span v-else class="transfer-foot-summary" />
          <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
          <Button size="sm" variant="primary" :disabled="primaryDisabled" @click="submit">
            {{ primaryLabel }}
          </Button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.transfer-backdrop {
  position: fixed;
  inset: 0;
  z-index: 260;
  display: grid;
  place-items: center;
  padding: var(--space-4);
  background: var(--overlay);
  backdrop-filter: blur(4px);
}

.transfer-dialog {
  display: flex;
  flex-direction: column;
  width: min(560px, calc(100vw - 32px));
  max-height: min(680px, calc(100vh - 48px));
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.transfer-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-5) var(--space-5) var(--space-3);
}

.transfer-heading h2 {
  margin: 0;
  font-size: var(--text-lg);
  letter-spacing: -0.02em;
}

.transfer-heading p {
  margin: 5px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

/* 模式切换单独一行、与主体同内边距，避免和下面两组区块挤在一起。 */
.transfer-switch {
  padding: 0 var(--space-5) var(--space-4);
}

.transfer-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-5);
  overflow: auto;
  padding: var(--space-4) var(--space-5) var(--space-5);
}

.transfer-section {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}

.transfer-section-grow {
  min-height: 0;
}

.transfer-group-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

/* 导出格式：两张可选卡片，比「标签 + 开关」更像一组并列选项。 */
.transfer-formats {
  display: grid;
  gap: var(--space-2);
  grid-template-columns: 1fr 1fr;
}

.transfer-format {
  display: flex;
  align-items: flex-start;
  gap: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: transparent;
  padding: var(--space-3);
  color: var(--text-muted);
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.transfer-format:hover {
  border-color: var(--border-strong);
  background: var(--bg-hover);
}

.transfer-format.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent);
}

.transfer-format-copy {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}

.transfer-format-copy strong {
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 600;
}

.transfer-format.active .transfer-format-copy strong {
  color: var(--accent);
}

.transfer-format-copy span {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.45;
}

.transfer-list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.transfer-list-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.transfer-count {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

/* 方案列表：一张带边框的容器，行间用发丝线分隔，滚动发生在容器内部。 */
.transfer-list {
  display: flex;
  flex-direction: column;
  min-height: 0;
  max-height: 260px;
  margin: 0;
  padding: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  overflow-y: auto;
  list-style: none;
}

.transfer-item {
  padding: var(--space-2) var(--space-3);
}

.transfer-item + .transfer-item {
  border-top: 1px solid var(--border);
}

.transfer-item:hover {
  background: var(--bg-hover);
}

/* 名称、型号徽标与禁用原因排成一行，行高压到单行。 */
.transfer-item-copy {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.transfer-name {
  color: var(--text);
  font-size: var(--text-sm);
}

.transfer-model {
  border-radius: var(--radius-pill);
  background: var(--bg-subtle);
  padding: 1px var(--space-2);
  color: var(--text-secondary);
  font-size: var(--text-2xs);
}

.transfer-note {
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.transfer-empty {
  margin: 0;
  border: 1px dashed var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-5);
  color: var(--text-muted);
  font-size: var(--text-xs);
  text-align: center;
}

.transfer-import {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-4);
}

.transfer-import-icon {
  display: grid;
  flex: 0 0 34px;
  place-items: center;
  width: 34px;
  height: 34px;
  border-radius: var(--radius-sm);
  background: var(--bg-raised);
  color: var(--text-secondary);
}

.transfer-import strong {
  color: var(--text);
  font-size: var(--text-sm);
}

.transfer-import p {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.transfer-foot {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.transfer-foot-summary {
  display: inline-flex;
  align-items: center;
  gap: var(--space-2);
  margin-right: auto;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}
</style>
