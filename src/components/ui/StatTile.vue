<script setup lang="ts">
import { computed } from 'vue';
import { ArrowRight, RotateCw } from 'lucide-vue-next';
import Skeleton from './Skeleton.vue';
import UiIconButton from './UiIconButton.vue';

/**
 * One metric in a StatRow (LAYOUT §6.1). A clickable tile is a real button role
 * so it opens the detail behind the number; a failed tile shows "—" and its own
 * retry and never blanks the rest of the row.
 */
const props = withDefaults(
  defineProps<{
    label: string;
    value?: string | number | null;
    note?: string;
    loading?: boolean;
    failed?: boolean;
    clickable?: boolean;
    /** 26px figures on the home page, 22px elsewhere. */
    size?: 'md' | 'lg';
  }>(),
  { value: null, note: '', loading: false, failed: false, clickable: false, size: 'md' },
);
const emit = defineEmits<{ select: []; retry: [] }>();

const text = computed(() =>
  props.value == null || props.value === '' ? '—' : String(props.value),
);
</script>

<template>
  <div
    class="stat-tile"
    :class="{ clickable, large: size === 'lg' }"
    :role="clickable ? 'button' : undefined"
    :tabindex="clickable ? 0 : undefined"
    @click="clickable && emit('select')"
    @keydown.enter="clickable && emit('select')"
    @keydown.space.prevent="clickable && emit('select')"
  >
    <span class="stat-label">{{ label }}</span>
    <span v-if="loading" class="stat-value">
      <Skeleton width="72px" height="20px" />
    </span>
    <span v-else-if="failed" class="stat-failed">
      <span class="stat-value">—</span>
      <UiIconButton size="xs" label="重新读取" @click.stop="emit('retry')">
        <RotateCw :size="13" />
      </UiIconButton>
    </span>
    <span v-else class="stat-value">{{ text }}</span>
    <span v-if="note && !loading" class="stat-note">{{ note }}</span>
    <ArrowRight v-if="clickable" class="stat-arrow" :size="14" aria-hidden="true" />
  </div>
</template>

<style scoped>
.stat-tile {
  position: relative;
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
  padding: 12px 14px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-xs);
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease;
}
.stat-tile.clickable {
  padding-right: 28px;
  text-align: left;
  cursor: pointer;
}
.stat-tile.clickable:hover {
  background: var(--accent-wash);
  border-color: var(--border-strong);
}
.stat-tile.clickable:active {
  background: var(--surface-sunken);
}
.stat-tile.clickable:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.stat-label {
  color: var(--text-secondary);
  font-size: 12px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.stat-value {
  display: flex;
  align-items: center;
  gap: 4px;
  min-height: 28px;
  color: var(--text);
  font-size: 22px;
  font-weight: 600;
  line-height: 1.3;
  font-variant-numeric: tabular-nums;
  letter-spacing: -0.01em;
}
.stat-tile.large .stat-value {
  font-size: 26px;
}
.stat-failed .stat-value {
  color: var(--text-tertiary);
}
.stat-note {
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.45;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.stat-arrow {
  position: absolute;
  top: 50%;
  right: 10px;
  margin-top: -7px;
  color: var(--text-tertiary);
  opacity: 0;
  transition:
    opacity var(--duration-fast) ease,
    transform var(--duration-fast) var(--ease-out);
}
.stat-tile.clickable:hover .stat-arrow {
  opacity: 1;
  transform: translateX(2px);
}
@media (prefers-reduced-motion: reduce) {
  .stat-arrow {
    transition: none;
  }
}
</style>
