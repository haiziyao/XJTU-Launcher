<script setup lang="ts">
const props = defineProps<{
  modelValue: string;
  tabs: readonly { id: string; title: string }[];
  label: string;
  /** Tighter tabs for in-page filters that carry counts or many entries. */
  compact?: boolean;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();
function key(event: KeyboardEvent, index: number) {
  const next =
    event.key === 'ArrowRight'
      ? (index + 1) % props.tabs.length
      : event.key === 'ArrowLeft'
        ? (index + props.tabs.length - 1) % props.tabs.length
        : event.key === 'Home'
          ? 0
          : event.key === 'End'
            ? props.tabs.length - 1
            : -1;
  if (next < 0) return;
  event.preventDefault();
  emit('update:modelValue', props.tabs[next].id);
  (event.currentTarget as HTMLElement).parentElement?.querySelectorAll('button')[next]?.focus();
}
</script>
<template>
  <div class="segmented-tabs" :class="{ compact }" role="tablist" :aria-label="label">
    <button
      v-for="(tab, index) in tabs"
      :key="tab.id"
      role="tab"
      :aria-selected="modelValue === tab.id"
      :tabindex="modelValue === tab.id ? 0 : -1"
      :class="{ active: modelValue === tab.id }"
      @click="emit('update:modelValue', tab.id)"
      @keydown="key($event, index)"
    >
      {{ tab.title }}
    </button>
  </div>
</template>
<style scoped>
.segmented-tabs {
  display: inline-flex;
  padding: 3px;
  gap: 2px;
  border-radius: var(--radius);
  background: var(--control-sunken);
  margin-bottom: 30px;
}
button {
  min-width: 112px;
  border: 0;
  border-radius: var(--radius-sm);
  padding: 6px 18px;
  background: transparent;
  color: var(--text-secondary);
  transition:
    background var(--duration) ease,
    color var(--duration) ease,
    box-shadow var(--duration) ease;
}
button:hover:not(.active):not(:disabled) {
  background: var(--accent-wash);
  color: var(--text);
}
button.active {
  background: var(--surface);
  color: var(--text);
  box-shadow: var(--shadow-xs);
  font-weight: 600;
}
button:active {
  transform: scale(0.98);
}
/* Compact tabs sit inside a page section: no fixed width, no trailing gap,
   and they scroll sideways instead of squeezing when there are many. */
.segmented-tabs.compact {
  max-width: 100%;
  margin-bottom: 0;
  overflow-x: auto;
  scrollbar-width: none;
}
.segmented-tabs.compact::-webkit-scrollbar {
  display: none;
}
.compact button {
  min-width: 0;
  padding: 5px 12px;
  font-size: 12.5px;
  white-space: nowrap;
}
</style>
