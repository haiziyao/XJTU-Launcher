<script setup lang="ts">
defineProps<{ checked: boolean; label: string; disabled?: boolean }>();
const emit = defineEmits<{ toggle: [] }>();
</script>

<template>
  <button
    class="ui-switch"
    type="button"
    role="switch"
    :aria-checked="checked"
    :aria-label="label"
    :disabled="disabled"
    @click="emit('toggle')"
  >
    <span class="ui-switch-thumb" />
  </button>
</template>
