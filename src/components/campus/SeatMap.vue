<script setup lang="ts">
import { computed } from 'vue';
import type { LibrarySeat } from '../../types';

/**
 * Library seat map (LAYOUT §9.8): free seats are white with an accent outline
 * and the word 空; taken seats are grey. The label, not the colour, is what
 * tells them apart. Booking stays disabled until the write protocol is probed.
 */
const props = withDefaults(
  defineProps<{
    seats: LibrarySeat[];
    selected?: string | null;
    selectable?: boolean;
    label?: string;
  }>(),
  { selected: null, selectable: false, label: '座位图' },
);
const emit = defineEmits<{ select: [seat: LibrarySeat] }>();

const free = computed(() => props.seats.filter((seat) => seat.free).length);

function choose(seat: LibrarySeat) {
  if (!seat.free || !props.selectable) return;
  emit('select', seat);
}
</script>

<template>
  <div class="seat-map" role="group" :aria-label="`${label}，空余 ${free} 个`">
    <div class="seat-grid">
      <button
        v-for="seat in seats"
        :key="seat.id"
        type="button"
        class="seat"
        :class="{ free: seat.free, taken: !seat.free, selected: selected === seat.id }"
        :disabled="!seat.free || !selectable"
        :aria-label="`座位 ${seat.id}，${seat.free ? '空余' : '已占用'}`"
        :aria-pressed="selectable && seat.free ? selected === seat.id : undefined"
        @click="choose(seat)"
      >
        {{ seat.free ? '空' : '' }}
      </button>
    </div>
    <div class="seat-legend">
      <span><i class="swatch free"></i>空余 {{ free }}</span>
      <span><i class="swatch taken"></i>已占用 {{ seats.length - free }}</span>
    </div>
  </div>
</template>

<style scoped>
.seat-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, 26px);
  gap: 4px;
  justify-content: start;
}
.seat {
  display: grid;
  place-items: center;
  width: 26px;
  height: 26px;
  padding: 0;
  border: 1px solid transparent;
  border-radius: var(--radius-xs);
  font-size: 10.5px;
  line-height: 1;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease;
}
.seat.free {
  border-color: var(--accent-outline);
  background: var(--surface);
  color: var(--accent-text);
}
.seat.free:hover:not(:disabled) {
  background: var(--accent-wash);
}
.seat.taken {
  background: var(--surface-sunken);
  color: transparent;
}
.seat.selected {
  border-color: var(--accent);
  background: var(--accent);
  color: #fff;
}
.seat:disabled {
  cursor: default;
  opacity: 1;
}
.seat.taken:disabled {
  opacity: 1;
}
.seat-legend {
  display: flex;
  gap: 14px;
  margin-top: 12px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.seat-legend span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.swatch {
  width: 11px;
  height: 11px;
  border-radius: 3px;
}
.swatch.free {
  border: 1px solid var(--accent-outline);
  background: var(--surface);
}
.swatch.taken {
  background: var(--surface-sunken);
}
</style>
