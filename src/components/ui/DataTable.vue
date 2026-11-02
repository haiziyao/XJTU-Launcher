<script lang="ts">
export interface DataColumn {
  key: string;
  label: string;
  align?: 'left' | 'right' | 'center';
  sortable?: boolean;
  width?: string;
}

export interface DataSort {
  key: string;
  dir: 'asc' | 'desc';
}
</script>

<script setup lang="ts">
import { computed } from 'vue';
import { ArrowDown, ArrowUp, ChevronsUpDown } from 'lucide-vue-next';
import Skeleton from './Skeleton.vue';

type Row = Record<string, unknown>;

/**
 * Sortable table (LAYOUT §4.2, §9.4): sticky header, optional row selection,
 * numerals right-aligned with tabular figures. Sorting and selection are
 * controlled by the page; cell content can be replaced per column with the
 * `cell-<key>` slot.
 */
const props = withDefaults(
  defineProps<{
    columns: DataColumn[];
    rows: Row[];
    /** Property that identifies a row for selection. */
    rowKey: string;
    sort?: DataSort | null;
    selected?: (string | number)[];
    selectable?: boolean;
    loading?: boolean;
    emptyText?: string;
    stickyHeader?: boolean;
    label?: string;
  }>(),
  {
    sort: null,
    selected: () => [],
    selectable: false,
    loading: false,
    emptyText: '没有数据',
    stickyHeader: true,
    label: '数据表',
  },
);
const emit = defineEmits<{
  'update:sort': [sort: DataSort | null];
  'update:selected': [keys: (string | number)[]];
  select: [row: Row];
}>();

const sorted = computed(() => {
  const sort = props.sort;
  if (!sort) return props.rows;
  const column = props.columns.find((entry) => entry.key === sort.key);
  const factor = sort.dir === 'asc' ? 1 : -1;
  return [...props.rows].sort((a, b) => {
    const left = a[sort.key];
    const right = b[sort.key];
    if (left == null) return 1;
    if (right == null) return -1;
    if (column?.align === 'right' || (typeof left === 'number' && typeof right === 'number')) {
      return (Number(left) - Number(right)) * factor;
    }
    return String(left).localeCompare(String(right), 'zh-Hans-CN', { numeric: true }) * factor;
  });
});

const keyOf = (row: Row) => row[props.rowKey] as string | number;
const isSelected = (row: Row) => props.selected.includes(keyOf(row));
const allSelected = computed(
  () => props.rows.length > 0 && props.rows.every((row) => isSelected(row)),
);

function sortBy(column: DataColumn) {
  if (!column.sortable) return;
  const dir = props.sort?.key === column.key && props.sort.dir === 'asc' ? 'desc' : 'asc';
  emit('update:sort', { key: column.key, dir });
}
function toggleAll() {
  emit('update:selected', allSelected.value ? [] : props.rows.map((row) => keyOf(row)));
}
function toggleRow(row: Row) {
  const key = keyOf(row);
  emit(
    'update:selected',
    isSelected(row) ? props.selected.filter((entry) => entry !== key) : [...props.selected, key],
  );
}
function ariaSort(column: DataColumn) {
  if (props.sort?.key !== column.key) return undefined;
  return props.sort.dir === 'asc' ? 'ascending' : 'descending';
}
</script>

<template>
  <div class="data-table-wrap">
    <table class="data-table" :class="{ 'sticky-header': stickyHeader }" :aria-label="label">
      <thead>
        <tr>
          <th v-if="selectable" class="check-cell" scope="col">
            <input
              type="checkbox"
              :checked="allSelected"
              :indeterminate.prop="!allSelected && selected.length > 0"
              aria-label="全选"
              @change="toggleAll"
            />
          </th>
          <th
            v-for="column in columns"
            :key="column.key"
            scope="col"
            :style="column.width ? { width: column.width } : undefined"
            :class="[`align-${column.align ?? 'left'}`, { sortable: column.sortable }]"
            :aria-sort="ariaSort(column)"
          >
            <button
              v-if="column.sortable"
              type="button"
              class="sort-button"
              :class="`align-${column.align ?? 'left'}`"
              @click="sortBy(column)"
            >
              <span>{{ column.label }}</span>
              <ArrowUp v-if="sort?.key === column.key && sort.dir === 'asc'" :size="12" />
              <ArrowDown v-else-if="sort?.key === column.key && sort.dir === 'desc'" :size="12" />
              <ChevronsUpDown v-else :size="12" class="sort-idle" />
            </button>
            <template v-else>{{ column.label }}</template>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-if="loading">
          <td :colspan="columns.length + (selectable ? 1 : 0)">
            <Skeleton :lines="3" height="14px" label="正在载入表格" />
          </td>
        </tr>
        <tr v-else-if="!sorted.length">
          <td class="empty-cell" :colspan="columns.length + (selectable ? 1 : 0)">
            {{ emptyText }}
          </td>
        </tr>
        <template v-else>
          <tr
            v-for="(row, index) in sorted"
            :key="keyOf(row)"
            :class="{ selected: selectable && isSelected(row) }"
            tabindex="0"
            @click="emit('select', row)"
            @keydown.enter="emit('select', row)"
          >
            <td v-if="selectable" class="check-cell" @click.stop>
              <input
                type="checkbox"
                :checked="isSelected(row)"
                :aria-label="`选择第 ${index + 1} 行`"
                @change="toggleRow(row)"
              />
            </td>
            <td
              v-for="column in columns"
              :key="column.key"
              :class="`align-${column.align ?? 'left'}`"
            >
              <slot :name="`cell-${column.key}`" :row="row" :value="row[column.key]">
                {{ row[column.key] }}
              </slot>
            </td>
          </tr>
        </template>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.data-table-wrap {
  min-width: 0;
  overflow: auto;
}
.data-table {
  width: 100%;
  border-collapse: separate;
  border-spacing: 0;
  font-size: 12.5px;
}
.data-table th {
  padding: 7px 10px;
  border-bottom: 1px solid var(--border);
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 500;
  text-align: left;
  white-space: nowrap;
  background: var(--surface);
}
.sticky-header thead th {
  position: sticky;
  z-index: 2;
  top: 0;
}
.data-table td {
  padding: 9px 10px;
  border-bottom: 1px solid var(--divider);
  color: var(--text);
  vertical-align: middle;
}
.data-table tbody tr:last-child td {
  border-bottom: 0;
}
.data-table tbody tr {
  transition: background var(--duration-fast) ease;
}
.data-table tbody tr:hover {
  background: var(--accent-wash);
}
.data-table tbody tr:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.data-table tbody tr.selected {
  background: var(--accent-soft);
}
.align-right {
  text-align: right;
  font-variant-numeric: tabular-nums;
}
.align-center {
  text-align: center;
}
.check-cell {
  width: 34px;
  padding-right: 0;
}
.check-cell input {
  width: 14px;
  height: 14px;
  margin: 0;
  accent-color: var(--accent);
}
.sort-button {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  cursor: pointer;
}
.sort-button.align-right {
  flex-direction: row-reverse;
}
.sort-button:hover {
  color: var(--text);
}
.sort-idle {
  opacity: 0.45;
}
.sortable[aria-sort='ascending'],
.sortable[aria-sort='descending'] {
  color: var(--text);
}
.empty-cell {
  padding: 28px 10px;
  color: var(--text-tertiary);
  text-align: center;
}
</style>
