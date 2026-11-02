<script setup lang="ts">
/**
 * Row inside a list + detail pane (LAYOUT §4.2). One line is 40px, a row with a
 * subtitle is 56px. Selected rows use --nav-active; the unread dot is a second,
 * non-colour signal next to the title.
 */
withDefaults(
  defineProps<{
    title: string;
    subtitle?: string;
    selected?: boolean;
    disabled?: boolean;
    unread?: boolean;
  }>(),
  { subtitle: '', selected: false, disabled: false, unread: false },
);
const emit = defineEmits<{ select: [] }>();
</script>

<template>
  <button
    type="button"
    class="list-item"
    :class="{ selected, 'has-subtitle': subtitle || $slots.subtitle }"
    role="option"
    :aria-selected="selected"
    :disabled="disabled"
    @click="emit('select')"
  >
    <span v-if="$slots.leading" class="list-leading"><slot name="leading" /></span>
    <span class="list-copy">
      <span class="list-title">
        <span class="list-title-text">{{ title }}</span>
        <span v-if="unread" class="list-unread"><span class="visually-hidden">未读</span></span>
      </span>
      <span v-if="subtitle || $slots.subtitle" class="list-subtitle">
        <slot name="subtitle">{{ subtitle }}</slot>
      </span>
    </span>
    <span v-if="$slots.meta" class="list-meta"><slot name="meta" /></span>
    <span v-if="$slots.status" class="list-status"><slot name="status" /></span>
  </button>
</template>

<style scoped>
.list-item {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 100%;
  min-height: 40px;
  padding: 0 12px;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text);
  text-align: left;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}
.list-item.has-subtitle {
  min-height: 56px;
  padding-top: 8px;
  padding-bottom: 8px;
}
.list-item:hover:not(:disabled):not(.selected) {
  background: var(--surface-hover);
}
.list-item:active:not(:disabled) {
  background: var(--surface-pressed);
}
.list-item.selected {
  background: var(--nav-active);
}
.list-item.selected .list-title-text {
  color: var(--nav-active-text);
  font-weight: 600;
}
.list-item:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: -2px;
}
.list-leading {
  display: flex;
  flex-shrink: 0;
}
.list-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.list-title {
  display: flex;
  align-items: center;
  gap: 6px;
  min-width: 0;
}
.list-title-text {
  overflow: hidden;
  font-size: 13px;
  font-weight: 500;
  line-height: 1.4;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.list-unread {
  display: inline-flex;
  flex-shrink: 0;
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}
.list-subtitle {
  overflow: hidden;
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.4;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.list-meta {
  flex-shrink: 0;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.list-status {
  display: flex;
  flex-shrink: 0;
  align-items: center;
}
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}
</style>
