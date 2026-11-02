<script setup lang="ts">
/**
 * Loading placeholder for first paint. Pages use it for stat tiles and list
 * rows instead of a spinner (LAYOUT §8); the shape should match the content
 * that replaces it so the page does not jump.
 */
withDefaults(
  defineProps<{
    width?: string;
    height?: string;
    radius?: string;
    lines?: number;
    label?: string;
  }>(),
  {
    width: '100%',
    height: '12px',
    radius: 'var(--radius-sm)',
    lines: 1,
    label: '',
  },
);
</script>

<template>
  <div class="skeleton-stack" role="status" :aria-busy="true" :aria-label="label || '正在载入'">
    <span
      v-for="line in lines"
      :key="line"
      class="skeleton"
      :style="{ width, height, borderRadius: radius }"
      aria-hidden="true"
    />
  </div>
</template>

<style scoped>
.skeleton-stack {
  display: flex;
  flex-direction: column;
  gap: 8px;
  width: 100%;
}
.skeleton {
  display: block;
  background: linear-gradient(
    90deg,
    var(--surface-sunken) 0%,
    #e9ebef 45%,
    var(--surface-sunken) 90%
  );
  background-size: 220% 100%;
  animation: skeleton-sweep 1.4s ease-in-out infinite;
}
@keyframes skeleton-sweep {
  from {
    background-position: 120% 0;
  }
  to {
    background-position: -120% 0;
  }
}
@media (prefers-reduced-motion: reduce) {
  .skeleton {
    animation: none;
  }
}
</style>
