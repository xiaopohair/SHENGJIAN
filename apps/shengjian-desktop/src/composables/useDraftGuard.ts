import { computed, onMounted, onUnmounted, type Ref } from "vue";
import { confirmAction } from "../lib/confirm";
import { registerLeaveGuard } from "../lib/navigation";
import { tr, trDynamic } from "../lib/i18n";

interface DraftGuardOptions<T> {
  draft: Ref<T | null>;
  baseline: Ref<string>;
  discardMessage: string;
}

export function useDraftGuard<T>(options: DraftGuardOptions<T>) {
  const dirty = computed(
    () =>
      Boolean(options.draft.value) &&
      JSON.stringify(options.draft.value) !== options.baseline.value,
  );

  async function confirmDiscard(): Promise<boolean> {
    if (!options.draft.value || !dirty.value) return true;
    return confirmAction({
      title: tr("放弃未保存的修改？"),
      message: trDynamic(options.discardMessage),
      confirmLabel: tr("放弃修改"),
      danger: true,
    });
  }

  let unregister: (() => void) | undefined;
  onMounted(() => {
    unregister = registerLeaveGuard(confirmDiscard);
  });
  onUnmounted(() => {
    unregister?.();
  });

  return { dirty, confirmDiscard };
}
