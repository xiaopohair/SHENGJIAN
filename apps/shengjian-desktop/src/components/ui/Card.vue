<script setup lang="ts">
withDefaults(
  defineProps<{
    padding?: "none" | "sm" | "md" | "lg";
  }>(),
  { padding: "md" },
);
</script>

<template>
  <section class="ui-card" :class="`padding-${padding}`">
    <slot />
  </section>
</template>

<style scoped>
.ui-card {
  min-width: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-raised);
  box-shadow: var(--shadow-sm);
}

.ui-card.padding-none {
  padding: 0;
}

.ui-card.padding-sm {
  padding: var(--space-3);
}

.ui-card.padding-md {
  padding: var(--space-5);
}

.ui-card.padding-lg {
  padding: var(--space-6);
}
</style>
