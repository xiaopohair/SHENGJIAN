<script setup lang="ts">
import { computed, watch } from "vue";
import { appTargetFriendlyName, describeAppTarget } from "../../composables/useAppPicker";
import { tr } from "../../lib/i18n";
import Button from "../ui/Button.vue";
import TextInput from "../ui/TextInput.vue";

/**
 * 只读的应用目标输入框 + 「选择应用」按钮：按键方案与宏编辑器共用。
 *
 * 输入框显示友好名（解析中/失败时回退原始 target），不可直接编辑——点输入框或
 * 按钮都打开应用选择器；手动填写走选择器里的「手动填写（高级）」。
 */
const props = withDefaults(
  defineProps<{
    target: string;
    inputId?: string;
    placeholder?: string;
    chooseLabel?: string;
    ariaLabel?: string;
    disabled?: boolean;
  }>(),
  {
    inputId: "",
    placeholder: "",
    chooseLabel: "",
    ariaLabel: "",
    disabled: false,
  },
);

const emit = defineEmits<{
  choose: [];
}>();

const label = computed(() => appTargetFriendlyName(props.target, props.target));
const raw = computed(() => (props.target ?? "").trim());

// 触发器画面板里的描述缓存：拿到 friendly name 后文本框会自动更新。
watch(
  () => props.target,
  (target) => {
    if (target?.trim()) void describeAppTarget(target.trim());
  },
  { immediate: true },
);
</script>

<template>
  <div class="app-target-field">
    <TextInput
      class="app-target-field-input"
      readonly
      :input-id="inputId || undefined"
      :model-value="label"
      :title="raw || undefined"
      :aria-label="ariaLabel || tr('应用目标')"
      :placeholder="placeholder || tr('未设置应用目标')"
      @click="emit('choose')"
    />
    <Button size="sm" :disabled="disabled" @click="emit('choose')">
      {{ chooseLabel || tr("选择应用") }}
    </Button>
  </div>
</template>

<style scoped>
.app-target-field {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  min-width: 0;
}

.app-target-field-input {
  flex: 1 1 160px;
  min-width: 0;
  cursor: pointer;
}

.app-target-field-input:hover {
  border-color: var(--text-disabled);
}
</style>
