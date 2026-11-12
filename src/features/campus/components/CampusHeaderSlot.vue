<script setup lang="ts">
import { computed } from 'vue';
import { useCampusHeader } from '../campusHeader';

const props = defineProps<{ slot: 'meta' | 'tools' }>();
const header = useCampusHeader();
const target = computed(() => (header ? header[props.slot].value : null));
</script>

<template>
  <Teleport v-if="target" :to="target"><slot /></Teleport>
</template>
