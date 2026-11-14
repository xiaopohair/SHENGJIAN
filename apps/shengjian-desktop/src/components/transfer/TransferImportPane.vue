<script setup lang="ts">
import { computed } from "vue";
import { FileJson, TriangleAlert, Upload } from "lucide-vue-next";
import { tr, type TranslationKey } from "../../lib/i18n";
import type {
  ConflictPolicy,
  ImportAction,
  ImportStatus,
  ResolvedImportRow,
} from "../../lib/profileTransfer";

const props = defineProps<{
  file: { name: string; meta: string } | null;
  rows: ResolvedImportRow[];
  policy: ConflictPolicy;
  onlyCompatible: boolean;
  model: string | null;
}>();

const emit = defineEmits<{
  pickFile: [];
  clearFile: [];
  changePolicy: [policy: ConflictPolicy];
  override: [id: string, action: ImportAction];
  toggleCompatible: [];
}>();

const STATUS_LABEL: Record<ImportStatus, TranslationKey> = {
  new: "新增",
  update: "更新",
  conflict: "冲突",
};

const ACTION_LABEL: Record<ImportAction, TranslationKey> = {
  add: "新增",
  overwrite: "覆盖",
  keep: "保留",
  skip: "跳过",
};

function statusLabel(row: ResolvedImportRow): string {
  return tr(STATUS_LABEL[row.status]);
}

function actionLabel(action: ImportAction): string {
  return tr(ACTION_LABEL[action]);
}

/** 冲突行只有三种落法，顺序固定：替换、并行、放弃。 */
const ACTION_OPTIONS: ImportAction[] = ["overwrite", "keep", "skip"];

const policyOptions = computed(() => [
  {
    value: "overwrite" as ConflictPolicy,
    title: tr("覆盖本地"),
    detail: tr("用文件里的版本替换，本地原内容不保留。"),
  },
  {
    value: "keep" as ConflictPolicy,
    title: tr("保留两者"),
    detail: tr("导入为新方案，名字后面加「副本」。"),
  },
  {
    value: "skip" as ConflictPolicy,
    title: tr("跳过"),
    detail: tr("同名方案保持不动。"),
  },
]);

const compatibleLabel = computed(() =>
  props.model
    ? tr("只导入与 {model} 兼容的方案", { model: props.model })
    : tr("只导入与当前遥控器兼容的方案"),
);

/** 冲突行才给逐条决定；默认方案和不兼容是硬规则，直接说明为什么跳过。 */
function showSegment(row: ResolvedImportRow): boolean {
  return row.status === "conflict" && !row.locked && row.compatible;
}
</script>

<template>
  <div class="transfer-split">
    <section class="transfer-list-pane">
      <header class="pane-head">
        <h3 class="pane-label">{{ tr("导入") }}</h3>
        <span v-if="file" class="pane-count">{{
          tr("{count} 个方案", { count: rows.length })
        }}</span>
      </header>

      <template v-if="!file">
        <button type="button" class="drop-zone" @click="emit('pickFile')">
          <span class="drop-icon"><Upload :size="20" aria-hidden="true" /></span>
          <strong>{{ tr("选择方案文件") }}</strong>
          <span class="drop-hint">{{
            tr("支持 .json 备份文件，也可以选一个推荐方案的 .json。")
          }}</span>
        </button>
        <p class="pane-hint">{{ tr("导入只新增或覆盖方案，不会清空现有配置。") }}</p>
      </template>

      <template v-else>
        <div class="file-card">
          <span class="file-icon"><FileJson :size="16" aria-hidden="true" /></span>
          <span class="file-copy">
            <span class="file-name" :title="file.name">{{ file.name }}</span>
            <span class="file-meta">{{ file.meta }}</span>
          </span>
          <button type="button" class="link-button" @click="emit('clearFile')">
            {{ tr("更换") }}
          </button>
        </div>

        <ul class="import-rows" :aria-label="tr('将导入')">
          <li v-for="row in rows" :key="row.profile.id" class="import-row">
            <span class="status-chip" :class="`status-${row.status}`">{{ statusLabel(row) }}</span>
            <span class="row-main">
              <span class="row-name">
                {{ row.profile.name }}
                <em v-if="!row.compatible" class="row-tag warn">{{ tr("型号不符") }}</em>
              </span>
              <span class="row-meta">
                {{ row.profile.deviceModel ?? tr("通用") }} · {{ row.binds }} {{ tr("键位") }} ·
                {{ tr("{count} 个应用", { count: row.profile.appTargets.length }) }}
              </span>
            </span>
            <span class="row-side">
              <span v-if="row.locked" class="row-note">{{ tr("默认方案不覆盖") }}</span>
              <span v-else-if="!row.compatible && onlyCompatible" class="row-note">{{
                tr("跳过")
              }}</span>
              <span v-else-if="showSegment(row)" class="mini-segment" role="group">
                <button
                  v-for="option in ACTION_OPTIONS"
                  :key="option"
                  type="button"
                  :aria-pressed="row.action === option"
                  @click="emit('override', row.profile.id, option)"
                >
                  {{ actionLabel(option) }}
                </button>
              </span>
              <span v-else-if="row.action !== 'add'" class="row-note">
                {{ actionLabel(row.action) }}
              </span>
            </span>
          </li>
        </ul>
      </template>
    </section>

    <section class="transfer-side">
      <h3 class="pane-label side-label">{{ tr("处理方式") }}</h3>
      <div class="side-scroll">
        <template v-if="file">
          <div class="side-field">
            <span class="side-label">{{ tr("遇到同名方案") }}</span>
            <div class="option-list" role="radiogroup" :aria-label="tr('遇到同名方案')">
              <button
                v-for="option in policyOptions"
                :key="option.value"
                type="button"
                class="option-card"
                role="radio"
                :aria-checked="policy === option.value"
                @click="emit('changePolicy', option.value)"
              >
                <span class="option-radio" aria-hidden="true" />
                <span class="option-copy">
                  <span class="option-title">{{ option.title }}</span>
                  <span class="option-detail">{{ option.detail }}</span>
                </span>
              </button>
            </div>
          </div>

          <div class="side-field">
            <label class="switch-row">
              <span>{{ compatibleLabel }}</span>
              <input
                type="checkbox"
                class="switch-input"
                :checked="onlyCompatible"
                @change="emit('toggleCompatible')"
              />
              <span class="switch-track" aria-hidden="true" />
            </label>
          </div>

          <p class="side-note">{{ tr("导入不会删除本地方案；同一 id 的方案会被更新。") }}</p>
        </template>

        <template v-else>
          <p class="side-note">{{ tr("先选文件，读到内容后再决定同名方案怎么办。") }}</p>
          <div class="side-field">
            <span class="side-label">{{ tr("选好文件后会检查") }}</span>
            <ul class="check-list">
              <li>
                <span>{{ tr("同名方案") }}</span>
                <em>{{ tr("可以逐个决定覆盖或保留") }}</em>
              </li>
              <li>
                <span>{{ tr("遥控器型号") }}</span>
                <em>{{ tr("与当前遥控器不兼容的会标出来") }}</em>
              </li>
              <li>
                <span>{{ tr("键位与引用") }}</span>
                <em>{{ tr("一并带过来，不留半条方案") }}</em>
              </li>
            </ul>
          </div>
          <p class="macro-hint">
            <TriangleAlert :size="13" aria-hidden="true" />
            <span>{{ tr("备份文件不含宏定义，导入后引用宏的键位需要本机已有这些宏。") }}</span>
          </p>
        </template>
      </div>
    </section>
  </div>
</template>

<style scoped>
.transfer-split {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 324px;
  flex: 1 1 auto;
  min-height: 0;
}

.transfer-list-pane {
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: var(--space-3) 0 0;
}

.pane-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  padding: 0 var(--space-4) var(--space-2);
}

.pane-label {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 600;
  letter-spacing: 0.06em;
}

.pane-count {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

.drop-zone {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin: var(--space-2) var(--space-4) var(--space-3);
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--bg-subtle);
  padding: var(--space-5);
  color: var(--text-muted);
  font: inherit;
  text-align: center;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.drop-zone:hover {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.drop-icon {
  display: grid;
  place-items: center;
  width: 40px;
  height: 40px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-elevated);
  color: var(--text-secondary);
}

.drop-zone strong {
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 650;
}

.drop-hint {
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.pane-hint {
  margin: 0;
  padding: 0 var(--space-4) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-2xs);
}

.file-card {
  display: grid;
  grid-template-columns: 34px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  margin: var(--space-2) var(--space-4) var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: 8px 10px;
}

.file-icon {
  display: grid;
  place-items: center;
  width: 34px;
  height: 34px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-elevated);
  color: var(--accent);
}

.file-copy {
  min-width: 0;
}

.file-name {
  display: block;
  overflow: hidden;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.file-meta {
  display: block;
  margin-top: 1px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

.link-button {
  border: 0;
  border-radius: var(--radius-xs);
  background: transparent;
  padding: 2px 5px;
  color: var(--text-secondary);
  font: inherit;
  font-size: var(--text-2xs);
  cursor: pointer;
}

.link-button:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.import-rows {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-1);
  min-height: 0;
  margin: 0;
  padding: 0 var(--space-2) var(--space-2);
  overflow-y: auto;
  list-style: none;
  scrollbar-width: thin;
}

.import-row {
  display: grid;
  grid-template-columns: auto minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  padding: 8px 9px;
  border-radius: var(--radius-md);
  border: 1px solid var(--border);
}

.import-row:hover {
  background: var(--bg-hover);
}

.status-chip {
  border: 1px solid transparent;
  border-radius: var(--radius-xs);
  padding: 1px 6px;
  font-size: 10px;
  font-weight: 600;
  white-space: nowrap;
}

.status-new {
  border-color: color-mix(in srgb, var(--accent) 32%, transparent);
  background: var(--accent-soft);
  color: var(--accent);
}

.status-update {
  border-color: color-mix(in srgb, var(--info) 30%, transparent);
  background: color-mix(in srgb, var(--info) 12%, transparent);
  color: var(--info-text);
}

.status-conflict {
  border-color: color-mix(in srgb, var(--warning) 32%, transparent);
  background: color-mix(in srgb, var(--warning) 10%, transparent);
  color: var(--warning-text);
}

.row-main {
  min-width: 0;
}

.row-name {
  display: flex;
  align-items: center;
  gap: 6px;
  overflow: hidden;
  color: var(--text);
  font-size: var(--text-sm);
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-tag {
  flex: 0 0 auto;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xs);
  padding: 0 5px;
  font-size: 10px;
  font-style: normal;
  font-weight: 600;
}

.row-tag.warn {
  border-color: color-mix(in srgb, var(--danger) 30%, transparent);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
  color: var(--danger-text);
}

.row-meta {
  display: block;
  margin-top: 1px;
  overflow: hidden;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-side {
  display: flex;
  align-items: center;
  gap: 6px;
}

.row-note {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  white-space: nowrap;
}

.mini-segment {
  display: inline-flex;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-raised);
}

.mini-segment button {
  border: 0;
  border-right: 1px solid var(--border);
  background: transparent;
  padding: 3px 7px;
  color: var(--text-muted);
  font: inherit;
  font-size: 10.5px;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}

.mini-segment button:last-child {
  border-right: 0;
}

.mini-segment button:hover {
  background: var(--bg-hover);
  color: var(--text-secondary);
}

.mini-segment button[aria-pressed="true"] {
  background: var(--bg-elevated);
  color: var(--text);
  font-weight: 600;
}

.transfer-side {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-left: 1px solid var(--border);
  background: color-mix(in srgb, var(--bg-subtle) 55%, transparent);
}

.side-label {
  margin: var(--space-3) var(--space-4) 0;
}

.side-scroll {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
  min-height: 0;
  padding: var(--space-2) var(--space-4) var(--space-3);
  overflow-y: auto;
}

.side-field {
  display: flex;
  flex-direction: column;
  gap: 7px;
}

.side-field > .side-label {
  margin: 0;
}

.option-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.option-card {
  display: grid;
  grid-template-columns: 14px minmax(0, 1fr);
  gap: 9px;
  align-items: start;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: 7px 10px;
  color: inherit;
  font: inherit;
  text-align: left;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease;
}

.option-card:hover {
  border-color: var(--border-strong);
}

.option-card[aria-checked="true"] {
  border-color: var(--accent);
  background: var(--accent-soft);
}

.option-radio {
  position: relative;
  margin-top: 2px;
  width: 14px;
  height: 14px;
  border: 1.5px solid var(--border-control);
  border-radius: 50%;
}

.option-card[aria-checked="true"] .option-radio {
  border-color: var(--accent);
}

.option-radio::after {
  content: "";
  position: absolute;
  inset: 3px;
  border-radius: 50%;
  background: var(--accent);
  opacity: 0;
  transition: opacity var(--duration-fast) ease;
}

.option-card[aria-checked="true"] .option-radio::after {
  opacity: 1;
}

.option-title {
  display: block;
  color: var(--text);
  font-size: var(--text-xs);
  font-weight: 650;
}

.option-detail {
  display: block;
  margin-top: 2px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.45;
}

.switch-row {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  color: var(--text-secondary);
  font-size: var(--text-xs);
  line-height: 1.4;
  cursor: pointer;
}

.switch-input {
  position: absolute;
  width: 1px;
  height: 1px;
  opacity: 0;
  clip-path: inset(50%);
}

.switch-track {
  position: relative;
  flex: 0 0 32px;
  width: 32px;
  height: 18px;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-pill);
  background: var(--bg-hover);
  transition:
    background var(--duration-base) ease,
    border-color var(--duration-base) ease;
}

.switch-track::after {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--text-muted);
  transition:
    left var(--duration-base) var(--ease-out, ease),
    background var(--duration-base) ease;
}

.switch-input:checked + .switch-track {
  border-color: var(--accent);
  background: var(--accent);
}

.switch-input:checked + .switch-track::after {
  left: 16px;
  background: var(--text-on-accent);
}

.side-note {
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.55;
}

.check-list {
  display: flex;
  flex-direction: column;
  gap: 7px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.check-list li {
  display: flex;
  flex-direction: column;
  gap: 1px;
  color: var(--text-secondary);
  font-size: var(--text-xs);
}

.check-list em {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-style: normal;
}

.macro-hint {
  display: flex;
  gap: var(--space-2);
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.macro-hint svg {
  flex: 0 0 auto;
  margin-top: 1px;
}
</style>
