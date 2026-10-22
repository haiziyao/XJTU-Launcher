<script setup lang="ts">
import type { PetId } from '../pets';

const props = defineProps<{ variant: PetId }>();

const shapes: Record<PetId, string> = {
  xixi: 'M32 15c13.8 0 23 9.6 23 21.5S45.8 54 32 54 9 48.4 9 36.5 18.2 15 32 15Z',
  anan: 'M32 16c13.4 0 22.5 9.2 22.5 21S45.4 53 32 53 9.5 48 9.5 37 18.6 16 32 16Z',
  jiaojiao: 'M32 16c13.4 0 22.5 9.2 22.5 21S45.4 53 32 53 9.5 48 9.5 37 18.6 16 32 16Z',
  tongtong: 'M32 15.5c13.8 0 23 9.4 23 21S45.8 53.5 32 53.5 9 48 9 36.5s9.2-21 23-21Z',
  dada: 'M32 16c12.2 0 21 9.4 21 21 0 9.6-8.8 16-21 16s-21-6.4-21-16c0-11.6 8.8-21 21-21Z',
  xuexue:
    'M32 18c5.8 0 10.6 3 12.8 7.4 5.4.5 9.7 4.2 9.7 9.2 0 5.4-4.7 9.4-10.5 9.4H20c-5.8 0-10.5-4-10.5-9.4 0-5 4.2-8.7 9.6-9.2C21.3 21 26.2 18 32 18Z',
};

void props;
</script>

<template>
  <svg class="pet-art" viewBox="0 0 64 64" aria-hidden="true">
    <ellipse class="pet-shadow" cx="32" cy="56.5" rx="13" ry="3.2" />
    <g class="pet-body">
      <!-- Ears and head pieces sit behind the body outline. -->
      <g v-if="variant === 'anan'" class="pet-ear">
        <path d="M21.8 21 18.4 10.4 28 15.4Z" />
        <path d="M42.2 21 45.6 10.4 36 15.4Z" />
        <path class="pet-ear-inner" d="m23 18.4-1.2-3.9 4 2.1Z" />
        <path class="pet-ear-inner" d="m41 18.4 1.2-3.9-4 2.1Z" />
      </g>
      <g v-if="variant === 'jiaojiao'" class="pet-ear">
        <ellipse cx="24.5" cy="10" rx="4" ry="9" transform="rotate(-8 24.5 10)" />
        <ellipse cx="39.5" cy="10" rx="4" ry="9" transform="rotate(8 39.5 10)" />
        <ellipse
          class="pet-ear-inner"
          cx="24.5"
          cy="10"
          rx="1.7"
          ry="5.4"
          transform="rotate(-8 24.5 10)"
        />
        <ellipse
          class="pet-ear-inner"
          cx="39.5"
          cy="10"
          rx="1.7"
          ry="5.4"
          transform="rotate(8 39.5 10)"
        />
      </g>
      <g v-if="variant === 'tongtong'" class="pet-ear">
        <circle cx="19.5" cy="19.5" r="6" />
        <circle cx="44.5" cy="19.5" r="6" />
        <circle class="pet-ear-inner" cx="19.5" cy="19.5" r="2.9" />
        <circle class="pet-ear-inner" cx="44.5" cy="19.5" r="2.9" />
      </g>
      <g v-if="variant === 'xixi'">
        <path class="pet-sprout-line" d="M32 16c0-4 1.4-6.8 3.8-8.6" />
        <path class="pet-sprout-leaf" d="M35.4 8.6c3.8-1.6 7-.6 8.2 1.8-2.4 2.4-6 2.2-8.2-1.8Z" />
      </g>
      <g v-if="variant === 'dada'" class="pet-tuft">
        <path d="M32 16.5v-4.6" />
        <path d="m35.4 17.4 2-3.6" />
        <path d="m28.6 17.4-2-3.6" />
      </g>
      <g v-if="variant === 'xuexue'" class="pet-snow">
        <path d="M46 6.4v7" />
        <path d="m42.9 8.2 6.2 3.4" />
        <path d="m49.1 8.2-6.2 3.4" />
      </g>

      <circle v-if="variant !== 'xuexue'" class="pet-arm" cx="10.5" cy="41" r="3.6" />
      <circle v-if="variant !== 'xuexue'" class="pet-arm" cx="53.5" cy="41" r="3.6" />
      <path class="pet-shape" :d="shapes[variant]" />

      <ellipse v-if="variant === 'tongtong'" class="pet-muzzle" cx="32" cy="40.5" rx="9" ry="6.4" />
      <circle v-if="variant === 'tongtong'" class="pet-ink" cx="32" cy="38.2" r="1.7" />
      <path v-if="variant === 'dada'" class="pet-beak" d="M28.6 38.6h6.8L32 43.6Z" />
      <g v-if="variant === 'anan'" class="pet-whisker">
        <path d="M13.2 34.4h5.4M13.2 38.2h5.4M45.4 34.4h5.4M45.4 38.2h5.4" />
      </g>

      <g class="pet-eyes">
        <circle cx="24.5" cy="34" r="2.4" class="pet-ink" />
        <circle cx="39.5" cy="34" r="2.4" class="pet-ink" />
      </g>
      <g class="pet-eyes-sad">
        <path d="m22.4 32 4.2 4.2M26.6 32l-4.2 4.2" />
        <path d="m37.4 32 4.2 4.2M41.6 32l-4.2 4.2" />
      </g>

      <path
        v-if="variant === 'xixi'"
        class="pet-mouth-open"
        d="M29.3 39.2h5.4q-.6 3.4-2.7 3.4t-2.7-3.4Z"
      />
      <path v-else-if="variant === 'tongtong'" class="pet-mouth" d="M30 42.2q2 1.6 4 0" />
      <path v-else-if="variant !== 'dada'" class="pet-mouth" d="M29.6 40.6q2.4 2.2 4.8 0" />
      <path class="pet-mouth-sad" d="M29.6 42.4q2.4-2.2 4.8 0" />

      <ellipse class="pet-blush" cx="18.2" cy="39.8" rx="3.1" ry="1.9" />
      <ellipse class="pet-blush" cx="45.8" cy="39.8" rx="3.1" ry="1.9" />
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
</template>
