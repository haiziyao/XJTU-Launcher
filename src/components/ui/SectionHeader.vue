<script setup lang="ts">
import { computed } from 'vue';
import { RotateCw } from 'lucide-vue-next';
import UiIconButton from './UiIconButton.vue';
import { updatedInfo } from '../../shared/domain';

/**
 * Header of one campus section (LAYOUT §3.3): title, optional subtitle and
 * update time on the left; filters, refresh and the "⋯" menu on the right.
 * `sticky` keeps it visible while the list scrolls under it.
 */
const props = withDefaults(
  defineProps<{
    title: string;
    subtitle?: string;
    updatedAt?: number | null;
    refreshing?: boolean;
    sticky?: boolean;
  }>(),
  { subtitle: '', updatedAt: null, refreshing: false, sticky: false },
);
const emit = defineEmits<{ refresh: [] }>();

// “今天 16:20” becomes “16:20” here; the word 更新于 already says when.
const updated = computed(() =>
  props.updatedAt == null ? '' : updatedInfo(props.updatedAt).replace(/^今天 /, ''),
);
</script>

<template>
  <header class="section-header" :class="{ sticky }">
    <div class="section-heading">
      <h2>{{ title }}</h2>
      <span v-if="subtitle" class="section-subtitle">{{ subtitle }}</span>
      <span v-if="updated" class="section-updated">更新于 {{ updated }}</span>
    </div>
    <div class="section-controls">
      <slot name="filters" />
      <slot name="actions" />
      <UiIconButton
        :label="refreshing ? '正在刷新' : '刷新'"
        :disabled="refreshing"
        @click="emit('refresh')"
      >
        <RotateCw :size="15" :class="{ spinning: refreshing }" />
      </UiIconButton>
      <slot name="more" />
    </div>
  </header>
</template>

<style scoped>
.section-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  min-height: 32px;
  margin-bottom: 14px;
}
.section-header.sticky {
  position: sticky;
  z-index: 6;
  top: 0;
  padding: 8px 0;
  background: var(--surface);
}
.section-heading {
  display: flex;
  align-items: baseline;
  gap: 8px;
  min-width: 0;
}
.section-heading h2 {
  font-size: 15px;
  font-weight: 600;
  letter-spacing: -0.01em;
  white-space: nowrap;
}
.section-subtitle {
  color: var(--text-secondary);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.section-updated {
  color: var(--text-tertiary);
  font-size: 11.5px;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.section-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
</style>
