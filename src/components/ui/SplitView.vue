<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { ArrowLeft } from 'lucide-vue-next';
import UiButton from './UiButton.vue';

/**
 * List + detail column (LAYOUT §3.1–3.2). The split is chosen by the measured
 * content width, not the window: ≥1000 shows both panes with a draggable
 * separator, 760–999 fixes the list at 280px, and below that one pane at a time
 * with a 返回列表 button. The caller owns the selected row and the width value.
 */
const props = withDefaults(
  defineProps<{
    width?: number;
    min?: number;
    max?: number;
    /** Single-column mode: show the detail pane instead of the list. */
    detailOpen?: boolean;
    listLabel?: string;
    detailLabel?: string;
  }>(),
  {
    width: 320,
    min: 280,
    max: 420,
    detailOpen: false,
    listLabel: '列表',
    detailLabel: '详情',
  },
);
const emit = defineEmits<{ 'update:width': [value: number]; back: [] }>();

const root = ref<HTMLElement | null>(null);
const measured = ref(0);
const dragging = ref(false);
const dragWidth = ref(props.width);
let observer: ResizeObserver | undefined;
let startX = 0;
let startWidth = 0;

onMounted(() => {
  if (!root.value) return;
  observer = new ResizeObserver((entries) => {
    measured.value = entries[0]?.contentRect.width ?? 0;
  });
  observer.observe(root.value);
  measured.value = root.value.getBoundingClientRect().width;
});
onBeforeUnmount(() => observer?.disconnect());

const clamp = (value: number, low: number, high: number) => Math.min(high, Math.max(low, value));

const mode = computed(() => {
  if (measured.value >= 1000) return 'split';
  if (measured.value >= 760) return 'narrow';
  return 'single';
});
/** The detail pane keeps at least 360px, so the list gives way first. */
const ceiling = computed(() => Math.min(props.max, Math.max(props.min, measured.value - 360)));
const listWidth = computed(() => {
  const value = dragging.value ? dragWidth.value : props.width;
  if (mode.value === 'narrow') return 280;
  return clamp(value, props.min, ceiling.value);
});
const paneStyle = computed(() =>
  mode.value === 'single' ? undefined : { '--split-list-width': `${listWidth.value}px` },
);

function down(event: PointerEvent) {
  if (mode.value !== 'split') return;
  startX = event.clientX;
  startWidth = listWidth.value;
  dragWidth.value = startWidth;
  dragging.value = true;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
}
function move(event: PointerEvent) {
  if (!dragging.value) return;
  dragWidth.value = clamp(startWidth + event.clientX - startX, props.min, ceiling.value);
}
function up(event: PointerEvent) {
  if (!dragging.value) return;
  dragging.value = false;
  emit('update:width', dragWidth.value);
  (event.currentTarget as HTMLElement).releasePointerCapture?.(event.pointerId);
}
function key(event: KeyboardEvent) {
  const step = event.shiftKey ? 40 : 16;
  let next = props.width;
  if (event.key === 'ArrowLeft') next = props.width - step;
  else if (event.key === 'ArrowRight') next = props.width + step;
  else if (event.key === 'Home') next = props.min;
  else if (event.key === 'End') next = ceiling.value;
  else return;
  event.preventDefault();
  emit('update:width', clamp(next, props.min, ceiling.value));
}
</script>

<template>
  <div ref="root" class="split-view" :class="`mode-${mode}`" :style="paneStyle">
    <section
      v-show="mode !== 'single' || !detailOpen"
      class="split-pane split-list"
      :aria-label="listLabel"
    >
      <slot name="list" />
    </section>
    <div
      v-if="mode === 'split'"
      class="split-resizer"
      role="separator"
      aria-orientation="vertical"
      aria-label="调整列表宽度"
      :aria-valuemin="min"
      :aria-valuemax="Math.round(ceiling)"
      :aria-valuenow="Math.round(listWidth)"
      tabindex="0"
      @pointerdown="down"
      @pointermove="move"
      @pointerup="up"
      @pointercancel="up"
      @keydown="key"
    ></div>
    <section
      v-show="mode !== 'single' || detailOpen"
      class="split-pane split-detail"
      :aria-label="detailLabel"
    >
      <div v-if="mode === 'single'" class="split-back">
        <UiButton variant="ghost" size="sm" @click="emit('back')">
          <ArrowLeft :size="14" /><span>返回列表</span>
        </UiButton>
      </div>
      <slot name="detail" />
    </section>
  </div>
</template>

<style scoped>
.split-view {
  display: flex;
  align-items: stretch;
  min-height: 0;
  height: 100%;
}
.split-pane {
  min-width: 0;
  min-height: 0;
  animation: split-fade var(--duration-fast) var(--ease-out);
}
.split-list {
  flex: 0 0 var(--split-list-width, 320px);
  overflow: auto;
  padding-right: 18px;
}
.mode-narrow .split-list {
  padding-right: 14px;
}
.split-detail {
  flex: 1 1 auto;
  min-width: 360px;
  overflow: auto;
  padding-left: 18px;
}
.mode-narrow .split-detail {
  padding-left: 14px;
}
/* 1px hairline with a wider hit area; hover thickens it to the accent. */
.split-resizer {
  position: relative;
  z-index: 1;
  flex: 0 0 9px;
  margin: 0 -4px;
  cursor: col-resize;
  touch-action: none;
}
.split-resizer::before {
  content: '';
  position: absolute;
  top: 0;
  bottom: 0;
  left: 4px;
  width: 1px;
  background: var(--border);
  transition:
    width var(--duration-fast) ease,
    background var(--duration-fast) ease;
}
.split-resizer:hover::before,
.split-resizer:focus-visible::before {
  left: 3px;
  width: 2px;
  background: var(--accent);
}
.split-resizer:focus-visible {
  outline: none;
}
.mode-single {
  display: block;
}
.mode-single .split-pane {
  width: 100%;
  height: 100%;
  padding: 0;
  overflow: auto;
}
.split-back {
  margin-bottom: 8px;
}
@keyframes split-fade {
  from {
    opacity: 0;
  }
  to {
    opacity: 1;
  }
}
@media (prefers-reduced-motion: reduce) {
  .split-pane {
    animation: none;
  }
  .split-resizer::before {
    transition: none;
  }
}
</style>
