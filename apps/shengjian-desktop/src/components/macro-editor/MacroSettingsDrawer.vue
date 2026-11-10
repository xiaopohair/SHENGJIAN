<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from "vue";
import { X } from "lucide-vue-next";
import { tr } from "../../lib/i18n";
import Button from "../ui/Button.vue";
import TextInput from "../ui/TextInput.vue";

const props = defineProps<{
  open: boolean;
}>();

const emit = defineEmits<{
  close: [];
}>();

const name = defineModel<string>("name", { required: true });
const summary = defineModel<string>("summary", { required: true });

const drawerRef = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;

function close() {
  emit("close");
}

function onKeydown(event: KeyboardEvent) {
  if (!props.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    close();
  }
}

watch(
  () => props.open,
  async (openState) => {
    if (openState) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      await nextTick();
      drawerRef.value?.querySelector<HTMLElement>(".macro-settings-body input")?.focus();
    } else {
      previousFocus?.focus();
      previousFocus = null;
    }
  },
);

onMounted(() => window.addEventListener("keydown", onKeydown, true));
onUnmounted(() => window.removeEventListener("keydown", onKeydown, true));
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="macro-settings-backdrop" @click.self="close">
      <aside
        ref="drawerRef"
        class="macro-settings-drawer"
        role="dialog"
        aria-modal="true"
        :aria-label="tr('宏属性')"
      >
        <header class="macro-settings-head">
          <div>
            <span class="eyebrow">{{ tr("属性") }}</span>
            <h2>{{ tr("宏属性") }}</h2>
            <p>{{ tr("名称与说明随宏一起保存。") }}</p>
          </div>
          <button
            type="button"
            class="macro-settings-close"
            :aria-label="tr('关闭')"
            @click="close"
          >
            <X :size="18" />
          </button>
        </header>

        <div class="macro-settings-body">
          <label class="macro-field">
            <span class="macro-field-label">{{ tr("名称") }}</span>
            <TextInput v-model="name" :maxlength="20" :placeholder="tr('未命名宏')" />
          </label>
          <label class="macro-field">
            <span class="macro-field-label">{{ tr("说明") }}</span>
            <TextInput v-model="summary" :maxlength="300" :placeholder="tr('说明')" />
          </label>
        </div>

        <footer class="macro-settings-foot">
          <Button variant="primary" @click="close">{{ tr("完成") }}</Button>
        </footer>
      </aside>
    </div>
  </Teleport>
</template>

<style scoped>
.macro-settings-backdrop {
  position: fixed;
  inset: 0;
  z-index: 240;
  display: flex;
  justify-content: flex-end;
  background: var(--overlay);
  backdrop-filter: blur(4px);
}

.macro-settings-drawer {
  display: flex;
  flex-direction: column;
  width: min(440px, calc(100vw - 24px));
  height: 100%;
  border-left: 1px solid var(--border-strong);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  animation: drawer-in var(--duration-base) var(--ease-out);
}

@keyframes drawer-in {
  from {
    transform: translateX(14px);
    opacity: 0.6;
  }
  to {
    transform: translateX(0);
    opacity: 1;
  }
}

.macro-settings-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-5);
}

.eyebrow {
  color: var(--text-muted);
  font-size: var(--text-2xs);
  font-weight: 700;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.macro-settings-head h2 {
  margin: 4px 0 0;
  font-size: var(--text-xl);
  letter-spacing: -0.03em;
}

.macro-settings-head p {
  margin: 5px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.macro-settings-close {
  display: grid;
  place-items: center;
  width: 30px;
  height: 30px;
  border: 1px solid transparent;
  border-radius: var(--radius-md);
  background: transparent;
  color: var(--text-muted);
  cursor: pointer;
}
.macro-settings-close:hover {
  border-color: var(--border);
  color: var(--text);
}

.macro-settings-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: var(--space-4);
  overflow-y: auto;
  padding: var(--space-5);
}

/* Label above input, full width: the drawer is narrow. */
.macro-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
}

.macro-field-label {
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.4;
}

.macro-settings-foot {
  display: flex;
  justify-content: flex-end;
  border-top: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}
</style>
