<script lang="ts">
export interface SlotCourt {
  id: string;
  name: string;
}

export interface SlotCell {
  time: string;
  /** Court id, matching SlotCourt.id. */
  court: string;
  stockId?: number;
  /** Price in yuan, as the venue list returns it. */
  price?: number;
  state: 'available' | 'full' | 'closed';
}
</script>

<script setup lang="ts">
import { computed } from 'vue';

/**
 * Venue time × court grid (LAYOUT §9.9). Bookable cells show the price; sold-out
 * and unbookable cells say 满 / 不可订 in words, so the state never depends on
 * colour. Selection is a list of stock ids and only exists when the page enables
 * booking (P2).
 */
const props = withDefaults(
  defineProps<{
    times: string[];
    courts: SlotCourt[];
    cells: SlotCell[];
    modelValue?: number[];
    /** Booking is disabled while the write protocol is not available. */
    selectable?: boolean;
    label?: string;
  }>(),
  { modelValue: () => [], selectable: false, label: '场馆时段' },
);
const emit = defineEmits<{ 'update:modelValue': [ids: number[]]; select: [cell: SlotCell] }>();

const index = computed(() => {
  const map = new Map<string, SlotCell>();
  for (const cell of props.cells) map.set(`${cell.time}|${cell.court}`, cell);
  return map;
});
const total = computed(() =>
  props.cells
    .filter((cell) => cell.stockId != null && props.modelValue.includes(cell.stockId))
    .reduce((sum, cell) => sum + (cell.price ?? 0), 0),
);

const yuan = (price: number) => (Number.isInteger(price) ? `¥${price}` : `¥${price.toFixed(2)}`);

function cell(time: string, court: SlotCourt): SlotCell | undefined {
  return index.value.get(`${time}|${court.id}`);
}
function picked(cell: SlotCell | undefined) {
  return cell?.stockId != null && props.modelValue.includes(cell.stockId);
}
function choose(cell: SlotCell | undefined) {
  if (!cell || cell.state !== 'available' || cell.stockId == null) {
    if (cell) emit('select', cell);
    return;
  }
  emit('select', cell);
  if (!props.selectable) return;
  emit(
    'update:modelValue',
    picked(cell)
      ? props.modelValue.filter((id) => id !== cell.stockId)
      : [...props.modelValue, cell.stockId as number],
  );
}
</script>

<template>
  <div class="slot-matrix" role="group" :aria-label="label">
    <table>
      <thead>
        <tr>
          <th scope="col" class="slot-time-head">时间</th>
          <th v-for="court in courts" :key="court.id" scope="col">{{ court.name }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="time in times" :key="time">
          <th scope="row" class="slot-time">{{ time }}</th>
          <td v-for="court in courts" :key="court.id">
            <button
              v-if="cell(time, court)"
              type="button"
              class="slot-cell"
              :class="[`is-${cell(time, court)?.state}`, { picked: picked(cell(time, court)) }]"
              :disabled="cell(time, court)?.state !== 'available'"
              :aria-pressed="selectable ? picked(cell(time, court)) : undefined"
              :aria-label="`${court.name} ${time}，${
                cell(time, court)?.state === 'available'
                  ? yuan(cell(time, court)?.price ?? 0)
                  : cell(time, court)?.state === 'full'
                    ? '已约满'
                    : '不可预订'
              }`"
              @click="choose(cell(time, court))"
            >
              {{
                cell(time, court)?.state === 'available'
                  ? yuan(cell(time, court)?.price ?? 0)
                  : cell(time, court)?.state === 'full'
                    ? '满'
                    : '不可订'
              }}
            </button>
            <span v-else class="slot-cell is-empty" aria-hidden="true">—</span>
          </td>
        </tr>
      </tbody>
    </table>
    <div v-if="selectable && modelValue.length" class="slot-bar">
      <span class="slot-summary"> 已选 {{ modelValue.length }} 个时段 · {{ yuan(total) }} </span>
      <slot name="bar" :count="modelValue.length" :total="total" />
    </div>
  </div>
</template>

<style scoped>
.slot-matrix {
  min-width: 0;
}
table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 2px;
  font-size: 12px;
}
th {
  padding: 4px 6px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 500;
  text-align: center;
  white-space: nowrap;
}
.slot-time-head,
.slot-time {
  text-align: left;
  font-variant-numeric: tabular-nums;
}
.slot-time {
  color: var(--text-secondary);
}
.slot-cell {
  display: grid;
  place-items: center;
  width: 100%;
  min-width: 52px;
  height: 30px;
  padding: 0 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  color: var(--text);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease;
}
button.slot-cell:hover:not(:disabled) {
  border-color: var(--accent-outline);
  background: var(--accent-wash);
}
button.slot-cell:active:not(:disabled) {
  background: var(--accent-soft);
}
.slot-cell.picked {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent-text);
  font-weight: 600;
}
.slot-cell.is-full,
.slot-cell.is-closed {
  border-color: transparent;
  background: var(--surface-sunken);
  color: var(--text-tertiary);
}
.slot-cell.is-empty {
  border-color: transparent;
  color: var(--text-tertiary);
}
.slot-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  margin-top: 10px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface-subtle);
}
.slot-summary {
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
</style>
