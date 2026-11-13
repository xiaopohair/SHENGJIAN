<script setup lang="ts">
import { computed, nextTick, onUnmounted, ref, shallowRef, useTemplateRef, watch } from "vue";
import { Sparkles, X } from "lucide-vue-next";
import { errorMessage, getRecommendedButtonProfiles } from "../lib/api";
import { tr } from "../lib/i18n";
import type { RecommendedButtonProfile } from "../lib/types";
import Button from "./ui/Button.vue";
import IconButton from "./ui/IconButton.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    /** 当前设备型号；后端按它过滤兼容方案，null = 型号未知。 */
    model?: string | null;
    /** 复制/保存流程进行中，禁用每行的操作按钮。 */
    busy?: boolean;
  }>(),
  { model: null, busy: false },
);

const emit = defineEmits<{
  use: [profile: RecommendedButtonProfile];
  close: [];
  /** 列表加载完成，携带推荐包版本（父组件用来记录「已查看」）。 */
  loaded: [bundleVersion: string];
}>();

const profiles = ref<RecommendedButtonProfile[]>([]);
const bundleVersion = shallowRef("");
const loading = shallowRef(false);
const error = shallowRef<string | null>(null);
const dialogRef = useTemplateRef<HTMLElement>("dialogRef");
let previousFocus: HTMLElement | null = null;
let requestSeq = 0;

const versionLabel = computed(() =>
  bundleVersion.value ? tr("配置版本：{version}", { version: bundleVersion.value }) : "",
);

function modelsLabel(profile: RecommendedButtonProfile): string {
  return profile.compatibleModels.length === 0 ? tr("通用") : profile.compatibleModels.join(" / ");
}

async function load() {
  const sequence = ++requestSeq;
  loading.value = true;
  error.value = null;
  try {
    const bundle = await getRecommendedButtonProfiles(props.model ?? null);
    if (sequence !== requestSeq) return;
    profiles.value = Array.isArray(bundle.profiles) ? bundle.profiles : [];
    bundleVersion.value = bundle.bundleVersion ?? "";
    emit("loaded", bundleVersion.value);
  } catch (cause) {
    if (sequence !== requestSeq) return;
    profiles.value = [];
    bundleVersion.value = "";
    error.value = errorMessage(cause);
  } finally {
    if (sequence === requestSeq) loading.value = false;
  }
}

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
  if (!props.open) return;
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

watch(
  () => props.open,
  async (open) => {
    if (open) {
      previousFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      window.addEventListener("keydown", onKeydown, true);
      await nextTick();
      const initial =
        dialogRef.value?.querySelector<HTMLElement>(".recommended-use") ??
        dialogRef.value?.querySelector<HTMLElement>(".recommended-head button");
      initial?.focus();
      void load();
    } else {
      window.removeEventListener("keydown", onKeydown, true);
      requestSeq += 1;
      previousFocus?.focus();
      previousFocus = null;
    }
  },
  { immediate: true },
);

onUnmounted(() => {
  window.removeEventListener("keydown", onKeydown, true);
});
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="recommended-backdrop" @click.self="close">
      <section
        ref="dialogRef"
        class="recommended-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="recommended-title"
      >
        <header class="recommended-head">
          <div class="recommended-heading">
            <h2 id="recommended-title">
              <Sparkles :size="17" aria-hidden="true" />
              {{ tr("推荐方案") }}
            </h2>
            <p>{{ tr("由配置目录下发的只读方案；复制后成为你自己的方案，可自由修改。") }}</p>
          </div>
          <IconButton size="sm" :label="tr('关闭')" @click="close">
            <X :size="16" />
          </IconButton>
        </header>

        <div v-if="loading" class="recommended-status">{{ tr("加载推荐方案…") }}</div>
        <div v-else-if="error" class="recommended-status error" role="alert">
          <span>{{ tr("无法加载推荐方案") }}</span>
          <Button size="sm" @click="load">{{ tr("重试") }}</Button>
        </div>

        <ul v-else-if="profiles.length" class="recommended-list">
          <li v-for="profile in profiles" :key="profile.id" class="recommended-item">
            <div class="recommended-item-copy">
              <div class="recommended-item-title">
                <strong>{{ profile.name }}</strong>
                <span v-if="profile.autoApply" class="recommended-flag">
                  {{ tr("自动应用") }}
                </span>
              </div>
              <p class="recommended-meta">
                <span>{{ tr("适用型号：{models}", { models: modelsLabel(profile) }) }}</span>
                <span>{{ tr("{count} 个应用目标", { count: profile.appTargets.length }) }}</span>
                <span>{{ tr("{count} 个绑定", { count: profile.bindCount }) }}</span>
              </p>
            </div>
            <Button
              class="recommended-use"
              size="sm"
              variant="primary"
              :disabled="props.busy"
              @click="emit('use', profile)"
            >
              {{ tr("复制并使用") }}
            </Button>
          </li>
        </ul>
        <p v-else class="recommended-empty">{{ tr("当前型号暂无推荐方案") }}</p>

        <footer class="recommended-foot">
          <span v-if="versionLabel" class="recommended-foot-version">{{ versionLabel }}</span>
          <Button size="sm" variant="ghost" @click="close">{{ tr("关闭") }}</Button>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.recommended-backdrop {
  position: fixed;
  inset: 0;
  z-index: 300;
  display: grid;
  place-items: center;
  background: var(--overlay);
  padding: var(--space-5);
  backdrop-filter: blur(4px);
}

.recommended-dialog {
  display: flex;
  flex-direction: column;
  width: min(620px, calc(100vw - 48px));
  max-height: min(640px, calc(100vh - 48px));
  border: 1px solid var(--border-strong);
  border-radius: var(--radius-xl);
  background: var(--bg-raised);
  box-shadow: var(--shadow-lg);
}

.recommended-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: var(--space-3);
  border-bottom: 1px solid var(--border);
  padding: var(--space-4) var(--space-5);
}

.recommended-heading {
  min-width: 0;
}

.recommended-heading h2 {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0;
  font-size: var(--text-lg);
  font-weight: 650;
  letter-spacing: -0.02em;
}

.recommended-heading p {
  margin: 4px 0 0;
  color: var(--text-muted);
  font-size: var(--text-xs);
  line-height: 1.45;
}

.recommended-status {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-2);
  margin: var(--space-4) var(--space-5);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg-subtle);
  padding: var(--space-2) var(--space-3);
  color: var(--text-muted);
  font-size: var(--text-xs);
}

.recommended-status.error {
  border-color: var(--danger-soft);
  color: var(--danger-text);
}

.recommended-list {
  display: flex;
  flex: 1 1 auto;
  flex-direction: column;
  gap: var(--space-2);
  min-height: 0;
  overflow-y: auto;
  margin: 0;
  padding: var(--space-4) var(--space-5);
  list-style: none;
}

.recommended-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--bg);
  padding: var(--space-3) var(--space-4);
}

.recommended-item-copy {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.recommended-item-title {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-2);
}

.recommended-item-title strong {
  font-size: var(--text-sm);
  font-weight: 650;
}

.recommended-flag {
  border: 1px solid var(--accent);
  border-radius: var(--radius-pill);
  background: var(--accent-soft);
  padding: 1px 7px;
  color: var(--accent-ink);
  font-size: var(--text-2xs);
  font-weight: 650;
  white-space: nowrap;
}

.recommended-meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  margin: 0;
  color: var(--text-muted);
  font-size: var(--text-2xs);
  line-height: 1.45;
}

.recommended-meta span {
  white-space: nowrap;
}

.recommended-use {
  flex: 0 0 auto;
}

.recommended-empty {
  margin: 0;
  padding: var(--space-6) var(--space-5);
  color: var(--text-muted);
  font-size: var(--text-sm);
  text-align: center;
}

.recommended-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: flex-end;
  gap: var(--space-2);
  border-top: 1px solid var(--border);
  padding: var(--space-3) var(--space-5);
}

.recommended-foot-version {
  margin-right: auto;
  color: var(--text-muted);
  font-family: var(--font-mono);
  font-size: var(--text-2xs);
}
</style>
