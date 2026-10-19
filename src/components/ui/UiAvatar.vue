<script setup lang="ts">
import { UserRound } from 'lucide-vue-next';
withDefaults(defineProps<{ src?: string | null; size?: number; alt?: string }>(), {
  src: null,
  size: 32,
  alt: '',
});
</script>

<template>
  <span class="ui-avatar" :style="{ '--avatar-size': size + 'px' }">
    <img v-if="src" :src="src" :alt="alt" /><UserRound
      v-else
      :size="Math.round(size * 0.55)"
      :stroke-width="1.7"
    />
  </span>
</template>
