<script setup lang="ts">
import { nextTick, onUnmounted, ref, watch } from "vue";
import { TriangleAlert } from "lucide-vue-next";
import { confirmState, settleConfirm } from "../../lib/confirm";
import { trDynamic } from "../../lib/i18n";
import Button from "./Button.vue";

const cancelButton = ref<InstanceType<typeof Button> | null>(null);
const dialogRef = ref<HTMLElement | null>(null);
let previousFocus: HTMLElement | null = null;

function focusableElements(): HTMLElement[] {
  if (!dialogRef.value) return [];
  return Array.from(
    dialogRef.value.querySelectorAll<HTMLElement>(
      'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])',
    ),
  ).filter((element) => !element.hasAttribute("disabled") && element.offsetParent !== null);
}

function cancel() {
  settleConfirm(false);
}

function accept() {
  settleConfirm(true);
}

function onKeydown(event: KeyboardEvent) {
  if (!confirmState.open) return;
  if (event.key === "Escape") {
    event.preventDefault();
    cancel();
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

watch(
  () => confirmState.open,
  async (open) => {
    if (open) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      window.addEventListener("keydown", onKeydown, true);
      await nextTick();
      (cancelButton.value?.$el as HTMLElement | undefined)?.focus();
    } else {
      window.removeEventListener("keydown", onKeydown, true);
      previousFocus?.focus();
      previousFocus = null;
    }
  },
);

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="confirmState.open" class="confirm-backdrop" @click.self="cancel">
      <section
        ref="dialogRef"
        class="confirm-dialog"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
        :aria-describedby="confirmState.message ? 'confirm-message' : undefined"
      >
        <div class="confirm-head">
          <span v-if="confirmState.danger" class="confirm-icon">
            <TriangleAlert :size="18" />
          </span>
          <h2 id="confirm-title">{{ trDynamic(confirmState.title) }}</h2>
        </div>
        <p v-if="confirmState.message" id="confirm-message" class="confirm-message">
          {{ trDynamic(confirmState.message) }}
        </p>
        <p v-if="confirmState.detail" class="confirm-detail">
          {{ trDynamic(confirmState.detail) }}
        </p>
        <div class="confirm-actions">
          <Button ref="cancelButton" variant="ghost" @click="cancel">
            {{ trDynamic(confirmState.cancelLabel || "取消") }}
          </Button>
          <Button :variant="confirmState.danger ? 'danger-solid' : 'primary'" @click="accept">
            {{ trDynamic(confirmState.confirmLabel || "确认") }}
          </Button>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-backdrop {
  position: fixed;
  inset: 0;
  z-index: 120;
  display: grid;
  place-items: center;
  background: var(--overlay);
  padding: var(--space-5);
  backdrop-filter: blur(4px);
}

.confirm-dialog {
  width: min(440px, 100%);
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
  padding: var(--space-5);
}

.confirm-head {
  display: flex;
  align-items: flex-start;
  gap: var(--space-3);
}

.confirm-icon {
  display: grid;
  flex: 0 0 34px;
  place-items: center;
  width: 34px;
  height: 34px;
  border-radius: var(--radius-md);
  background: var(--danger-soft);
  color: var(--danger);
}

h2 {
  padding-top: 4px;
  font-size: var(--text-lg);
  font-weight: 650;
}

.confirm-message {
  margin-top: var(--space-3);
  color: var(--text-secondary);
  font-size: var(--text-sm);
  line-height: 1.6;
  overflow-wrap: anywhere;
}

.confirm-detail {
  margin-top: var(--space-2);
  color: var(--text-muted);
  font-size: var(--text-xs);
  overflow-wrap: anywhere;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-2);
  margin-top: var(--space-5);
}
</style>
