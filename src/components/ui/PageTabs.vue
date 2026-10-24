<script setup lang="ts">
/**
 * Page-level tab bar: transparent, one hairline underneath, the active tab in
 * the accent tint. Use it directly below a PageHeader to switch whole pages or
 * large sections. In-page filters that sit inside a section stay on
 * SegmentedTabs, which keeps the filled pill container.
 */
const props = defineProps<{
  modelValue: string;
  tabs: readonly { id: string; title: string }[];
  label: string;
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
  <div class="page-tabs" role="tablist" :aria-label="label">
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
.page-tabs {
  display: flex;
  flex-wrap: wrap;
  align-items: flex-end;
  gap: 0;
  margin-bottom: 26px;
  padding: 0;
  border-bottom: 1px solid var(--border);
}
button {
  position: relative;
  margin-bottom: -1px;
  padding: 7px 16px;
  border: 1px solid transparent;
  border-bottom: 0;
  border-radius: 9px 9px 0 0;
  background: transparent;
  color: var(--text-secondary);
  font-weight: 500;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration-fast) ease,
    color var(--duration) ease;
}
button:hover:not(.active) {
  background: var(--surface-hover);
  color: var(--text);
}
/* Browser-style selection: the active tab sits on the hairline like a tab
   attached to the page, with a short accent strip along its top edge. */
button.active {
  background: var(--surface);
  border-color: var(--border);
  color: var(--text);
  font-weight: 600;
  z-index: 1;
}
button.active::before {
  content: '';
  position: absolute;
  left: -1px;
  right: -1px;
  top: -1px;
  height: 2px;
  border-radius: 9px 9px 0 0;
  background: var(--accent);
}
</style>
