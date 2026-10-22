import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue';
import { DEFAULT_SIDEBAR_WIDTH } from './domain';
import type { AppConfig } from './types';

const MIN_WIDTH = 130;
const MAX_WIDTH = 320;
const DEFAULT_WIDTH = DEFAULT_SIDEBAR_WIDTH;
const DEFAULT_COLLAPSED_WIDTH = 56;
const clamp = (width: number) => Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, Math.round(width)));

export function useSidebarResize(
  config: Ref<AppConfig>,
  loaded: Ref<boolean>,
  persist: (config: AppConfig) => Promise<void>,
  collapsedWidth = DEFAULT_COLLAPSED_WIDTH,
) {
  const preview = ref<number | null>(null);
  const resizing = ref(false);
  // Dragging below the minimum expanded width previews the collapsed rail, so a
  // single inward drag reaches the rail instead of stopping at the minimum.
  const collapsed = computed(() =>
    preview.value === null ? config.value.sidebarCollapsed : preview.value < MIN_WIDTH,
  );
  const width = computed(
    () => preview.value ?? (collapsed.value ? collapsedWidth : config.value.sidebarWidth),
  );
  const clampDrag = (value: number) =>
    Math.max(collapsedWidth, Math.min(MAX_WIDTH, Math.round(value)));
  let target: HTMLElement | null = null;
  let pointer: number | null = null;
  let startX = 0;
  let startWidth = 0;

  function cancel() {
    const previousTarget = target;
    const previousPointer = pointer;
    pointer = null;
    target = null;
    preview.value = null;
    resizing.value = false;
    if (previousPointer !== null && previousTarget?.hasPointerCapture(previousPointer))
      previousTarget.releasePointerCapture(previousPointer);
  }
  function start(event: PointerEvent) {
    if (!loaded.value || event.button !== 0 || pointer !== null) return;
    target = event.currentTarget as HTMLElement;
    pointer = event.pointerId;
    startX = event.clientX;
    startWidth = width.value;
    target.setPointerCapture(pointer);
    target.focus({ preventScroll: true });
    resizing.value = true;
    event.preventDefault();
  }
  function move(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    if (Math.abs(event.clientX - startX) < 3) {
      preview.value = null;
      return;
    }
    preview.value = clampDrag(startWidth + event.clientX - startX);
  }
  function finish(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    const next = preview.value;
    cancel();
    if (next === null) return;
    if (next < MIN_WIDTH) {
      // Releasing inside the rail zone collapses; the stored expanded width is kept.
      if (!config.value.sidebarCollapsed) void persist({ ...config.value, sidebarCollapsed: true });
      return;
    }
    void persist({ ...config.value, sidebarWidth: next, sidebarCollapsed: false });
  }
  function reset() {
    if (!loaded.value) return;
    cancel();
    void persist({ ...config.value, sidebarWidth: DEFAULT_WIDTH, sidebarCollapsed: false });
  }
  function key(event: KeyboardEvent) {
    if (!loaded.value) return;
    if (event.key === 'Escape') {
      cancel();
      return;
    }
    if (!['ArrowLeft', 'ArrowRight', 'Home', 'End', 'Enter'].includes(event.key)) return;
    event.preventDefault();
    const expanded = config.value.sidebarCollapsed ? MIN_WIDTH : width.value;
    if (event.key === 'Home') {
      if (!config.value.sidebarCollapsed) void persist({ ...config.value, sidebarCollapsed: true });
      return;
    }
    if (event.key === 'ArrowLeft') {
      if (config.value.sidebarCollapsed) return;
      if (expanded - 16 < MIN_WIDTH) {
        void persist({ ...config.value, sidebarCollapsed: true });
        return;
      }
    }
    const next =
      event.key === 'End'
        ? MAX_WIDTH
        : event.key === 'Enter'
          ? DEFAULT_WIDTH
          : clamp(
              event.key === 'ArrowLeft'
                ? expanded - 16
                : config.value.sidebarCollapsed
                  ? MIN_WIDTH
                  : expanded + 16,
            );
    void persist({ ...config.value, sidebarWidth: next, sidebarCollapsed: false });
  }
  onMounted(() => window.addEventListener('blur', cancel));
  onBeforeUnmount(() => {
    cancel();
    window.removeEventListener('blur', cancel);
  });
  return {
    width,
    collapsed,
    collapsedWidth,
    resizing,
    start,
    move,
    finish,
    cancel,
    reset,
    key,
  };
}
