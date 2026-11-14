<script setup lang="ts">
import { computed } from "vue";
import { tr } from "../../lib/i18n";
import { deviceLineArt } from "../../lib/deviceArt";
import { profileStats } from "../../lib/profileTransfer";
import { PROFILE_BUTTONS } from "../../lib/buttons";
import type { ButtonProfile, LogicalAction } from "../../lib/types";

const props = defineProps<{ profile: ButtonProfile }>();

const stats = computed(() => profileStats(props.profile));
const modelLabel = computed(() => props.profile.deviceModel ?? tr("通用"));

function describeAction(action: LogicalAction): string {
  const value = action as Record<string, unknown>;
  switch (action.type) {
    case "disabled":
      return tr("已禁用");
    case "focus-input":
      return tr("聚焦输入框");
    case "shortcut": {
      const label = typeof value.label === "string" ? value.label : "";
      const keys = Array.isArray(value.keys) ? value.keys.join(" + ") : "";
      return label || keys;
    }
    case "open-app":
      return tr("打开应用");
    case "system-action":
      return tr("系统动作");
    case "common-phrase":
      return tr("常用语");
    case "mouse":
      return tr("鼠标操作");
    case "run-macro":
      return tr("运行宏");
    default:
      return String(action.type);
  }
}

const bindingRows = computed(() => {
  const rows: Array<{ id: string; label: string; action: string }> = [];
  for (const button of PROFILE_BUTTONS) {
    const byTrigger = (props.profile.bindings ?? {})[button.id];
    if (!byTrigger) continue;
    const action = byTrigger.single ?? byTrigger.double ?? byTrigger.long;
    if (!action) continue;
    rows.push({ id: button.id, label: button.label, action: describeAction(action) });
  }
  return rows;
});

const visibleRows = computed(() => bindingRows.value.slice(0, 3));
const restCount = computed(() => Math.max(0, stats.value.binds - visibleRows.value.length));
</script>

<template>
  <div class="remote-card">
    <span class="device-art">
      <img
        class="device-image"
        :src="deviceLineArt(profile.deviceModel)"
        :alt="modelLabel"
        draggable="false"
      />
    </span>

    <div class="remote-meta">
      <strong class="remote-name">{{ profile.name }}</strong>
      <p class="remote-sub">
        {{ modelLabel }} · {{ stats.binds }} {{ tr("键位") }} · {{ stats.appCount }}
        {{ tr("应用") }}
      </p>
      <ul v-if="visibleRows.length" class="binding-list">
        <li v-for="row in visibleRows" :key="row.id">
          <span class="binding-key">{{ row.label }}</span>
          <span class="binding-action">{{ row.action }}</span>
        </li>
      </ul>
      <p v-if="restCount > 0" class="binding-more">
        {{ tr("还有 {count} 个键位", { count: restCount }) }}
      </p>
    </div>
  </div>
</template>

<style scoped>
.remote-card {
  display: flex;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-raised);
  padding: var(--space-3);
}

.device-art {
  display: grid;
  flex: 0 0 82px;
  width: 82px;
  height: 130px;
  place-items: center;
}

.device-image {
  width: auto;
  height: 126px;
  max-width: 100%;
  object-fit: contain;
  /* 线稿资源是深色的，深浅主题靠连接页同一套反相令牌切换。 */
  filter: invert(var(--line-art-invert)) drop-shadow(var(--shadow-sm));
  user-select: none;
}

.remote-meta {
  min-width: 0;
  flex: 1;
}

.remote-name {
  display: block;
  overflow: hidden;
  color: var(--text);
  font-size: var(--text-sm);
  font-weight: 650;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.remote-sub {
  margin: 2px 0 8px;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-variant-numeric: tabular-nums;
}

.binding-list {
  display: flex;
  flex-direction: column;
  gap: 3px;
  margin: 0;
  padding: 0;
  list-style: none;
}

.binding-list li {
  display: flex;
  justify-content: space-between;
  gap: var(--space-2);
  font-size: var(--text-2xs);
  color: var(--text-secondary);
}

.binding-key {
  flex: 0 0 auto;
  color: var(--text-muted);
}

.binding-action {
  overflow: hidden;
  text-align: right;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.binding-more {
  margin: 5px 0 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
}
</style>
