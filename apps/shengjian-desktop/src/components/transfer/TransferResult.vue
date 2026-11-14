<script setup lang="ts">
import { computed } from "vue";
import { Check, TriangleAlert, Undo2 } from "lucide-vue-next";
import { tr } from "../../lib/i18n";
import type { TransferResultState } from "../../lib/profileTransfer";
import Button from "../ui/Button.vue";

const props = withDefaults(
  defineProps<{
    state: TransferResultState;
    /** 导入完成后本机还留着导入前的快照时，可以撤销。 */
    canUndo?: boolean;
  }>(),
  { canUndo: false },
);

const emit = defineEmits<{ finish: []; undo: []; retry: [] }>();

const isError = computed(() => props.state.kind === "error");

const importChips = computed(() => {
  if (props.state.kind !== "import") return [];
  const counts = props.state.counts;
  const chips: Array<{ key: string; label: string; cls: string }> = [];
  if (counts.add)
    chips.push({ key: "add", label: tr("新增 {count}", { count: counts.add }), cls: "chip-new" });
  if (counts.overwrite)
    chips.push({
      key: "overwrite",
      label: tr("更新 {count}", { count: counts.overwrite }),
      cls: "chip-update",
    });
  if (counts.keep)
    chips.push({
      key: "keep",
      label: tr("保留 {count}", { count: counts.keep }),
      cls: "chip-keep",
    });
  if (counts.skip)
    chips.push({
      key: "skip",
      label: tr("跳过 {count}", { count: counts.skip }),
      cls: "chip-skip",
    });
  return chips;
});

const importTotal = computed(() => {
  if (props.state.kind !== "import") return 0;
  const counts = props.state.counts;
  return counts.add + counts.overwrite + counts.keep;
});

const importSummary = computed(() => {
  if (props.state.kind !== "import") return "";
  const counts = props.state.counts;
  const parts: string[] = [];
  if (counts.overwrite) parts.push(tr("替换 {count} 个方案", { count: counts.overwrite }));
  if (counts.keep) parts.push(tr("保留 {count} 个副本", { count: counts.keep }));
  if (counts.skip) parts.push(tr("跳过 {count} 个", { count: counts.skip }));
  return parts.length ? `${parts.join(" · ")}。` : "";
});
</script>

<template>
  <div class="transfer-result" :class="isError ? 'is-error' : 'is-done'">
    <span class="result-badge" aria-hidden="true">
      <TriangleAlert v-if="isError" :size="24" :stroke-width="1.9" />
      <Check v-else :size="24" :stroke-width="2.2" />
    </span>

    <template v-if="state.kind === 'error'">
      <h3 class="result-title">{{ state.title ?? tr("这个文件读不了") }}</h3>
      <p class="result-copy">{{ state.message }}</p>
      <div class="result-actions">
        <Button size="sm" @click="emit('retry')">{{ tr("换个文件") }}</Button>
        <Button size="sm" variant="primary" @click="emit('finish')">{{ tr("知道了") }}</Button>
      </div>
    </template>

    <template v-else-if="state.kind === 'export'">
      <h3 class="result-title">{{ tr("已导出 {count} 个方案", { count: state.count }) }}</h3>
      <p class="result-copy">
        {{ tr("{binds} 个键位", { binds: state.binds }) }}
        <template v-if="state.macros">
          · {{ tr("含 {count} 个宏", { count: state.macros }) }}
        </template>
        · {{ state.size }}
      </p>
      <p class="result-path" :title="state.path">{{ state.path }}</p>
      <div class="result-actions">
        <Button size="sm" variant="primary" @click="emit('finish')">{{ tr("完成") }}</Button>
      </div>
    </template>

    <template v-else>
      <h3 class="result-title">{{ tr("已导入 {count} 个方案", { count: importTotal }) }}</h3>
      <p class="result-copy">
        {{ importSummary }} {{ tr("被跳过的方案还在文件里，可以重新导入。") }}
      </p>
      <div v-if="importChips.length" class="result-chips">
        <span v-for="chip in importChips" :key="chip.key" class="result-chip" :class="chip.cls">
          {{ chip.label }}
        </span>
      </div>
      <div class="result-actions">
        <Button v-if="canUndo" size="sm" @click="emit('undo')">
          <Undo2 :size="14" aria-hidden="true" />
          {{ tr("撤销导入") }}
        </Button>
        <Button size="sm" variant="primary" @click="emit('finish')">{{ tr("完成") }}</Button>
      </div>
    </template>
  </div>
</template>

<style scoped>
.transfer-result {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 5px;
  min-height: 404px;
  padding: var(--space-6) var(--space-5);
  text-align: center;
}

.result-badge {
  display: grid;
  place-items: center;
  width: 50px;
  height: 50px;
  margin-bottom: 10px;
  border-radius: 50%;
}

.is-done .result-badge {
  border: 1px solid color-mix(in srgb, var(--accent) 34%, transparent);
  background: var(--accent-soft);
  color: var(--accent);
}

.is-error .result-badge {
  border: 1px solid color-mix(in srgb, var(--danger) 34%, transparent);
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  color: var(--danger);
}

.result-title {
  margin: 0;
  color: var(--text);
  font-size: var(--text-lg);
  font-weight: 650;
}

.result-copy {
  max-width: 430px;
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.6;
}

.result-path {
  max-width: 100%;
  margin: 10px 0 0;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-subtle);
  padding: 5px 10px;
  color: var(--text-secondary);
  font-family: var(--font-mono);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.result-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  justify-content: center;
  margin-top: 12px;
}

.result-chip {
  border: 1px solid transparent;
  border-radius: var(--radius-xs);
  padding: 1px 7px;
  font-size: 11px;
  font-weight: 600;
}

.chip-new {
  border-color: color-mix(in srgb, var(--accent) 32%, transparent);
  background: var(--accent-soft);
  color: var(--accent);
}

.chip-update {
  border-color: color-mix(in srgb, var(--info) 30%, transparent);
  background: color-mix(in srgb, var(--info) 12%, transparent);
  color: var(--info-text);
}

.chip-keep {
  border-color: color-mix(in srgb, var(--warning) 32%, transparent);
  background: color-mix(in srgb, var(--warning) 10%, transparent);
  color: var(--warning-text);
}

.chip-skip {
  border-color: var(--border);
  background: var(--bg-hover);
  color: var(--text-muted);
}

.result-actions {
  display: flex;
  gap: var(--space-2);
  margin-top: 20px;
}
</style>
