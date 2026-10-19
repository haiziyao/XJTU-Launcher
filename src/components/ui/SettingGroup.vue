<script setup lang="ts">
import { useId } from 'vue';
defineProps<{ title?: string; footnote?: string }>();
const id = useId();
</script>

<template>
  <section class="setting-group" :aria-labelledby="title ? id : undefined">
    <header v-if="title || $slots.actions" class="setting-group-header">
      <h3 :id="id">{{ title }}</h3>
      <div v-if="$slots.actions" class="row-actions"><slot name="actions" /></div>
    </header>
    <div class="setting-card"><slot /></div>
    <p v-if="footnote" class="setting-group-footnote">{{ footnote }}</p>
  </section>
</template>
