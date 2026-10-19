<script setup lang="ts">
import { useId } from 'vue';
defineProps<{ title?: string; subtitle?: string }>();
const id = useId();
</script>

<template>
  <section class="ui-card" :aria-labelledby="title ? id : undefined">
    <header v-if="title || $slots.actions" class="ui-card-header">
      <div class="ui-card-heading">
        <h2 :id="id">{{ title }}</h2>
        <span v-if="subtitle">{{ subtitle }}</span>
      </div>
      <div v-if="$slots.actions" class="ui-card-actions"><slot name="actions" /></div>
    </header>
    <slot />
  </section>
</template>
