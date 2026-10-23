<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, reactive, ref } from 'vue';
import { ZoomIn, ZoomOut } from 'lucide-vue-next';
import type { ImageCrop, PersonalImageKind } from '../types';

import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
const props = defineProps<{ src: string; kind: PersonalImageKind; busy: boolean }>();
const emit = defineEmits<{ confirm: [crop: ImageCrop]; cancel: [] }>();

const PAD = 28;
const MAX_ZOOM = 4;
// Wallpapers follow the window shape so the saved image fills it without further cropping.
const frame = (() => {
  if (props.kind === 'avatar') return { w: 260, h: 260 };
  const ratio = window.innerHeight / Math.max(window.innerWidth, 1);
  return { w: 460, h: Math.round(460 * Math.min(Math.max(ratio, 0.4), 1)) };
})();
const natural = reactive({ w: 0, h: 0 });
const zoom = ref(1);
const offset = reactive({ x: 0, y: 0 });
const dialog = ref<HTMLElement | null>(null);
let drag: { id: number; x: number; y: number; ox: number; oy: number } | null = null;

const scale = computed(() =>
  natural.w ? Math.max(frame.w / natural.w, frame.h / natural.h) * zoom.value : 0,
);
const imageStyle = computed(() => ({
  width: `${natural.w * scale.value}px`,
  height: `${natural.h * scale.value}px`,
  transform: `translate(${PAD + offset.x}px, ${PAD + offset.y}px)`,
}));

// Keep the image covering the frame so the crop never includes empty space.
function place(x: number, y: number) {
  offset.x = Math.min(0, Math.max(frame.w - natural.w * scale.value, x));
  offset.y = Math.min(0, Math.max(frame.h - natural.h * scale.value, y));
}
function loaded(event: Event) {
  const img = event.target as HTMLImageElement;
  natural.w = img.naturalWidth;
  natural.h = img.naturalHeight;
  zoom.value = 1;
  place((frame.w - natural.w * scale.value) / 2, (frame.h - natural.h * scale.value) / 2);
}
// Zoom around the frame centre, which is where the user is looking.
function setZoom(value: number) {
  if (!natural.w) return;
  const before = scale.value;
  const cx = (frame.w / 2 - offset.x) / before;
  const cy = (frame.h / 2 - offset.y) / before;
  zoom.value = Math.min(MAX_ZOOM, Math.max(1, value));
  place(frame.w / 2 - cx * scale.value, frame.h / 2 - cy * scale.value);
}
function wheel(event: WheelEvent) {
  setZoom(zoom.value * Math.exp(-event.deltaY * 0.002));
}
function start(event: PointerEvent) {
  if (event.button !== 0 || !natural.w) return;
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  drag = { id: event.pointerId, x: event.clientX, y: event.clientY, ox: offset.x, oy: offset.y };
}
function move(event: PointerEvent) {
  if (drag?.id !== event.pointerId) return;
  place(drag.ox + event.clientX - drag.x, drag.oy + event.clientY - drag.y);
}
function end(event: PointerEvent) {
  if (drag?.id === event.pointerId) drag = null;
}
function nudge(event: KeyboardEvent) {
  const step = event.shiftKey ? 40 : 10;
  const delta: Record<string, [number, number]> = {
    ArrowLeft: [step, 0],
    ArrowRight: [-step, 0],
    ArrowUp: [0, step],
    ArrowDown: [0, -step],
  };
  const d = delta[event.key];
  if (!d) return;
  event.preventDefault();
  place(offset.x + d[0], offset.y + d[1]);
}
function confirm() {
  if (!natural.w || props.busy) return;
  const w = natural.w * scale.value;
  const h = natural.h * scale.value;
  emit('confirm', { x: -offset.x / w, y: -offset.y / h, width: frame.w / w, height: frame.h / h });
}
function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape' && !props.busy) emit('cancel');
}
onMounted(async () => {
  window.addEventListener('keydown', onKey);
  await nextTick();
  dialog.value?.querySelector<HTMLElement>('.crop-stage')?.focus();
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <Teleport to="body">
    <div class="crop-backdrop" @click.self="!busy && emit('cancel')">
      <div
        ref="dialog"
        class="crop-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="crop-title"
      >
        <header class="crop-header">
          <h2 id="crop-title">{{ kind === 'avatar' ? '调整头像' : '调整壁纸' }}</h2>
          <p>拖动图片调整位置，滚轮或滑块缩放。</p>
        </header>
        <div
          class="crop-stage"
          :class="{ round: kind === 'avatar' }"
          :style="{ width: `${frame.w + PAD * 2}px`, height: `${frame.h + PAD * 2}px` }"
          tabindex="0"
          aria-label="裁剪区域，可用方向键移动"
          @pointerdown="start"
          @pointermove="move"
          @pointerup="end"
          @pointercancel="end"
          @wheel.prevent="wheel"
          @keydown="nudge"
        >
          <img :src="src" alt="" draggable="false" :style="imageStyle" @load="loaded" />
          <span
            class="crop-frame"
            :style="{
              left: `${PAD}px`,
              top: `${PAD}px`,
              width: `${frame.w}px`,
              height: `${frame.h}px`,
            }"
            aria-hidden="true"
          ></span>
        </div>
        <div class="crop-zoom">
          <UiIconButton label="缩小" size="sm" :disabled="zoom <= 1" @click="setZoom(zoom / 1.2)">
            <ZoomOut :size="14" />
          </UiIconButton>
          <input
            type="range"
            min="1"
            :max="MAX_ZOOM"
            step="0.01"
            :value="zoom"
            aria-label="缩放"
            @input="setZoom(Number(($event.target as HTMLInputElement).value))"
          />
          <UiIconButton
            label="放大"
            size="sm"
            :disabled="zoom >= MAX_ZOOM"
            @click="setZoom(zoom * 1.2)"
          >
            <ZoomIn :size="14" />
          </UiIconButton>
        </div>
        <footer class="crop-actions">
          <UiButton size="sm" :disabled="busy" @click="emit('cancel')">取消</UiButton>
          <UiButton variant="primary" size="sm" :disabled="busy || !natural.w" @click="confirm">
            {{ busy ? '正在保存…' : '确定' }}
          </UiButton>
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.crop-backdrop {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: grid;
  place-items: center;
  background: rgba(15, 23, 42, 0.32);
}
.crop-dialog {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 14px;
  max-width: calc(100vw - 48px);
  padding: 20px 22px 18px;
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.crop-header {
  align-self: stretch;
}
.crop-header h2 {
  font-size: 14px;
  font-weight: 600;
}
.crop-header p {
  margin-top: 2px;
  color: var(--text-tertiary);
  font-size: 12px;
}
.crop-stage {
  position: relative;
  max-width: 100%;
  overflow: hidden;
  border-radius: var(--radius);
  background: var(--surface-sunken);
  cursor: grab;
  touch-action: none;
  user-select: none;
}
.crop-stage:active {
  cursor: grabbing;
}
.crop-stage:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.crop-stage img {
  position: absolute;
  top: 0;
  left: 0;
  max-width: none;
  pointer-events: none;
}
/* The frame's huge shadow dims everything outside the crop. */
.crop-frame {
  position: absolute;
  border: 1px solid rgba(255, 255, 255, 0.9);
  box-shadow: 0 0 0 9999px rgba(15, 23, 42, 0.45);
  pointer-events: none;
}
.crop-stage.round .crop-frame {
  border-radius: 50%;
}
.crop-zoom {
  display: flex;
  align-items: center;
  gap: 8px;
  width: 260px;
}
.crop-zoom input {
  flex: 1;
  accent-color: var(--accent);
}
.crop-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  align-self: stretch;
}
</style>
