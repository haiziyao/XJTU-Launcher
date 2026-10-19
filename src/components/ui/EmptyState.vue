<script setup lang="ts">
import type { Component } from 'vue';
defineProps<{
  icon?: Component;
  title: string;
  description?: string;
  /** Spins the icon, for loading states. */
  busy?: boolean;
  tone?: 'default' | 'success' | 'danger';
  compact?: boolean;
}>();
</script>

<template>
  <div class="empty-state" :class="[`tone-${tone ?? 'default'}`, { compact }]">
    <span v-if="icon" class="empty-state-icon" aria-hidden="true">
      <component
        :is="icon"
        :size="compact ? 18 : 22"
        :stroke-width="1.7"
        :class="{ spinning: busy }"
      />
    </span>
    <h3>{{ title }}</h3>
    <p v-if="description" :role="tone === 'danger' ? 'alert' : undefined">{{ description }}</p>
    <div v-if="$slots.default" class="empty-state-actions"><slot /></div>
  </div>
</template>
