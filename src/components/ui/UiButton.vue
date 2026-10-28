<script setup lang="ts">
/**
 * Standard action button. Wraps the shared `.btn` styles in ui.css so pages do
 * not repeat class strings; see dev-docs/UI_COMPONENTS.md for the variant table.
 * The native <button> attributes (type, disabled, aria-*, @click) fall through.
 */
withDefaults(
  defineProps<{
    variant?: 'secondary' | 'primary' | 'tinted' | 'ghost';
    size?: 'md' | 'sm';
    danger?: boolean;
  }>(),
  { variant: 'secondary', size: 'md', danger: false },
);
</script>

<template>
  <button
    :class="[
      'btn',
      variant === 'secondary' ? null : variant,
      size === 'sm' ? 'small' : null,
      danger ? 'danger' : null,
    ]"
  >
    <slot />
  </button>
</template>
