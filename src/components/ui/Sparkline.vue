<script lang="ts">
export interface SparkPoint {
  label: string;
  value: number;
}
</script>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';

/**
 * Single-series line chart for grade trends (LAYOUT §6.2). One axis, 2px accent
 * line, 8px markers, grid in --divider, axis text at 11px in a text token. A
 * crosshair and tooltip follow hover and keyboard focus; “查看数据” reveals the
 * numbers as a table.
 */
const props = withDefaults(
  defineProps<{
    points: SparkPoint[];
    height?: number;
    label?: string;
    format?: (value: number) => string;
  }>(),
  { height: 128, label: '折线图', format: (value: number) => String(value) },
);

const root = ref<HTMLElement | null>(null);
const width = ref(320);
const active = ref(-1);
const showTable = ref(false);
let observer: ResizeObserver | undefined;

onMounted(() => {
  if (!root.value) return;
  observer = new ResizeObserver((entries) => {
    width.value = Math.max(120, entries[0]?.contentRect.width ?? 320);
  });
  observer.observe(root.value);
  width.value = Math.max(120, root.value.getBoundingClientRect().width);
});
onBeforeUnmount(() => observer?.disconnect());

const PAD = { top: 14, right: 40, bottom: 20, left: 4 };

const domain = computed(() => {
  const values = props.points.map((point) => point.value);
  const low = Math.min(...values);
  const high = Math.max(...values);
  if (low === high) return { low: low - 1, high: high + 1 };
  const margin = (high - low) * 0.15;
  return { low: low - margin, high: high + margin };
});

const plot = computed(() => ({
  width: Math.max(20, width.value - PAD.left - PAD.right),
  height: Math.max(20, props.height - PAD.top - PAD.bottom),
}));

const scaleX = (index: number) =>
  props.points.length <= 1
    ? PAD.left + plot.value.width / 2
    : PAD.left + (index / (props.points.length - 1)) * plot.value.width;
const scaleY = (value: number) =>
  PAD.top +
  plot.value.height -
  ((value - domain.value.low) / (domain.value.high - domain.value.low)) * plot.value.height;

const line = computed(() =>
  props.points
    .map((point, index) => `${index ? 'L' : 'M'}${scaleX(index)},${scaleY(point.value)}`)
    .join(' '),
);
const nodes = computed(() =>
  props.points.map((point, index) => ({
    ...point,
    x: scaleX(index),
    y: scaleY(point.value),
    aria: `${point.label}，${props.format(point.value)}`,
  })),
);
const current = computed(() => (active.value >= 0 ? nodes.value[active.value] : null));
const showMarkers = computed(() => props.points.length <= 12);
const grid = computed(() => [0, 0.5, 1].map((ratio) => PAD.top + ratio * plot.value.height));
const axisLabels = computed(() => [
  { y: scaleY(domain.value.high), text: props.format(domain.value.high) },
  { y: scaleY(domain.value.low), text: props.format(domain.value.low) },
]);
</script>

<template>
  <div class="sparkline">
    <div v-if="points.length < 2" class="spark-empty">数据不足，暂不显示趋势。</div>
    <template v-else>
      <div ref="root" class="chart" :style="{ height: `${height}px` }">
        <svg
          :viewBox="`0 0 ${width} ${height}`"
          :width="width"
          :height="height"
          role="img"
          :aria-label="label"
          @pointerleave="active = -1"
        >
          <line
            v-for="(y, index) in grid"
            :key="index"
            :x1="PAD.left"
            :x2="width - PAD.right"
            :y1="y"
            :y2="y"
            class="grid"
          />
          <text
            v-for="axis in axisLabels"
            :key="axis.text"
            :x="width - PAD.right + 6"
            :y="axis.y + 3.5"
            class="axis-text"
          >
            {{ axis.text }}
          </text>
          <path :d="line" class="line" />
          <circle
            v-for="(node, index) in nodes"
            v-if="showMarkers"
            :key="node.label"
            :cx="node.x"
            :cy="node.y"
            :r="4"
            class="marker"
            :class="{ active: active === index }"
            tabindex="0"
            role="img"
            :aria-label="node.aria"
            @pointerenter="active = index"
            @focus="active = index"
            @blur="active = -1"
          />
          <line
            v-if="current"
            :x1="current.x"
            :x2="current.x"
            :y1="PAD.top"
            :y2="height - PAD.bottom"
            class="crosshair"
          />
        </svg>
        <div
          v-if="current"
          class="chart-tip"
          :style="{ left: `${current.x}px`, top: `${current.y}px` }"
        >
          <span>{{ current.label }}</span>
          <strong>{{ format(current.value) }}</strong>
        </div>
        <div class="axis-x">
          <span>{{ points[0].label }}</span>
          <span>{{ points[points.length - 1].label }}</span>
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
            <th scope="col">学期</th>
            <th scope="col">数值</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="point in points" :key="point.label">
            <td>{{ point.label }}</td>
            <td>{{ format(point.value) }}</td>
          </tr>
        </tbody>
      </table>
    </template>
  </div>
</template>

<style scoped>
.sparkline {
  min-width: 0;
}
.spark-empty {
  padding: 18px 0;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.chart {
  position: relative;
  width: 100%;
}
svg {
  display: block;
  overflow: visible;
}
.grid {
  stroke: var(--divider);
  stroke-width: 1;
}
.axis-text {
  fill: var(--text-tertiary);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.line {
  fill: none;
  stroke: var(--accent);
  stroke-width: 2;
  stroke-linejoin: round;
  stroke-linecap: round;
}
.marker {
  fill: var(--surface);
  stroke: var(--accent);
  stroke-width: 2;
  outline: none;
  transition: fill var(--duration-fast) ease;
}
.marker.active,
.marker:focus-visible {
  fill: var(--accent);
}
.crosshair {
  stroke: var(--border-strong);
  stroke-width: 1;
  stroke-dasharray: 3 3;
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
  transform: translate(-50%, calc(-100% - 8px));
}
.chart-tip strong {
  color: var(--text);
  font-variant-numeric: tabular-nums;
}
.axis-x {
  display: flex;
  justify-content: space-between;
  margin-top: 2px;
  padding: 0 4px 0 4px;
  color: var(--text-tertiary);
  font-size: 11px;
}
.chart-foot {
  display: flex;
  justify-content: flex-end;
  margin-top: 4px;
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
