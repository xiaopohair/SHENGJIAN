import { nextTick, onUnmounted, toValue, watch, type MaybeRefOrGetter, type Ref } from "vue";

const FOCUSABLE_SELECTOR = [
  "a[href]",
  "button:not([disabled])",
  "textarea:not([disabled])",
  "input:not([disabled])",
  "select:not([disabled])",
  '[tabindex]:not([tabindex="-1"])',
].join(",");

export interface ModalFocusOptions {
  /** 对话框根元素；同时用作 Tab 圈定与默认聚焦目标。 */
  container: Ref<HTMLElement | null>;
  /** 是否打开；组件常驻、靠 v-if 控制时传 getter 即可。 */
  open: MaybeRefOrGetter<boolean>;
  /** Esc 或点击遮罩时的关闭动作。 */
  onClose: () => void;
  /** 打开时优先聚焦的元素；缺省聚焦容器内第一个可聚焦元素。 */
  initialFocus?: () => HTMLElement | null | undefined;
  /** 额外按键（如 Enter 确认）；先在打开时执行，处理后可 preventDefault 跳过内置逻辑。 */
  onKey?: (event: KeyboardEvent) => void;
}

/**
 * 模态对话框的焦点与键盘约定：Esc 关闭、Tab 在容器内循环、打开时聚焦、
 * 关闭/卸载时还给之前的焦点。各对话框只保留自己的业务按键。
 */
export function useModalFocus(options: ModalFocusOptions) {
  let previousFocus: HTMLElement | null = null;
  let listening = false;

  function focusableElements(): HTMLElement[] {
    const root = options.container.value;
    if (!root) return [];
    return [...root.querySelectorAll<HTMLElement>(FOCUSABLE_SELECTOR)].filter(
      (element) =>
        !element.hasAttribute("disabled") &&
        element.getAttribute("aria-hidden") !== "true" &&
        element.getClientRects().length > 0,
    );
  }

  function focusInitial() {
    const target = options.initialFocus?.() ?? focusableElements()[0] ?? options.container.value;
    target?.focus();
  }

  function onKeydown(event: KeyboardEvent) {
    if (!toValue(options.open)) return;
    options.onKey?.(event);
    if (event.defaultPrevented) return;
    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      options.onClose();
      return;
    }
    if (event.key !== "Tab") return;
    const items = focusableElements();
    if (items.length === 0) {
      event.preventDefault();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement;
    const outside = !options.container.value?.contains(active);
    if (event.shiftKey) {
      if (active === first || outside) {
        event.preventDefault();
        last.focus();
      }
    } else if (active === last || outside) {
      event.preventDefault();
      first.focus();
    }
  }

  function attach() {
    if (listening) return;
    window.addEventListener("keydown", onKeydown, true);
    listening = true;
  }

  function detach() {
    if (!listening) return;
    window.removeEventListener("keydown", onKeydown, true);
    listening = false;
  }

  watch(
    () => toValue(options.open),
    async (open) => {
      if (open) {
        previousFocus =
          document.activeElement instanceof HTMLElement ? document.activeElement : null;
        attach();
        await nextTick();
        focusInitial();
      } else {
        detach();
        previousFocus?.focus();
        previousFocus = null;
      }
    },
    { immediate: true },
  );

  onUnmounted(() => {
    detach();
    if (toValue(options.open)) {
      previousFocus?.focus();
      previousFocus = null;
    }
  });

  return { focusableElements, focusInitial };
}
