<script setup lang="ts">
/**
 * Square or round button that contains only an icon; the visible icon carries
 * no text, so `label` is required for the accessible name. Use UiButton for
 * anything with a text label. Variants and sizes are listed in
 * dev-docs/UI_COMPONENTS.md.
 */
withDefaults(
  defineProps<{
    label: string;
    variant?: 'quiet' | 'accent' | 'primary' | 'neutral' | 'danger' | 'inherit';
    size?: 'xs' | 'sm' | 'md';
    shape?: 'square' | 'circle';
    /** Toggled / current state, used by toolbar buttons such as the Agent panel. */
    active?: boolean;
  }>(),
  { variant: 'quiet', size: 'sm', shape: 'square', active: false },
);
</script>

<template>
  <button
    type="button"
    :aria-label="label"
    :class="[variant, size, { circle: shape === 'circle', active }]"
    class="ui-icon-button"
  >
    <slot />
  </button>
</template>

<style scoped>
/* One icon-button scale for the whole app: xs 20px (inline dismiss), sm 24px
   (row and field actions), md 28px (toolbars). */
.ui-icon-button {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  padding: 0;
  border: 0;
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-tertiary);
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease,
    transform var(--duration-fast) var(--ease-out);
}
.ui-icon-button.circle {
  border-radius: 50%;
}
.ui-icon-button.xs {
  width: 20px;
  height: 20px;
}
.ui-icon-button.sm {
  width: 24px;
  height: 24px;
}
.ui-icon-button.md {
  width: 28px;
  height: 28px;
}
.ui-icon-button:active:not(:disabled) {
  transform: scale(0.96);
}
.ui-icon-button.active {
  background: var(--surface-pressed);
  color: var(--text);
}
.quiet:hover:not(:disabled) {
  background: var(--surface-hover);
  color: var(--text);
}
.accent:hover:not(:disabled) {
  background: var(--accent-wash);
  color: var(--accent-text);
}
.accent.active {
  background: var(--accent-wash);
  color: var(--accent-text);
}
.danger:hover:not(:disabled) {
  background: var(--danger-soft);
  color: var(--danger);
}
/* Keeps the colour of the surrounding notice. */
.inherit {
  color: inherit;
}
.inherit:hover:not(:disabled) {
  background: var(--surface-hover);
}
.primary {
  background: var(--accent);
  color: var(--accent-contrast);
}
.primary:hover:not(:disabled) {
  background: var(--accent-hover);
}
.primary:disabled {
  background: var(--surface-sunken);
  color: var(--text-tertiary);
}
/* Filled but quiet, for a primary control in its stopped / inactive state. */
.neutral {
  background: var(--surface-sunken);
  color: var(--text);
}
.neutral:hover:not(:disabled) {
  background: var(--surface-pressed);
}
</style>
