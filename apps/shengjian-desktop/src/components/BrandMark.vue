<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import markOnDark from "../assets/brand/shengjian-mark.svg";
import markOnLight from "../assets/brand/shengjian-mark-on-light.svg";
import appIcon from "../assets/app-icon.png";

const props = withDefaults(defineProps<{ height?: number; variant?: "mark" | "app" }>(), {
  height: 24,
  variant: "mark",
});

const theme = ref<"light" | "dark">("dark");
/** The app icon carries its own dark plate, so it reads the same in both themes. */
const src = computed(() => {
  if (props.variant === "app") return appIcon;
  return theme.value === "light" ? markOnLight : markOnDark;
});
let observer: MutationObserver | undefined;

function readTheme() {
  theme.value = document.documentElement.dataset.theme === "light" ? "light" : "dark";
}

onMounted(() => {
  readTheme();
  observer = new MutationObserver(readTheme);
  observer.observe(document.documentElement, { attributes: true, attributeFilter: ["data-theme"] });
});

onUnmounted(() => observer?.disconnect());
</script>

<template>
  <span class="brand-mark" :style="{ height: `${height}px` }" aria-hidden="true">
    <img :src="src" alt="" />
  </span>
</template>

<style scoped>
.brand-mark {
  display: inline-flex;
  flex: 0 0 auto;
  align-items: center;
}

.brand-mark img {
  display: block;
  width: auto;
  height: 100%;
}
</style>
