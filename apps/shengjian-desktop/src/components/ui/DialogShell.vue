<script setup lang="ts">
/**
 * 对话框外壳：遮罩、卡片、标题区、关闭按钮，以及焦点圈定 / Esc 关闭。
 *
 * 仓库里所有"大弹窗"都走这一份样式，避免每个弹窗各写一套。内容与底部操作
 * 通过默认插槽和 footer 插槽传入；各弹窗自己负责内容区的内边距。
 */
import { nextTick, onMounted, onUnmounted, useTemplateRef } from "vue";
import { X } from "lucide-vue-next";
import { trDynamic } from "../../lib/i18n";

const props = withDefaults(
  defineProps<{
    title: string;
    titleId?: string;
    eyebrow?: string;
    subtitle?: string;
    /** 卡片最大宽度，默认与配对指南一致。 */
    width?: string;
    /** 固定内容区高度：切换标签页时弹窗不会跟着跳。 */
    contentHeight?: string;
  }>(),
  { titleId: "", eyebrow: "", subtitle: "", width: "min(960px, 100%)", contentHeight: "" },
);

const emit = defineEmits<{ close: [] }>();

const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
const closeButton = useTemplateRef<HTMLElement>("closeButton");
let previousFocus: HTMLElement | null = null;

function close() {
  emit("close");
}

function focusableElements(): HTMLElement[] {
  if (!dialogRef.value) return [];
  return Array.from(
    dialogRef.value.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((element) => !element.hasAttribute("disabled") && element.offsetParent !== null);
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape") {
    event.preventDefault();
    close();
    return;
  }
  if (event.key !== "Tab") return;
  const elements = focusableElements();
  if (elements.length === 0) return;
  const first = elements[0];
  const last = elements[elements.length - 1];
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}

onMounted(async () => {
  previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  window.addEventListener("keydown", onKeydown, true);
  await nextTick();
  closeButton.value?.focus();
});

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
  previousFocus?.focus();
  previousFocus = null;
});
</script>

<template>
  <Teleport to="body">
    <div class="dialog-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="dialog"
        :style="{ width: props.width }"
        role="dialog"
        aria-modal="true"
        :aria-labelledby="titleId || undefined"
      >
        <header class="dialog-head">
          <div class="dialog-heading">
            <p v-if="eyebrow" class="dialog-eyebrow">{{ eyebrow }}</p>
            <h2 :id="titleId || undefined">{{ title }}</h2>
            <p v-if="subtitle" class="dialog-lead">{{ subtitle }}</p>
          </div>
          <button
            ref="closeButton"
            type="button"
            class="dialog-close"
            :aria-label="trDynamic('关闭')"
            @click="close"
          >
            <X :size="18" />
          </button>
        </header>

        <div class="dialog-content" :style="contentHeight ? { height: contentHeight } : undefined">
          <slot />
        </div>

        <footer v-if="$slots.footer" class="dialog-foot">
          <slot name="footer" />
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.dialog-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  overflow-y: auto;
  background: var(--overlay);
  padding: var(--space-3);
  backdrop-filter: blur(6px);
}

.dialog {
  display: flex;
  flex-direction: column;
  min-height: 0;
  max-width: 100%;
  max-height: calc(100vh - 2 * var(--space-3));
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}

.dialog-head {
  display: flex;
  flex: 0 0 auto;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-4);
  padding: var(--space-5) var(--space-6) var(--space-3);
}

.dialog-eyebrow {
  display: flex;
  align-items: center;
  gap: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-xs);
  font-weight: 650;
  letter-spacing: 0.04em;
  text-transform: uppercase;
}

.dialog-heading h2 {
  margin-top: var(--space-2);
  font-size: var(--text-2xl);
  font-weight: 700;
  letter-spacing: -0.02em;
  line-height: 1.2;
}

.dialog-lead {
  margin-top: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-sm);
  line-height: 1.5;
}

.dialog-close {
  display: grid;
  flex: 0 0 32px;
  place-items: center;
  width: 32px;
  height: 32px;
  border: 0;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}

.dialog-close:hover {
  background: var(--bg-hover);
  color: var(--text);
}

.dialog-close:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

.dialog-content {
  display: flex;
  flex-direction: column;
  min-height: 0;
  overflow-y: auto;
}

.dialog-foot {
  display: flex;
  flex: 0 0 auto;
  align-items: center;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-6);
}

@media (max-width: 720px) {
  .dialog-head {
    padding: var(--space-4) var(--space-4) var(--space-2);
  }

  .dialog-foot {
    padding: var(--space-3) var(--space-4);
  }
}
</style>
