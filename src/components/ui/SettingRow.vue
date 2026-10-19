<script setup lang="ts">
import { ChevronRight } from 'lucide-vue-next';
// Clickable rows render as a button with a chevron; keep their slots free of other controls.
defineProps<{ title?: string; description?: string; clickable?: boolean; disabled?: boolean }>();
const emit = defineEmits<{ select: [] }>();
</script>

<template>
  <component
    :is="clickable ? 'button' : 'div'"
    class="setting-row"
    :class="{ clickable }"
    :type="clickable ? 'button' : undefined"
    :disabled="clickable ? disabled : undefined"
    @click="clickable && emit('select')"
  >
    <span v-if="$slots.leading" class="setting-row-leading"><slot name="leading" /></span>
    <span class="setting-row-copy">
      <strong
        ><slot name="title">{{ title }}</slot></strong
      >
      <small v-if="description || $slots.description"
        ><slot name="description">{{ description }}</slot></small
      >
    </span>
    <span v-if="$slots.default" class="setting-row-control"><slot /></span>
    <ChevronRight v-if="clickable" :size="16" class="setting-row-chevron" aria-hidden="true" />
  </component>
</template>
