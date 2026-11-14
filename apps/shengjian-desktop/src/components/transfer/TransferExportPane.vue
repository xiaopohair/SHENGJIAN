<script setup lang="ts">
import { computed, ref } from "vue";
import { TriangleAlert } from "lucide-vue-next";
import { tr } from "../../lib/i18n";
import { profileStats, transferSummary, type TransferFormat } from "../../lib/profileTransfer";
import type { ButtonProfile } from "../../lib/types";
import TransferRemotePreview from "./TransferRemotePreview.vue";

const props = defineProps<{
  profiles: ButtonProfile[];
  selectedIds: string[];
  format: TransferFormat;
}>();

const emit = defineEmits<{
  toggle: [id: string];
  toggleAll: [];
  changeFormat: [format: TransferFormat];
}>();

const hoveredId = ref<string | null>(null);

interface Row {
  profile: ButtonProfile;
  binds: number;
  appCount: number;
  selectable: boolean;
}

const rows = computed<Row[]>(() =>
  props.profiles.map((profile) => {
    const stats = profileStats(profile);
    return {
      profile,
      binds: stats.binds,
      appCount: stats.appCount,
      // 默认方案是该型号的基线，不能被整理成推荐方案下发。
      selectable: !(props.format === "preset" && profile.isDefault),
    };
  }),
);

const selectedProfiles = computed(() =>
  props.profiles.filter((profile) => props.selectedIds.includes(profile.id)),
);

const summary = computed(() => transferSummary(selectedProfiles.value));

const allSelected = computed(() => {
  const selectable = rows.value.filter((row) => row.selectable);
  return (
    selectable.length > 0 && selectable.every((row) => props.selectedIds.includes(row.profile.id))
  );
});

const previewProfile = computed<ButtonProfile | null>(() => {
  const hovered = props.profiles.find((profile) => profile.id === hoveredId.value);
  if (hovered) return hovered;
  return selectedProfiles.value[0] ?? props.profiles[0] ?? null;
});

const formatOptions = computed(() => [
  {
    value: "document" as TransferFormat,
    title: tr("备份文件"),
    detail: tr("打成一个文件，可原样恢复。"),
  },
  {
    value: "preset" as TransferFormat,
    title: tr("推荐方案"),
    detail: tr("每个方案一个 .json，便于分发。"),
  },
]);
</script>

<template>
  <div class="transfer-split">
    <section class="transfer-list-pane" aria-labelledby="transfer-list-label">
      <header class="pane-head">
        <h3 id="transfer-list-label" class="pane-label">{{ tr("选择方案") }}</h3>
        <div class="pane-head-actions">
          <span class="pane-count">
            {{
              tr("已选 {picked}/{total}", { picked: selectedIds.length, total: profiles.length })
            }}
          </span>
          <button type="button" class="link-button" @click="emit('toggleAll')">
            {{ allSelected ? tr("全不选") : tr("全选") }}
          </button>
        </div>
      </header>

      <ul
        class="profile-rows"
        role="listbox"
        aria-multiselectable="true"
        :aria-label="tr('选择方案')"
      >
        <li
          v-for="row in rows"
          :key="row.profile.id"
          class="profile-row"
          :class="{
            selected: selectedIds.includes(row.profile.id) && row.selectable,
            disabled: !row.selectable,
          }"
          role="option"
          :aria-selected="selectedIds.includes(row.profile.id) && row.selectable"
          :aria-disabled="!row.selectable"
          :tabindex="row.selectable ? 0 : -1"
          @click="row.selectable && emit('toggle', row.profile.id)"
          @mouseenter="hoveredId = row.profile.id"
          @mouseleave="hoveredId = null"
          @keydown.enter.prevent="row.selectable && emit('toggle', row.profile.id)"
          @keydown.space.prevent="row.selectable && emit('toggle', row.profile.id)"
        >
          <span class="row-check" aria-hidden="true" />
          <span class="row-main">
            <span class="row-name">
              {{ row.profile.name }}
              <em v-if="row.profile.isDefault" class="row-tag">{{ tr("默认") }}</em>
              <em v-if="!row.selectable" class="row-tag warn">{{ tr("推荐方案不含默认") }}</em>
            </span>
            <span class="row-meta">
              {{ row.binds }} {{ tr("键位") }} ·
              {{ row.appCount ? tr("{count} 个应用", { count: row.appCount }) : tr("未绑定应用") }}
            </span>
          </span>
          <span class="row-model">{{ row.profile.deviceModel ?? tr("通用") }}</span>
        </li>
      </ul>

      <p v-if="!profiles.length" class="pane-hint">{{ tr("还没有可导出的方案") }}</p>
      <p v-else-if="summary.count === 0" class="pane-hint">
        {{ tr("全选可以一次备份全部方案。") }}
      </p>
      <p v-else class="pane-hint">{{ tr("导出只读，不改动本机配置") }}</p>
    </section>

    <section class="transfer-side">
      <h3 class="pane-label side-label">{{ tr("导出设置") }}</h3>
      <div class="side-scroll">
        <TransferRemotePreview v-if="previewProfile" :profile="previewProfile" />

        <div class="side-field">
          <span class="side-label">{{ tr("导出格式") }}</span>
          <div class="option-list" role="radiogroup" :aria-label="tr('导出格式')">
            <button
              v-for="option in formatOptions"
              :key="option.value"
              type="button"
              class="option-card"
              role="radio"
              :aria-checked="format === option.value"
              @click="emit('changeFormat', option.value)"
            >
              <span class="option-radio" aria-hidden="true" />
              <span class="option-copy">
                <span class="option-title">{{ option.title }}</span>
                <span class="option-detail">{{ option.detail }}</span>
              </span>
            </button>
          </div>
        </div>

        <p v-if="summary.macros > 0" class="macro-note">
          <TriangleAlert :size="13" aria-hidden="true" />
          <span>
            {{
              tr("选中的方案引用了 {count} 个宏，备份文件不含宏定义。", { count: summary.macros })
            }}
            {{ tr("换台电脑导入后，这些键位会显示为失效。") }}
          </span>
        </p>
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

.pane-head-actions {
  display: flex;
  align-items: center;
  gap: var(--space-2);
}

.pane-count {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

.link-button {
  border: 0;
  border-radius: var(--radius-xs);
  background: transparent;
  padding: 2px 5px;
  color: var(--accent);
  font: inherit;
  font-size: var(--text-2xs);
  font-weight: 600;
  cursor: pointer;
}

.link-button:hover {
  background: var(--accent-soft);
}

.profile-rows {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-1);
  min-height: 0;
  margin: 0;
  padding: 0 var(--space-2);
  overflow-y: auto;
  list-style: none;
  scrollbar-width: thin;
}

.profile-row {
  display: grid;
  grid-template-columns: 18px minmax(0, 1fr) auto;
  gap: 10px;
  align-items: center;
  padding: 8px 9px;
  border-radius: var(--radius-md);
  cursor: pointer;
  transition: background var(--duration-fast) ease;
  border: 1px solid var(--border);
}

.profile-row:hover {
  background: var(--bg-hover);
}

.profile-row.selected {
  background: color-mix(in srgb, var(--accent) 9%, transparent);
  border-color: var(--accent-soft-strong);
}

.profile-row.selected:hover {
  background: color-mix(in srgb, var(--accent) 14%, transparent);
}

.profile-row.disabled {
  opacity: 0.5;
  cursor: default;
}

.profile-row:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}

.row-check {
  display: grid;
  place-items: center;
  width: 16px;
  height: 16px;
  border: 1px solid var(--border-control);
  border-radius: 4.5px;
  background: var(--bg);
  color: transparent;
  font-size: 10px;
}

.row-check::after {
  content: "";
  width: 8px;
  height: 4px;
  border-bottom: 2px solid currentColor;
  border-left: 2px solid currentColor;
  transform: rotate(-45deg) translateY(-1px);
}

.selected .row-check {
  border-color: var(--accent);
  background: var(--accent);
  color: var(--text-on-accent);
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
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.row-tag {
  flex: 0 0 auto;
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xs);
  padding: 0 5px;
  color: var(--text-muted);
  font-size: 10px;
  font-style: normal;
  font-weight: 600;
}

.row-tag.warn {
  border-color: color-mix(in srgb, var(--warning) 40%, transparent);
  background: color-mix(in srgb, var(--warning) 10%, transparent);
  color: var(--warning-text);
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

.row-model {
  padding: 1px 5px;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--bg-raised);
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: 10.5px;
  white-space: nowrap;
}

.pane-hint {
  margin: 0;
  padding: 0 var(--space-4) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-2xs);
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

.side-label + .side-scroll {
  margin-top: var(--space-2);
}

.side-scroll {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  padding: 0 var(--space-4) 10px;
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

.macro-note {
  display: flex;
  gap: var(--space-2);
  margin: 0;
  border: 1px solid color-mix(in srgb, var(--warning) 30%, transparent);
  border-radius: var(--radius-sm);
  background: color-mix(in srgb, var(--warning) 8%, transparent);
  padding: 8px 10px;
  color: var(--warning-text);
  font-size: var(--text-2xs);
  line-height: 1.5;
}

.macro-note svg {
  flex: 0 0 auto;
  margin-top: 1px;
}
</style>
