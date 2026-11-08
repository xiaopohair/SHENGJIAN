<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import markOnDark from "../assets/brand/shengjian-mark.svg";
import markOnLight from "../assets/brand/shengjian-mark-on-light.svg";

withDefaults(defineProps<{ height?: number }>(), { height: 24 });

const theme = ref<"light" | "dark">("dark");
const src = computed(() => (theme.value === "light" ? markOnLight : markOnDark));
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
