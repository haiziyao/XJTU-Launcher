<script setup lang="ts">
/**
 * Equal-width metric row (LAYOUT §6.1): 3–5 StatTiles on one line, folding into
 * two lines when the content column is narrow. Keeps its own 20px gap below.
 */
withDefaults(defineProps<{ label?: string }>(), { label: '指标' });
</script>

<template>
  <div class="stat-row" role="group" :aria-label="label">
    <slot />
  </div>
</template>

<style scoped>
.stat-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(132px, 1fr));
  gap: 12px;
  margin-bottom: 20px;
}
</style>
