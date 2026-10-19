import { nextTick, onBeforeUnmount, onMounted, ref, watch, type Ref } from 'vue';

/**
 * Positions one highlight behind the active item of a vertical list, so selection slides
 * between items instead of jumping. Items carry `data-indicator="<key>"` and the container
 * must be their offset parent (position: relative).
 */
export function useSlidingIndicator(
  container: Ref<HTMLElement | null>,
  active: () => string | undefined,
) {
  const style = ref<Record<string, string>>({ opacity: '0' });
  const animated = ref(false);
  let visible = false;
  let frame = 0;
  let resizeObserver: ResizeObserver | undefined;
  let mutationObserver: MutationObserver | undefined;

  function update() {
    const root = container.value;
    const key = active();
    const item =
      root && key ? root.querySelector<HTMLElement>(`[data-indicator="${CSS.escape(key)}"]`) : null;
    if (!item) {
      visible = false;
      style.value = { ...style.value, opacity: '0' };
      return;
    }
    // Appear in place when nothing was highlighted before; slide between items otherwise.
    if (!visible) {
      animated.value = false;
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        frame = requestAnimationFrame(() => (animated.value = true));
      });
    }
    visible = true;
    style.value = {
      opacity: '1',
      height: `${item.offsetHeight}px`,
      transform: `translateY(${item.offsetTop}px)`,
    };
  }

  watch(active, () => void nextTick(update));
  onMounted(() => {
    update();
    if (!container.value) return;
    resizeObserver = new ResizeObserver(update);
    resizeObserver.observe(container.value);
    mutationObserver = new MutationObserver(update);
    mutationObserver.observe(container.value, { childList: true, subtree: true });
  });
  onBeforeUnmount(() => {
    cancelAnimationFrame(frame);
    resizeObserver?.disconnect();
    mutationObserver?.disconnect();
  });
  return { style, animated, update };
}
