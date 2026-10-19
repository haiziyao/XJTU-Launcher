import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue';
import type { AppConfig } from './types';

const MIN_WIDTH = 176;
const MAX_WIDTH = 320;
const DEFAULT_WIDTH = 196;
const clamp = (width: number) => Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, Math.round(width)));

export function useSidebarResize(
  config: Ref<AppConfig>,
  loaded: Ref<boolean>,
  persist: (config: AppConfig) => Promise<void>,
) {
  const preview = ref<number | null>(null);
  const resizing = ref(false);
  const collapsed = computed(() => preview.value === null && config.value.sidebarCollapsed);
  const width = computed(() => preview.value ?? (collapsed.value ? 56 : config.value.sidebarWidth));
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
    preview.value = clamp(startWidth + event.clientX - startX);
  }
  function finish(event: PointerEvent) {
    if (event.pointerId !== pointer) return;
    const next = preview.value;
    cancel();
    if (next !== null)
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
    const next =
      event.key === 'Home'
        ? MIN_WIDTH
        : event.key === 'End'
          ? MAX_WIDTH
          : event.key === 'Enter'
            ? DEFAULT_WIDTH
            : clamp(
                (collapsed.value ? MIN_WIDTH : config.value.sidebarWidth) +
                  (event.key === 'ArrowLeft' ? -16 : 16),
              );
    void persist({ ...config.value, sidebarWidth: next, sidebarCollapsed: false });
  }
  onMounted(() => window.addEventListener('blur', cancel));
  onBeforeUnmount(() => {
    cancel();
    window.removeEventListener('blur', cancel);
  });
  return { width, collapsed, resizing, start, move, finish, cancel, reset, key };
}
