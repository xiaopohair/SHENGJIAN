<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, useTemplateRef, watch } from "vue";
import { FileUp, X } from "lucide-vue-next";
import { tr } from "../lib/i18n";
import type { ButtonProfile } from "../lib/types";
import Button from "./ui/Button.vue";
import Checkbox from "./ui/Checkbox.vue";
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
const EXPORT_MODE_ITEMS: ToggleGroupItem[] = [
  { value: "document", label: "方案文件" },
  { value: "preset", label: "推荐方案" },
];

/** 默认方案是该型号的基线，不能当作推荐方案下发。 */
function isPresetEligible(profile: ButtonProfile): boolean {
  return !profile.isDefault;
}

const eligibleIds = computed(() =>
  props.profiles.filter((profile) => isPresetEligible(profile)).map((profile) => profile.id),
);
const selectedCount = computed(() => selected.value.size);
const allSelected = computed(
  () => props.profiles.length > 0 && selected.value.size === props.profiles.length,
);
const canExport = computed(() => selectedCount.value > 0 && !props.busy);

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

function onWindowKeydown(event: KeyboardEvent) {
  if (event.key !== "Escape") return;
  event.preventDefault();
  event.stopPropagation();
  close();
}

// 切到「推荐方案」时自动剔除默认方案，避免用户点了导出才发现导不出。
watch(exportMode, (value) => {
  if (value !== "preset") return;
  const next = new Set(selected.value);
  for (const id of next) {
    if (!eligibleIds.value.includes(id)) next.delete(id);
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
  if (!canExport.value) return;
  if (exportMode.value === "preset") emit("exportPreset", orderedSelection.value);
  else emit("exportDocument", orderedSelection.value);
}

function startImport() {
  if (props.busy) return;
  emit("import");
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
  }
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
        @keydown="onKeydown"
      >
        <header class="transfer-head">
          <h2 id="transfer-title">{{ tr("导入与导出") }}</h2>
          <button type="button" class="transfer-close" :aria-label="tr('关闭')" @click="close">
            <X :size="16" />
          </button>
        </header>

        <div class="transfer-tabs">
          <ToggleGroup
            v-model="mode"
            :items="MODE_ITEMS"
            aria-label="导入或导出"
            class="transfer-tab-group"
          />
        </div>

        <div class="transfer-body">
          <template v-if="mode === 'export'">
            <div class="transfer-row">
              <span class="transfer-label">{{ tr("导出格式") }}</span>
              <ToggleGroup v-model="exportMode" :items="EXPORT_MODE_ITEMS" aria-label="导出格式" />
            </div>
            <p class="transfer-hint">
              {{
                exportMode === "document"
                  ? tr("导出为一个文件，之后可以再导入回来。")
                  : tr("每个方案单独一个文件，用于内置推荐方案。")
              }}
            </p>

            <div class="transfer-list-head">
              <span class="transfer-label">
                {{ tr("选择方案") }}（{{ selectedCount }}/{{ profiles.length }}）
              </span>
              <Button size="sm" variant="ghost" @click="toggleAll">
                {{ allSelected ? tr("全不选") : tr("全选") }}
              </Button>
            </div>

            <ul v-if="profiles.length" class="transfer-list">
              <li v-for="profile in profiles" :key="profile.id" class="transfer-item">
                <Checkbox
                  :model-value="selected.has(profile.id)"
                  :disabled="exportMode === 'preset' && !isPresetEligible(profile)"
                  @update:model-value="toggle(profile.id)"
                >
                  <span class="transfer-name">{{ profile.name }}</span>
                  <span class="transfer-meta">
                    <span>{{ profile.deviceModel ?? tr("通用") }}</span>
                    <span v-if="exportMode === 'preset' && !isPresetEligible(profile)">
                      {{ tr("默认方案不可作为推荐方案") }}
                    </span>
                  </span>
                </Checkbox>
              </li>
            </ul>
            <p v-else class="transfer-empty">{{ tr("还没有可导出的方案") }}</p>
          </template>

          <template v-else>
            <p class="transfer-hint">{{ tr("从文件导入方案，会覆盖当前方案列表。") }}</p>
            <Button variant="primary" :disabled="props.busy" @click="startImport">
              <FileUp :size="14" />
              {{ tr("选择文件并导入") }}
            </Button>
          </template>
        </div>

        <footer class="transfer-foot">
          <Button size="sm" variant="ghost" @click="close">{{ tr("取消") }}</Button>
          <Button
            v-if="mode === 'export'"
            size="sm"
            variant="primary"
            :disabled="!canExport"
            @click="submit"
          >
            {{ exportMode === "preset" ? tr("选择文件夹并导出") : tr("选择位置并导出") }}
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
  width: min(520px, calc(100vw - 32px));
  max-height: calc(100vh - 48px);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.transfer-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  padding: var(--space-5) var(--space-5) var(--space-3);
}

.transfer-head h2 {
  margin: 0;
  font-size: var(--text-lg);
  letter-spacing: -0.02em;
}

.transfer-close {
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

.transfer-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.transfer-tabs {
  padding: 0 var(--space-5);
}

.transfer-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-3);
  overflow: auto;
  padding: var(--space-4) var(--space-5);
}

.transfer-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-3);
}

.transfer-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
}

.transfer-hint {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.5;
}

.transfer-list-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
}

.transfer-list {
  display: flex;
  flex-direction: column;
  gap: 2px;
  margin: 0;
  padding: 0;
  max-height: 260px;
  overflow-y: auto;
  list-style: none;
}

.transfer-item {
  border-radius: var(--radius-sm);
  padding: var(--space-2);
}

.transfer-item:hover {
  background: var(--bg-hover);
}

.transfer-name {
  display: block;
  color: var(--text);
  font-size: var(--text-sm);
}

.transfer-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.transfer-empty {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.transfer-foot {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}
</style>
