<script setup lang="ts">
import { computed } from 'vue';
import PetArt from './PetArt.vue';
import type { PetId } from '../pets';
import type { PetState } from '../types';

const props = defineProps<{ state: PetState; message: string; variant: PetId }>();
const emit = defineEmits<{ check: [] }>();

const label = computed(() => {
  switch (props.state) {
    case 'checking':
      return '正在检查校园网络';
    case 'ok':
      return props.message || '校园网络正常';
    case 'warn':
      return props.message || '校园网络异常';
    default:
      return '校园网络';
  }
});
const hint = computed(() => (props.state === 'idle' ? '点一下，检查校园网络' : label.value));
</script>

<template>
  <button
    type="button"
    class="sidebar-pet"
    :class="`is-${state}`"
    :title="hint"
    :aria-label="`${label}，点击检查`"
    :aria-busy="state === 'checking'"
    :disabled="state === 'checking'"
    @click="emit('check')"
  >
    <span class="pet-slot">
      <PetArt :variant="variant" />
      <span class="pet-think" aria-hidden="true"><i></i><i></i><i></i></span>
      <span class="pet-status" role="status">{{ state === 'idle' ? '' : label }}</span>
    </span>
  </button>
</template>

<style scoped>
.sidebar-pet {
  display: grid;
  place-items: center;
  width: 100%;
  padding: 4px 0 2px;
  border: 0;
  border-radius: var(--radius);
  background: transparent;
  color: inherit;
  cursor: pointer;
}
.sidebar-pet:disabled {
  cursor: default;
  opacity: 1;
}
.pet-slot {
  position: relative;
  display: block;
  width: 56px;
  height: 56px;
  transition: transform var(--duration) var(--ease-out);
}
.sidebar-pet:hover:not(:disabled) .pet-slot {
  transform: translateY(-1px) scale(1.03);
}
.sidebar-pet:active:not(:disabled) .pet-slot {
  transform: scale(0.97);
}
.sidebar-pet:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}

/* Checking: drifting thought dots beside the head. */
.pet-think {
  position: absolute;
  top: 4px;
  right: 3px;
  display: flex;
  gap: 3px;
  opacity: 0;
  transition: opacity var(--duration-fast) ease;
}
.pet-think i {
  width: 3.5px;
  height: 3.5px;
  border-radius: 50%;
  background: var(--accent);
  animation: pet-dot 900ms ease-in-out infinite;
}
.pet-think i:nth-child(2) {
  animation-delay: 140ms;
}
.pet-think i:nth-child(3) {
  animation-delay: 280ms;
}

.pet-status {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  padding: 0;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}

@keyframes pet-dot {
  0%,
  100% {
    transform: translateY(0);
    opacity: 0.45;
  }
  50% {
    transform: translateY(-3px);
    opacity: 1;
  }
}
</style>
