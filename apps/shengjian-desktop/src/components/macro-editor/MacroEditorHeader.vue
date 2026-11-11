<script setup lang="ts">
import { CircleAlert, Copy, Play, SlidersHorizontal, Square, Trash2 } from "lucide-vue-next";
import { tr, trDynamic } from "../../lib/i18n";
import type { MacroRunStatus } from "../../lib/types";
import Button from "../../components/ui/Button.vue";
import IconButton from "../../components/ui/IconButton.vue";
import StatusPill from "../../components/ui/StatusPill.vue";

const props = defineProps<{
  name: string;
  summary: string;
  editingNew: boolean;
  busy: boolean;
  dirty: boolean;
  runStatus?: MacroRunStatus | null;
  draftTesting?: boolean;
  saveError?: string | null;
  canTest: boolean;
}>();

const emit = defineEmits<{
  "update:name": [name: string];
  "update:summary": [summary: string];
  save: [];
  cancel: [];
  "test-draft": [];
  "cancel-draft": [];
  "duplicate-macro": [];
  "delete-macro": [];
  "open-settings": [];
}>();

function startOrCancelRun() {
  if (props.runStatus?.running) {
    emit("cancel-draft");
    return;
  }
  emit("test-draft");
}
</script>

<template>
  <header class="editor-head">
    <div class="editor-identity">
      <div class="name-row">
        <span class="eyebrow">{{ editingNew ? tr("新建宏：") : tr("编辑宏：") }}</span>
        <input
          class="name-input"
          maxlength="20"
          :value="name"
          :placeholder="trDynamic('未命名宏')"
          :aria-label="tr('名称')"
          @input="emit('update:name', ($event.target as HTMLInputElement).value)"
        />
        <StatusPill v-if="dirty" :label="trDynamic('未保存')" tone="warning" dot />
      </div>
    </div>
    <div class="editor-actions-wrap">
      <div class="editor-actions">
        <Button size="sm" variant="ghost" :disabled="busy" @click="emit('cancel')">{{
          tr("取消")
        }}</Button>
        <Button
          v-if="runStatus?.running"
          size="sm"
          variant="danger"
          :disabled="busy"
          @click="startOrCancelRun"
        >
          <Square :size="13" />
          {{ tr("停止") }}
        </Button>
        <Button
          v-else
          size="sm"
          variant="secondary"
          :disabled="!canTest || busy"
          @click="startOrCancelRun"
        >
          <Play :size="14" />
          {{ draftTesting ? tr("启动中…") : tr("试运行") }}
        </Button>
        <Button size="sm" variant="primary" :loading="busy" @click="emit('save')">
          {{ tr("保存") }}
        </Button>
        <IconButton
          size="sm"
          :label="trDynamic('属性')"
          :disabled="busy"
          @click="emit('open-settings')"
        >
          <SlidersHorizontal :size="14" />
        </IconButton>
        <template v-if="!editingNew">
          <IconButton
            size="sm"
            :label="trDynamic('复制宏')"
            :disabled="busy"
            @click="emit('duplicate-macro')"
          >
            <Copy :size="14" />
          </IconButton>
          <IconButton
            size="sm"
            variant="danger"
            :label="trDynamic('删除宏')"
            :disabled="busy"
            @click="emit('delete-macro')"
          >
            <Trash2 :size="14" />
          </IconButton>
        </template>
      </div>
      <p v-if="saveError" class="save-error" role="alert">
        <CircleAlert :size="13" />
        <span>{{ saveError }}</span>
      </p>
    </div>
  </header>
</template>

<style scoped>
.editor-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  /* Match the buttons page title row: a 1px rule under the editor title. */
  padding: 0 2px 8px;
  border-bottom: 1px solid var(--border);
}
.editor-identity {
  flex: 1;
  min-width: 0;
}
.editor-actions-wrap {
  display: flex;
  flex: 0 0 auto;
  flex-direction: column;
  /* Cap the column at its intrinsic width; otherwise it stretches and starves the title. */
  width: max-content;
  max-width: 100%;
  align-items: flex-end;
  gap: 6px;
  min-width: 0;
}
.save-error {
  display: flex;
  align-items: flex-start;
  justify-content: flex-end;
  gap: 4px;
  max-width: 420px;
  margin: 0;
  color: var(--danger-text);
  font-size: 12px;
  line-height: 1.5;
  text-align: left;
}
.save-error > svg {
  flex: 0 0 auto;
  margin-top: 3px;
}
.name-row {
  display: flex;
  align-items: center;
  gap: 8px;
  min-width: 0;
}
.eyebrow {
  flex: 0 0 auto;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}
.name-input {
  flex: 1 1 200px;
  width: auto;
  max-width: 520px;
  min-width: 0;
  border: 0;
  outline: 0;
  background: transparent;
  padding: 0;
  color: var(--text);
  font-size: 17px;
  font-weight: 600;
  line-height: 1.3;
  letter-spacing: -0.01em;
}
.name-input:focus {
  border-bottom: 1px solid var(--accent);
}
.name-row > .ui-status {
  flex: 0 0 auto;
}
.editor-actions {
  display: flex;
  flex: 0 0 auto;
  /* Keep the whole action row on one line at the minimum window width. */
  width: max-content;
  max-width: 100%;
  flex-wrap: nowrap;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  /* The global .editor-actions is a sticky bottom bar; this row sits inline. */
  position: static;
  z-index: auto;
  border-top: 0;
  background: transparent;
  padding: 0;
}
/* Only below the supported minimum width: stack the header and let actions fill. */
@media (max-width: 1100px) {
  .editor-head {
    flex-direction: column;
  }
  .editor-actions-wrap {
    width: 100%;
    align-items: stretch;
  }
  .editor-actions {
    width: 100%;
  }
  .save-error {
    max-width: none;
    text-align: left;
  }
}
</style>
