<script setup lang="ts">
import { computed } from 'vue';
import type { PetState } from '../types';

const props = defineProps<{ state: PetState; message: string }>();
const emit = defineEmits<{ check: [] }>();

const label = computed(() => {
  switch (props.state) {
    case 'checking':
      return '正在检查校园会话';
    case 'ok':
      return props.message || '校园会话正常';
    case 'warn':
      return props.message || '校园会话异常';
    default:
      return '校园会话';
  }
});
const hint = computed(() => (props.state === 'idle' ? '点一下，检查校园会话' : label.value));
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
      <!-- A soft plush that takes its colours from the theme accent. -->
      <svg class="pet-art" viewBox="0 0 64 64" aria-hidden="true">
        <ellipse class="pet-shadow" cx="32" cy="56.5" rx="13" ry="3.2" />
        <g class="pet-body">
          <path class="pet-sprout-line" d="M32 16c0-4 1.4-6.8 3.8-8.6" />
          <path class="pet-sprout-leaf" d="M35.4 8.6c3.8-1.6 7-.6 8.2 1.8-2.4 2.4-6 2.2-8.2-1.8Z" />
          <circle class="pet-arm" cx="10.5" cy="41" r="3.6" />
          <circle class="pet-arm" cx="53.5" cy="41" r="3.6" />
          <path
            class="pet-shape"
            d="M32 15C45.8 15 55 24.6 55 36.5S45.8 54 32 54 9 48.4 9 36.5 18.2 15 32 15Z"
          />
          <g class="pet-eyes">
            <circle cx="24.5" cy="34" r="2.4" class="pet-ink" />
            <circle cx="39.5" cy="34" r="2.4" class="pet-ink" />
          </g>
          <g class="pet-eyes-sad">
            <path d="m22.4 32 4.2 4.2M26.6 32l-4.2 4.2" />
            <path d="m37.4 32 4.2 4.2M41.6 32l-4.2 4.2" />
          </g>
          <path class="pet-mouth" d="M29.6 40.6q2.4 2.2 4.8 0" />
          <path class="pet-mouth-sad" d="M29.6 42.4q2.4-2.2 4.8 0" />
          <ellipse class="pet-blush" cx="18.5" cy="39.5" rx="3.2" ry="1.9" />
          <ellipse class="pet-blush" cx="45.5" cy="39.5" rx="3.2" ry="1.9" />
        </g>
        <path
          class="pet-spark pet-spark-a"
          d="M50 10.5 51.5 14.5 55.5 16 51.5 17.5 50 21.5 48.5 17.5 44.5 16 48.5 14.5Z"
        />
        <path
          class="pet-spark pet-spark-b"
          d="M15 19 16 21.5 18.5 22.5 16 23.5 15 26 14 23.5 11.5 22.5 14 21.5Z"
        />
      </svg>
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
  padding: 10px 0 6px;
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

.pet-art {
  display: block;
  width: 100%;
  height: 100%;
  overflow: visible;
}
.pet-body {
  transform-box: fill-box;
  transform-origin: 50% 100%;
}
.pet-shadow {
  fill: rgba(15, 23, 42, 0.1);
  transform-box: fill-box;
  transform-origin: center;
}
.pet-shape,
.pet-arm {
  fill: var(--accent-soft);
  stroke: var(--accent);
  stroke-opacity: 0.38;
  stroke-width: 1.3;
}
.pet-sprout-line {
  fill: none;
  stroke: var(--accent);
  stroke-opacity: 0.55;
  stroke-width: 1.6;
  stroke-linecap: round;
}
.pet-sprout-leaf {
  fill: var(--accent);
  fill-opacity: 0.7;
}
.pet-ink {
  fill: var(--text);
}
.pet-eyes {
  transform-box: fill-box;
  transform-origin: center;
  animation: pet-blink 5.6s ease-in-out infinite;
}
.pet-eyes-sad,
.pet-mouth-sad {
  opacity: 0;
}
.pet-eyes-sad path {
  fill: none;
  stroke: var(--text);
  stroke-width: 1.5;
  stroke-linecap: round;
}
.pet-mouth,
.pet-mouth-sad {
  fill: none;
  stroke: var(--text);
  stroke-width: 1.6;
  stroke-linecap: round;
}
.pet-blush {
  fill: var(--accent);
  fill-opacity: 0.26;
}
.pet-spark {
  fill: var(--accent);
  opacity: 0;
  transform-box: fill-box;
  transform-origin: center;
}

/* Checking: a gentle head tilt plus drifting thought dots. */
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

.sidebar-pet.is-idle .pet-body {
  animation: pet-breathe 3.6s ease-in-out infinite;
}
.sidebar-pet.is-checking .pet-body {
  animation: pet-tilt 1.2s ease-in-out infinite;
}
.sidebar-pet.is-checking .pet-think {
  opacity: 1;
}
.sidebar-pet.is-ok .pet-body {
  animation: pet-jump 760ms var(--ease-out) both;
}
.sidebar-pet.is-warn .pet-body {
  animation: pet-wobble 620ms ease-in-out both;
}
.sidebar-pet.is-warn .pet-eyes,
.sidebar-pet.is-warn .pet-mouth {
  opacity: 0;
}
.sidebar-pet.is-warn .pet-eyes-sad,
.sidebar-pet.is-warn .pet-mouth-sad {
  opacity: 1;
}
.sidebar-pet.is-ok .pet-spark {
  animation: pet-spark 900ms ease-out 120ms both;
}
.sidebar-pet.is-ok .pet-spark-b {
  animation-delay: 260ms;
}

@keyframes pet-breathe {
  0%,
  100% {
    transform: scale(1, 1);
  }
  50% {
    transform: scale(1.015, 1.03);
  }
}
@keyframes pet-tilt {
  0%,
  100% {
    transform: rotate(-3deg) translateY(0);
  }
  50% {
    transform: rotate(3deg) translateY(-1px);
  }
}
@keyframes pet-jump {
  0% {
    transform: translateY(0) scale(1, 1);
  }
  20% {
    transform: translateY(1.5px) scale(1.05, 0.93);
  }
  48% {
    transform: translateY(-9px) scale(0.97, 1.04);
  }
  70% {
    transform: translateY(0) scale(1.04, 0.95);
  }
  85% {
    transform: translateY(-2px) scale(0.99, 1.01);
  }
  100% {
    transform: translateY(0) scale(1, 1);
  }
}
@keyframes pet-wobble {
  0%,
  100% {
    transform: rotate(0deg);
  }
  20% {
    transform: rotate(-6deg);
  }
  40% {
    transform: rotate(5deg);
  }
  60% {
    transform: rotate(-3deg);
  }
  80% {
    transform: rotate(2deg);
  }
}
@keyframes pet-blink {
  0%,
  92%,
  100% {
    transform: scaleY(1);
  }
  95% {
    transform: scaleY(0.12);
  }
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
@keyframes pet-spark {
  0% {
    opacity: 0;
    transform: scale(0.4);
  }
  40% {
    opacity: 1;
  }
  100% {
    opacity: 0;
    transform: scale(1.15);
  }
}
</style>
