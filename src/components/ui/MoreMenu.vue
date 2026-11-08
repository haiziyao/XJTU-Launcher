<script lang="ts">
import type { Component } from 'vue';

export interface MoreItem {
  id: string;
  label: string;
  icon?: Component;
  danger?: boolean;
  disabled?: boolean;
}
</script>

<script setup lang="ts">
import { onBeforeUnmount, ref, watch } from 'vue';
import { MoreHorizontal } from 'lucide-vue-next';
import UiIconButton from './UiIconButton.vue';

/**
 * Compact "⋯" menu for secondary actions (LAYOUT §1.6): the campus web fallback
 * link, exports, mark-as-unread. Primary actions stay on the page.
 */
const props = withDefaults(
  defineProps<{
    items: MoreItem[];
    label?: string;
    align?: 'left' | 'right';
    disabled?: boolean;
  }>(),
  { label: '更多操作', align: 'right', disabled: false },
);
const emit = defineEmits<{ select: [id: string] }>();

const open = ref(false);
const root = ref<HTMLElement | null>(null);

function choose(item: MoreItem) {
  if (item.disabled) return;
  open.value = false;
  emit('select', item.id);
}
function onOutside(event: PointerEvent) {
  if (root.value && !root.value.contains(event.target as Node)) open.value = false;
}
watch(open, (value) => {
  if (value) document.addEventListener('pointerdown', onOutside, true);
  else document.removeEventListener('pointerdown', onOutside, true);
});
onBeforeUnmount(() => document.removeEventListener('pointerdown', onOutside, true));
</script>

<template>
  <div ref="root" class="more-menu" @keydown.esc="open = false">
    <UiIconButton
      :label="label"
      size="md"
      :disabled="props.disabled"
      :active="open"
      aria-haspopup="menu"
      :aria-expanded="open"
      @click="open = !open"
    >
      <MoreHorizontal :size="16" />
    </UiIconButton>
    <Transition name="drop">
      <ul v-if="open" class="more-menu-list" :class="align" role="menu" :aria-label="label">
        <li v-for="item in items" :key="item.id">
          <button
            type="button"
            role="menuitem"
            :class="{ danger: item.danger }"
            :disabled="item.disabled"
            @click="choose(item)"
          >
            <component :is="item.icon" v-if="item.icon" :size="14" aria-hidden="true" />
            <span>{{ item.label }}</span>
          </button>
        </li>
      </ul>
    </Transition>
  </div>
</template>

<style scoped>
.more-menu {
  position: relative;
  display: inline-flex;
}
.more-menu-list {
  position: absolute;
  z-index: 20;
  top: calc(100% + 6px);
  min-width: 176px;
  max-width: 260px;
  margin: 0;
  padding: 4px;
  list-style: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  box-shadow: var(--shadow-md);
}
.more-menu-list.right {
  right: 0;
}
.more-menu-list.left {
  left: 0;
}
.more-menu-list button {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 100%;
  padding: 6px 8px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  font-size: 12.5px;
  text-align: left;
  white-space: nowrap;
  cursor: default;
}
.more-menu-list button:hover:not(:disabled),
.more-menu-list button:focus-visible {
  background: var(--accent-wash);
  color: var(--accent-text);
  outline: none;
}
.more-menu-list button.danger {
  color: var(--danger);
}
.more-menu-list button.danger:hover:not(:disabled) {
  background: var(--danger-soft);
  color: var(--danger);
}
.more-menu-list svg {
  color: var(--text-tertiary);
}
.more-menu-list button:hover svg {
  color: inherit;
}
</style>
