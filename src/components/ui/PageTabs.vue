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
  gap: 10px;
  margin-bottom: 30px;
  padding: 0 0 14px;
  border-bottom: 1px solid var(--border);
}
button {
  padding: 6px 12px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  transition:
    background var(--duration-fast) ease,
    color var(--duration) ease;
}
button:hover:not(.active) {
  background: var(--accent-wash);
  color: var(--text);
}
button.active {
  background: var(--accent-soft);
  color: var(--accent);
}
button:active {
  transform: scale(0.98);
}
</style>
