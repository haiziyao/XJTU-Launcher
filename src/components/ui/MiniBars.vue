<script lang="ts">
export interface MiniBar {
  label: string;
  value: number;
  /** Today's bar stays at full opacity; the rest are held back. */
  highlight?: boolean;
}
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

/**
 * Single-series bar chart for a short series such as daily spending
 * (LAYOUT §6.2). Hand-drawn SVG, one axis, 2px gaps, 4px rounded data-ends;
 * values are text tokens, never the series colour. Hover and keyboard focus
 * both show the value, and “查看数据” swaps in a table.
 */
const props = withDefaults(
  defineProps<{
    values: MiniBar[];
    height?: number;
    label?: string;
    format?: (value: number) => string;
  }>(),
  { height: 48, label: '柱状图', format: (value: number) => String(value) },
);

const root = ref<HTMLElement | null>(null);
const width = ref(240);
const hover = ref(-1);
const showTable = ref(false);
let observer: ResizeObserver | undefined;

onMounted(() => {
  if (!root.value) return;
  observer = new ResizeObserver((entries) => {
    width.value = Math.max(60, entries[0]?.contentRect.width ?? 240);
  });
  observer.observe(root.value);
  width.value = Math.max(60, root.value.getBoundingClientRect().width);
});
onBeforeUnmount(() => observer?.disconnect());

const GAP = 2;
const RADIUS = 4;
const max = computed(() => Math.max(1, ...props.values.map((entry) => entry.value)));
const slot = computed(() => width.value / Math.max(1, props.values.length));
const barWidth = computed(() => Math.max(2, Math.min(24, slot.value - GAP)));

const bars = computed(() =>
  props.values.map((entry, index) => {
    const bar = barWidth.value;
    const x = index * slot.value + (slot.value - bar) / 2;
    const value = Math.max(0, entry.value);
    const height = value === 0 ? 0 : Math.max(2, (value / max.value) * (props.height - 2));
    const y = props.height - height;
    const radius = Math.min(RADIUS, bar / 2, height);
    const path =
      height > 0 && radius > 0
        ? `M${x},${y + height} L${x},${y + radius} Q${x},${y} ${x + radius},${y} ` +
          `L${x + bar - radius},${y} Q${x + bar},${y} ${x + bar},${y + radius} ` +
          `L${x + bar},${y + height} Z`
        : `M${x},${y} h${bar} v${height} h${-bar} Z`;
    return {
      ...entry,
      path,
      x,
      center: x + bar / 2,
      label: entry.label,
      aria: `${entry.label}，${props.format(entry.value)}`,
    };
  }),
);

const active = computed(() => (hover.value >= 0 ? bars.value[hover.value] : null));
const activeTop = computed(() => {
  if (!active.value) return 0;
  const ratio = Math.min(1, Math.max(0, active.value.value / max.value));
  return props.height - ratio * (props.height - 2);
});
</script>

<template>
  <div class="mini-bars">
    <div ref="root" class="chart" :style="{ height: `${height}px` }">
      <svg
        :viewBox="`0 0 ${width} ${height}`"
        :width="width"
        :height="height"
        role="img"
        :aria-label="label"
        @pointerleave="hover = -1"
      >
        <path
          v-for="(bar, index) in bars"
          :key="bar.label"
          :d="bar.path"
          class="bar"
          :class="{ highlight: bar.highlight || hover === index }"
          tabindex="0"
          role="img"
          :aria-label="bar.aria"
          @pointerenter="hover = index"
          @focus="hover = index"
          @blur="hover = -1"
        />
      </svg>
      <div
        v-if="active"
        class="chart-tip"
        :style="{ left: `${active.center}px`, top: `${activeTop}px` }"
      >
        <span>{{ active.label }}</span>
        <strong>{{ format(active.value) }}</strong>
      </div>
    </div>
    <div class="chart-foot">
      <button type="button" class="table-toggle" @click="showTable = !showTable">
        {{ showTable ? '收起数据' : '查看数据' }}
      </button>
    </div>
    <table v-if="showTable" class="chart-table">
      <thead>
        <tr>
          <th scope="col">日期</th>
          <th scope="col">数值</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="entry in values" :key="entry.label">
          <td>{{ entry.label }}</td>
          <td>{{ format(entry.value) }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.mini-bars {
  min-width: 0;
}
.chart {
  position: relative;
  width: 100%;
}
svg {
  display: block;
  overflow: visible;
}
.bar {
  fill: var(--accent);
  fill-opacity: 0.45;
  outline: none;
  transition: fill-opacity var(--duration-fast) ease;
}
.bar.highlight,
.bar:hover,
.bar:focus-visible {
  fill-opacity: 1;
}
.bar:focus-visible {
  stroke: var(--accent);
  stroke-width: 1.5;
}
.chart-tip {
  position: absolute;
  z-index: 3;
  display: flex;
  gap: 8px;
  padding: 3px 7px;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: var(--surface);
  box-shadow: var(--shadow-sm);
  color: var(--text-secondary);
  font-size: 11px;
  white-space: nowrap;
  pointer-events: none;
  transform: translate(-50%, calc(-100% - 6px));
}
.chart-tip strong {
  color: var(--text);
  font-variant-numeric: tabular-nums;
}
.chart-foot {
  display: flex;
  justify-content: flex-end;
  margin-top: 6px;
}
.table-toggle {
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--text-tertiary);
  font-size: 11px;
  text-decoration: underline;
  text-underline-offset: 2px;
}
.table-toggle:hover {
  color: var(--accent-text);
}
.chart-table {
  width: 100%;
  margin-top: 6px;
  border-collapse: collapse;
  font-size: 11.5px;
}
.chart-table th,
.chart-table td {
  padding: 4px 6px;
  border-bottom: 1px solid var(--divider);
  text-align: left;
}
.chart-table th {
  color: var(--text-tertiary);
  font-weight: 500;
}
.chart-table td:last-child,
.chart-table th:last-child {
  text-align: right;
  font-variant-numeric: tabular-nums;
}
</style>
