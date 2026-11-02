<script lang="ts">
import type { Component } from 'vue';

export interface RailItem {
  id: string;
  title: string;
  icon?: Component;
  /** Count badge; only numbers, never a red dot (LAYOUT §2.1). */
  badge?: number;
  disabled?: boolean;
  /** Why a disabled section stays in the rail: 需校园网 / 待接入 / 需直连. */
  reason?: string;
  /** Group heading; items without one sit at the top. */
  group?: string;
}
</script>

<script setup lang="ts">
import { computed, ref } from 'vue';
import { useSlidingIndicator } from '../../useSlidingIndicator';

/**
 * Section navigation for the 畅行校园 workspace (LAYOUT §3.1, §4.2). Disabled
 * sections stay visible with their reason: hiding them would make the feature
 * list look complete when it is not.
 */
const props = withDefaults(
  defineProps<{
    items: RailItem[];
    modelValue: string;
    collapsed?: boolean;
    label?: string;
  }>(),
  { collapsed: false, label: '校园分区' },
);
const emit = defineEmits<{ 'update:modelValue': [id: string]; select: [item: RailItem] }>();

const nav = ref<HTMLElement | null>(null);
const indicator = useSlidingIndicator(nav, () => props.modelValue);

const groups = computed(() => {
  const result: { title: string; items: RailItem[] }[] = [];
  for (const item of props.items) {
    const title = item.group ?? '';
    let group = result.find((entry) => entry.title === title);
    if (!group) {
      group = { title, items: [] };
      result.push(group);
    }
    group.items.push(item);
  }
  return result;
});

function pick(item: RailItem) {
  if (item.disabled) return;
  emit('update:modelValue', item.id);
  emit('select', item);
}
function hint(item: RailItem) {
  return item.disabled && item.reason ? `${item.title}，${item.reason}` : item.title;
}
</script>

<template>
  <nav ref="nav" class="campus-rail" :class="{ collapsed }" :aria-label="label">
    <span
      class="rail-indicator"
      :class="{ animated: indicator.animated.value }"
      :style="indicator.style.value"
      aria-hidden="true"
    ></span>
    <div v-for="group in groups" :key="group.title || 'root'" class="rail-group">
      <div v-if="group.title && !collapsed" class="rail-group-title">{{ group.title }}</div>
      <button
        v-for="item in group.items"
        :key="item.id"
        type="button"
        class="rail-item"
        :class="{ active: modelValue === item.id, unavailable: item.disabled }"
        :data-indicator="item.id"
        :disabled="item.disabled"
        :title="hint(item)"
        :aria-label="hint(item)"
        :aria-current="modelValue === item.id ? 'page' : undefined"
        @click="pick(item)"
      >
        <component :is="item.icon" v-if="item.icon" :size="17" aria-hidden="true" />
        <span v-if="!collapsed" class="rail-title">{{ item.title }}</span>
        <span v-if="item.badge && !collapsed" class="rail-badge">{{ item.badge }}</span>
      </button>
    </div>
  </nav>
</template>

<style scoped>
.campus-rail {
  position: sticky;
  top: 0;
  display: flex;
  flex-direction: column;
  gap: 10px;
  width: 168px;
  flex-shrink: 0;
}
.campus-rail.collapsed {
  width: 48px;
}
/* No position here: the sliding indicator measures offsetTop against the nav,
   so each item's offset parent must stay the nav itself. */
.rail-group {
  display: flex;
  flex-direction: column;
  gap: 2px;
}
.rail-group-title {
  padding: 0 10px;
  margin-bottom: 2px;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.02em;
}
.rail-indicator {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  border-radius: var(--radius-sm);
  background: var(--accent-soft);
  pointer-events: none;
  opacity: 0;
}
.rail-indicator.animated {
  transition:
    transform 300ms var(--ease-standard),
    height 300ms var(--ease-standard),
    opacity var(--duration) ease;
}
.rail-item {
  position: relative;
  display: flex;
  align-items: center;
  gap: 9px;
  width: 100%;
  height: 32px;
  padding: 0 10px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  text-align: left;
  white-space: nowrap;
  transition:
    background var(--duration-fast) ease,
    color var(--duration) ease;
}
.collapsed .rail-item {
  justify-content: center;
  padding: 0;
}
.rail-item:hover:not(.active):not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}
.rail-item.active {
  color: var(--accent-text);
  font-weight: 500;
}
.rail-item.unavailable {
  color: var(--text-tertiary);
  opacity: 0.55;
}
.rail-title {
  overflow: hidden;
  text-overflow: ellipsis;
}
.rail-badge {
  min-width: 18px;
  height: 16px;
  margin-left: auto;
  padding: 0 5px;
  border-radius: var(--radius-full);
  background: var(--surface);
  color: var(--text-secondary);
  font-size: 10.5px;
  font-weight: 500;
  line-height: 16px;
  text-align: center;
  font-variant-numeric: tabular-nums;
}
.rail-item.active .rail-badge {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.rail-item.unavailable .rail-badge {
  background: var(--surface-sunken);
}
</style>
