<script lang="ts">
export type Tone = 'blue' | 'green' | 'orange' | 'violet' | 'red' | 'teal' | 'gray';
</script>

<script setup lang="ts">
withDefaults(defineProps<{ tone?: Tone; size?: number }>(), { tone: 'blue', size: 36 });
</script>

<template>
  <span class="icon-tile" :class="`tone-${tone}`" :style="{ '--tile-size': size + 'px' }">
    <slot />
  </span>
</template>
