<script setup lang="ts">
import { computed, type Component } from 'vue';
import { entryIcon } from '../../shared/icons';

/**
 * A sidebar entry's glyph: the user's own image when `icons.json` maps the id,
 * otherwise the built-in icon component the caller passes.
 */
const props = withDefaults(
  defineProps<{
    /** Stable entry id, e.g. `nav.home` or `campus.todo`. */
    name: string;
    icon?: Component;
    /** Matches the built-in glyph size so swapping never shifts the row. */
    size?: number;
  }>(),
  { size: 20 },
);
const src = computed(() => entryIcon(props.name));
const box = computed(() => `${props.size}px`);
</script>
<template>
  <img v-if="src" :src="src" :style="{ width: box, height: box }" class="entry-icon" alt="" />
  <component :is="icon" v-else-if="icon" :size="size" />
</template>
<style scoped>
.entry-icon {
  flex-shrink: 0;
  /* A percentage radius follows `size`, so 17px rail icons match 20px nav. */
  border-radius: 24%;
  object-fit: cover;
}
</style>
