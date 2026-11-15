<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from "vue";
import {
  ArrowDown,
  ArrowUp,
  Eye,
  MessageSquare,
  Plus,
  Trash2,
  TriangleAlert,
} from "lucide-vue-next";
import {
  errorMessage,
  getCommonPhrasesDocument,
  openCommonPhrasesPanel,
  saveCommonPhrases,
} from "../lib/api";
import { confirmAction } from "../lib/confirm";
import { tr, trDynamic } from "../lib/i18n";
import { pushToast } from "../lib/toast";
import type { CommonPhrasesDocument } from "../lib/types";
import Button from "./ui/Button.vue";
import EmptyState from "./ui/EmptyState.vue";
import IconButton from "./ui/IconButton.vue";
import LoadingState from "./ui/LoadingState.vue";
import Notice from "./ui/Notice.vue";
import StatusPill from "./ui/StatusPill.vue";
import TextInput from "./ui/TextInput.vue";

/** 名称上限：与面板单行显示宽度对齐。 */
const LABEL_LIMIT = 24;
/** 插入文本上限：与保存校验一致。 */
const TEXT_LIMIT = 500;
/** 自动保存的合并窗口，与设置页其它项保持一致。 */
const SAVE_DELAY_MS = 250;

type SaveState = "saving" | "saved" | "error";

const phrasesDocument = ref<CommonPhrasesDocument | null>(null);
const loading = ref(true);
const loadError = ref<string | null>(null);
const saveState = ref<SaveState>("saved");
const previewing = ref(false);

const root = ref<HTMLElement | null>(null);

let saveTimer: number | undefined;
let activeSave: Promise<void> | null = null;
let saveAgain = false;
/** 首次读取完成前不要触发自动保存，否则会把空文档写回去。 */
let hydrated = false;

const slots = computed(() => phrasesDocument.value?.slots ?? []);

/** 名称重复的行 → 第一条同名的序号（面板上会出现两个一样的条目）。 */
const duplicateOf = computed<Record<number, number>>(() => {
  const seen = new Map<string, number>();
  const duplicates: Record<number, number> = {};
  slots.value.forEach((slot, index) => {
    const key = slot.label.trim();
    if (!key) return;
    const first = seen.get(key);
    if (first === undefined) {
      seen.set(key, index);
      return;
    }
    duplicates[index] = first;
  });
  return duplicates;
});

function rowLabel(index: number): string {
  return `${tr("名称")} · ${tr("第 {index} 位", { index: index + 1 })}`;
}

function textAreaLabel(index: number): string {
  return `${tr("要插入的文本")} · ${tr("第 {index} 位", { index: index + 1 })}`;
}

function addPhrase() {
  if (!phrasesDocument.value) return;
  const index = phrasesDocument.value.slots.length;
  phrasesDocument.value.slots.push({
    button: `phrase-${Date.now()}-${index}`,
    label: "",
    text: "",
  });
  void nextTick(() => {
    root.value?.querySelector<HTMLInputElement>(`[data-phrase-name="${index}"]`)?.focus();
  });
}

async function removePhrase(index: number) {
  const list = phrasesDocument.value?.slots;
  const slot = list?.[index];
  if (!list || !slot) return;
  const label = slot.label.trim();
  const text = slot.text.trim();
  if (label || text) {
    const name = label || text.slice(0, 20);
    const confirmed = await confirmAction({
      title: trDynamic("删除这条常用语？"),
      message: trDynamic("“{name}”将从遥控器面板中移除。", { name }),
      confirmLabel: tr("删除"),
      cancelLabel: tr("取消"),
      danger: true,
    });
    if (!confirmed) return;
  }
  // 等待期间文档可能已经变化：只删除仍在这一行的条目。
  const current = phrasesDocument.value?.slots;
  if (!current || current[index] !== slot) return;
  current.splice(index, 1);
}

function movePhrase(index: number, delta: number) {
  const list = phrasesDocument.value?.slots;
  if (!list) return;
  const target = index + delta;
  if (target < 0 || target >= list.length) return;
  const [moved] = list.splice(index, 1);
  if (!moved) return;
  list.splice(target, 0, moved);
}

function scheduleSave() {
  if (!hydrated || !phrasesDocument.value) return;
  saveState.value = "saving";
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    saveTimer = undefined;
    void persist();
  }, SAVE_DELAY_MS);
}

function persist(): Promise<void> {
  if (!phrasesDocument.value) return Promise.resolve();
  if (activeSave) {
    saveAgain = true;
    return activeSave;
  }
  activeSave = (async () => {
    try {
      do {
        saveAgain = false;
        const snapshot: CommonPhrasesDocument = {
          slots: slots.value.map((slot) => ({ ...slot })),
        };
        await saveCommonPhrases(snapshot);
      } while (saveAgain);
      saveState.value = "saved";
    } catch {
      saveState.value = "error";
    } finally {
      activeSave = null;
    }
  })();
  return activeSave;
}

function retrySave() {
  saveState.value = "saving";
  void persist();
}

/** 把在途的手动编辑立即落盘（切分区、预览面板前调用）。 */
function flushPendingSave(): Promise<void> {
  if (saveTimer === undefined) return activeSave ?? Promise.resolve();
  window.clearTimeout(saveTimer);
  saveTimer = undefined;
  return persist();
}

async function load() {
  loading.value = true;
  loadError.value = null;
  hydrated = false;
  try {
    phrasesDocument.value = await getCommonPhrasesDocument();
    saveState.value = "saved";
  } catch (cause) {
    loadError.value = errorMessage(cause);
    phrasesDocument.value = null;
  } finally {
    loading.value = false;
    await nextTick();
    hydrated = true;
  }
}

async function preview() {
  if (previewing.value) return;
  previewing.value = true;
  try {
    // 面板打开时会重新读盘：先把在途编辑落盘，否则预览看到的是旧内容。
    await flushPendingSave();
    if (saveState.value === "error") {
      pushToast(tr("保存失败"), "error");
      return;
    }
    const opened = await openCommonPhrasesPanel();
    if (!opened) pushToast(tr("无法打开常用语面板"), "error");
  } catch (cause) {
    pushToast(errorMessage(cause), "error");
  } finally {
    previewing.value = false;
  }
}

watch(() => phrasesDocument.value?.slots, scheduleSave, { deep: true });

onMounted(load);

onBeforeUnmount(() => {
  void flushPendingSave();
});
</script>

<template>
  <div ref="root" class="common-phrases" role="group" :aria-label="tr('常用语')">
    <LoadingState v-if="loading" class="phrase-loading" label="正在加载常用语…" />

    <Notice v-else-if="loadError" kind="error" class="phrase-notice">
      <span class="phrase-notice-text">{{ loadError }}</span>
      <Button size="sm" variant="ghost" @click="load">{{ tr("重试") }}</Button>
    </Notice>

    <template v-else-if="phrasesDocument">
      <div class="phrase-panel">
        <div class="phrase-toolbar">
          <div class="phrase-toolbar-actions">
            <StatusPill
              v-if="saveState === 'saving'"
              tone="neutral"
              dot
              :label="trDynamic('保存中…')"
            />
            <StatusPill
              v-else-if="saveState === 'saved'"
              tone="success"
              dot
              :label="tr('已保存')"
            />
            <IconButton :label="tr('预览面板')" :disabled="previewing" size="md" @click="preview">
              <Eye :size="16" />
            </IconButton>
            <Button size="sm" variant="primary" @click="addPhrase">
              <Plus :size="14" />
              {{ tr("添加") }}
            </Button>
          </div>
        </div>

        <Notice v-if="saveState === 'error'" kind="error" class="phrase-notice">
          <span class="phrase-notice-text">{{ trDynamic("常用语没有保存成功，请重试。") }}</span>
          <Button size="sm" variant="ghost" @click="retrySave">{{ tr("重试") }}</Button>
        </Notice>

        <table v-if="slots.length" class="phrase-table">
          <caption class="sr-only">
            {{
              tr("常用语")
            }}
          </caption>
          <thead>
            <tr>
              <th scope="col" class="phrase-col-index">#</th>
              <th scope="col" class="phrase-col-name">{{ tr("名称") }}</th>
              <th scope="col">{{ tr("内容") }}</th>
              <th scope="col" class="phrase-col-actions">{{ tr("操作") }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(slot, index) in slots" :key="slot.button" class="phrase-row">
              <td class="phrase-col-index">
                <span
                  class="phrase-cap"
                  :class="{ 'is-duplicate': duplicateOf[index] !== undefined }"
                  aria-hidden="true"
                  >{{ index + 1 }}</span
                >
                <span
                  v-if="duplicateOf[index] !== undefined"
                  class="phrase-index-warning"
                  :title="
                    trDynamic('与第 {index} 条名称相同', { index: (duplicateOf[index] ?? 0) + 1 })
                  "
                >
                  <TriangleAlert :size="13" aria-hidden="true" />
                </span>
              </td>
              <td class="phrase-col-name">
                <div class="phrase-name-field">
                  <TextInput
                    v-model="slot.label"
                    :maxlength="LABEL_LIMIT"
                    :placeholder="tr('名称（显示在面板上）')"
                    :aria-label="rowLabel(index)"
                    :data-phrase-name="index"
                  />
                </div>
              </td>
              <td>
                <div class="phrase-content">
                  <textarea
                    v-model="slot.text"
                    class="phrase-text"
                    rows="1"
                    :maxlength="TEXT_LIMIT"
                    :placeholder="tr('要插入的文本')"
                    :aria-label="textAreaLabel(index)"
                  />
                  <span
                    v-if="slot.text.length"
                    class="phrase-text-count"
                    :class="{ 'is-limit': slot.text.length >= TEXT_LIMIT }"
                    aria-hidden="true"
                  >
                    {{ slot.text.length }}/{{ TEXT_LIMIT }}
                  </span>
                </div>
              </td>
              <td class="phrase-col-actions">
                <div class="phrase-row-actions">
                  <IconButton
                    :label="tr('上移')"
                    size="sm"
                    :disabled="index === 0"
                    @click="movePhrase(index, -1)"
                  >
                    <ArrowUp :size="15" />
                  </IconButton>
                  <IconButton
                    :label="tr('下移')"
                    size="sm"
                    :disabled="index === slots.length - 1"
                    @click="movePhrase(index, 1)"
                  >
                    <ArrowDown :size="15" />
                  </IconButton>
                  <IconButton
                    :label="tr('删除')"
                    variant="danger"
                    size="sm"
                    @click="removePhrase(index)"
                  >
                    <Trash2 :size="15" />
                  </IconButton>
                </div>
              </td>
            </tr>
          </tbody>
        </table>

        <EmptyState
          v-else
          class="phrase-empty"
          title="还没有常用语"
          description="添加常用语；绑定「常用语」动作后，在面板中用 ↑/↓ 选择、OK 插入，返回键关闭。"
        >
          <template #icon><MessageSquare :size="20" /></template>
          <template #actions>
            <Button variant="primary" size="sm" @click="addPhrase">
              <Plus :size="14" />
              {{ tr("添加") }}
            </Button>
          </template>
        </EmptyState>
      </div>
    </template>
  </div>
</template>

<style scoped>
.common-phrases {
  display: flex;
  flex-direction: column;
  border-radius: var(--radius-lg);
}

/* 与方案页「表格视图」（ProfileWorkbench 的 matrix 面板）同一套骨架：
   卡片边框 + 圆角 + 溢出裁切，工具条 12px 内边距 + 1px 下边框，
   表格满铺卡片，表头 --bg-subtle，行分隔线落在 td 上。 */
.phrase-panel {
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-sm);
}
.phrase-toolbar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
  border-bottom: 1px solid var(--border);
  padding: var(--space-3);
}

.phrase-toolbar-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.phrase-notice {
  margin: var(--space-3);
}

.phrase-notice-text {
  margin-right: var(--space-2);
}

.phrase-loading {
  margin: var(--space-4);
}

.phrase-empty {
  margin: var(--space-4);
}

.phrase-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  table-layout: fixed;
}

.phrase-table th {
  background: var(--bg-subtle);
  padding: 9px 12px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.08em;
  text-align: left;
  text-transform: uppercase;
  white-space: nowrap;
}

/* 分隔线与方案页矩阵一致：每条 td 自带 1px 上边框，首行不画（表头自己不画线）。 */
.phrase-table td {
  border-top: 1px solid var(--border);
  padding: 4px 12px;
  vertical-align: middle;
}

.phrase-table tbody tr:hover td {
  background: var(--bg-hover);
}

/* 编辑中的行沿用矩阵的选中底色比例（accent-soft 52%），两处观感一致。 */
.phrase-table tbody tr:focus-within td {
  background: color-mix(in srgb, var(--accent-soft) 52%, transparent);
}

.phrase-table th.phrase-col-index,
.phrase-table td.phrase-col-index {
  width: 56px;
  padding-inline: 8px;
  text-align: center;
}

.phrase-table th.phrase-col-name,
.phrase-table td.phrase-col-name {
  width: 200px;
}

.phrase-table th.phrase-col-actions,
.phrase-table td.phrase-col-actions {
  width: 112px;
  padding-inline: 8px;
  text-align: right;
}

/* 序号做成一枚键帽，与矩阵视图的 .key-cap 同一形态（30px 改 26px，行高更矮）。 */
.phrase-cap {
  display: inline-grid;
  place-items: center;
  min-width: 26px;
  height: 26px;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-raised);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 10px;
  font-variant-numeric: tabular-nums;
}

.phrase-row:focus-within .phrase-cap,
.phrase-row:hover .phrase-cap {
  border-color: var(--border-strong);
  color: var(--text);
}

/* 名称重复是可修的小问题：键帽转警告色，不整行报警。 */
.phrase-cap.is-duplicate {
  border-color: var(--warning);
  color: var(--warning-text);
  background: color-mix(in srgb, var(--warning) 12%, var(--bg-raised));
}

.phrase-name-field {
  position: relative;
  min-width: 0;
}

.phrase-index-warning {
  display: inline-flex;
  margin-left: 3px;
  color: var(--warning-text);
  vertical-align: -2px;
}

/* 表格里的输入项默认贴成文字，悬停/聚焦才显出控件。 */
.phrase-row :deep(.ui-input) {
  border-color: transparent;
  background: transparent;
}

.phrase-row :deep(.ui-input:hover:not(:disabled)) {
  border-color: var(--border-strong);
  background: var(--bg-elevated);
}

.phrase-row :deep(.ui-input:focus-visible) {
  border-color: var(--accent);
  background: var(--bg-elevated);
}

.phrase-content {
  position: relative;
  /* 给计数留一条固定车位：输入框永远不压到它，文字也不会被计数挡住。 */
  padding-right: 58px;
  min-width: 0;
  /* 撑住这一格的高度：焦点态输入框改成绝对定位后不再有流内高度，这一格若跟着塌成 0，
     `td` 的 `vertical-align: middle` 会把 0 高度盒子居中，输入框看起来就整体向下漂移。 */
  height: 36px;
}

.phrase-text-count {
  position: absolute;
  top: 50%;
  right: 8px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
  translate: 0 -50%;
  pointer-events: none;
}

.phrase-text-count.is-limit {
  color: var(--warning-text);
}

.phrase-text {
  box-sizing: border-box;
  /* 块级：消掉 inline-block 的基线间隙，行高才和 36px 的表格单元格严格对齐。 */
  display: block;
  width: 100%;
  /* 表格行固定单行：不换行，也就不会在格子底部露出半行字。
     36px 与矩阵单元格的 min-height 对齐，两处行高一致。 */
  height: 36px;
  min-height: 36px;
  overflow: hidden;
  white-space: pre;
  resize: none;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  padding: 6px 10px;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  line-height: 20px;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.phrase-text::placeholder {
  color: var(--text-muted);
}

.phrase-text:hover {
  border-color: var(--border-strong);
  background: var(--bg-elevated);
}

/* 编辑时浮起展开，长文本、换行都能看全；表格行高不变。 */
.phrase-text:focus-visible {
  position: absolute;
  top: 0;
  right: 58px;
  left: 0;
  z-index: 3;
  width: auto;
  height: auto;
  min-height: 36px;
  max-height: 160px;
  overflow-y: auto;
  field-sizing: content;
  border-color: var(--accent);
  background: var(--bg-elevated);
  white-space: pre-wrap;
  box-shadow: var(--shadow-md);
  outline: none;
}

.phrase-row-actions {
  display: flex;
  align-items: center;
  gap: 2px;
}
</style>
