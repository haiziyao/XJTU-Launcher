<script setup lang="ts">
import type { PetId } from '../pets';

defineProps<{ variant: PetId }>();

const shapes: Record<PetId, string> = {
  xixi: 'M32 19C43.5 19 53 27.5 53 38.3C53 48.2 43.8 53 32 53C20.2 53 11 48.2 11 38.3C11 27.5 20.5 19 32 19Z',
  anan: 'M32 16.5C44 16.5 53 24.5 53 36.4C53 47 44 53 32 53C20 53 11 47 11 36.4C11 24.5 20 16.5 32 16.5Z',
  jiaojiao: 'M32 21C43 21 52 28 52 39C52 48.5 43 53 32 53C21 53 12 48.5 12 39C12 28 21 21 32 21Z',
  tongtong:
    'M32 16C44.5 16 53 25 53 36.8C53 47.5 44.5 53 32 53C19.5 53 11 47.5 11 36.8C11 25 19.5 16 32 16Z',
  dada: 'M32 18C44 18 53 27 53 38C53 48 44 53 32 53C20 53 11 48 11 38C11 27 20 18 32 18Z',
  xuexue: 'M32 20C43 20 52 28 52 39C52 48.5 44 53 32 53C20 53 12 48.5 12 39C12 28 21 20 32 20Z',
};
</script>

<template>
  <svg class="pet-art" :data-pet="variant" viewBox="0 0 64 64" aria-hidden="true">
    <ellipse class="pet-shadow" cx="32" cy="56.5" rx="13.5" ry="3.2" />
    <g class="pet-body">
      <!-- Ears, sprouts and other pieces that sit behind the head. -->
      <template v-if="variant === 'xixi'">
        <path class="pet-sprout-line" d="M32 20C32 14.6 33.4 10.8 36.4 8.6" />
        <path
          class="pet-sprout-leaf"
          d="M36.4 8.6C41.4 5 46.4 6.4 47.6 10.6C43.4 12.8 38.6 11.8 36.4 8.6Z"
        />
        <path
          class="pet-sprout-leaf pet-sprout-leaf-b"
          d="M33.3 13.6C29.5 10.4 26.1 11.1 25 14.2C28.1 16.3 31.4 15.7 33.3 13.6Z"
        />
      </template>
      <template v-else-if="variant === 'anan'">
        <path class="pet-ear" d="M20.5 22 16.2 9.6 28.6 15.2Z" />
        <path class="pet-ear" d="M43.5 22 47.8 9.6 35.4 15.2Z" />
        <path class="pet-ear-inner" d="M21.8 19.6 19.4 12.6 26.2 15.8Z" />
        <path class="pet-ear-inner" d="M42.2 19.6 44.6 12.6 37.8 15.8Z" />
      </template>
      <template v-else-if="variant === 'jiaojiao'">
        <ellipse class="pet-ear" cx="25" cy="13" rx="4.4" ry="10.4" transform="rotate(-7 25 13)" />
        <ellipse class="pet-ear" cx="39" cy="13" rx="4.4" ry="10.4" transform="rotate(7 39 13)" />
        <ellipse
          class="pet-ear-inner"
          cx="25"
          cy="13"
          rx="1.9"
          ry="6.6"
          transform="rotate(-7 25 13)"
        />
        <ellipse
          class="pet-ear-inner"
          cx="39"
          cy="13"
          rx="1.9"
          ry="6.6"
          transform="rotate(7 39 13)"
        />
        <path class="pet-accent" d="M41.4 15.4 46.6 12.6 46.6 18.2Z" />
        <path class="pet-accent" d="M41.4 15.4 36.2 12.6 36.2 18.2Z" />
        <circle class="pet-accent" cx="41.4" cy="15.4" r="1.2" />
      </template>
      <template v-else-if="variant === 'tongtong'">
        <circle class="pet-ink" cx="18.8" cy="18.5" r="6.2" />
        <circle class="pet-ink" cx="45.2" cy="18.5" r="6.2" />
      </template>
      <template v-else-if="variant === 'dada'">
        <path class="pet-line" d="M32 19V13M32 13Q29.4 9.8 26.6 10.4M32 13Q34.6 9.8 37.4 10.4" />
      </template>
      <template v-else>
        <g class="pet-accent-line">
          <path d="M47 7V15M43.5 9L50.5 13M50.5 9L43.5 13" />
        </g>
      </template>

      <circle v-if="variant !== 'xuexue'" class="pet-arm" cx="11.6" cy="43" r="3.2" />
      <circle v-if="variant !== 'xuexue'" class="pet-arm" cx="52.4" cy="43" r="3.2" />
      <path class="pet-shape" :d="shapes[variant]" />

      <!-- Front accessories: bamboo leaf, bell, scarf. -->
      <template v-if="variant === 'tongtong'">
        <path
          class="pet-accent"
          d="M12.8 26C6.6 20 7.8 14.2 12.2 12.8C15.6 16.9 14.7 22.2 12.8 26Z"
        />
        <path class="pet-line" d="M12.7 26.2 16.2 31" />
      </template>
      <template v-else-if="variant === 'anan'">
        <path class="pet-line" d="M32 48.2V46.2" />
        <circle class="pet-accent" cx="32" cy="50.4" r="2.2" />
      </template>
      <template v-else-if="variant === 'xuexue'">
        <path
          class="pet-scarf"
          d="M14.6 43.4C20 47 44 47 49.4 43.4L49.4 49C44 52.4 20 52.4 14.6 49Z"
        />
        <path class="pet-scarf" d="M42.6 49 44.8 56.4 40.6 55 39 58.6 36.4 49Z" />
      </template>

      <!-- Faces. Tongtong's eye patches stay put while blinking. -->
      <template v-if="variant === 'tongtong'">
        <g class="pet-eyes-static">
          <ellipse
            class="pet-ink"
            cx="24"
            cy="33.4"
            rx="6"
            ry="6.8"
            transform="rotate(-12 24 33.4)"
          />
          <ellipse
            class="pet-ink"
            cx="40"
            cy="33.4"
            rx="6"
            ry="6.8"
            transform="rotate(12 40 33.4)"
          />
          <circle class="pet-muzzle" cx="24.4" cy="33" r="2.1" />
          <circle class="pet-muzzle" cx="39.6" cy="33" r="2.1" />
          <circle class="pet-ink" cx="24.8" cy="33.2" r="0.9" />
          <circle class="pet-ink" cx="39.2" cy="33.2" r="0.9" />
        </g>
        <ellipse class="pet-muzzle" cx="32" cy="41" rx="8.4" ry="6" />
        <ellipse class="pet-ink" cx="32" cy="39.2" rx="2.1" ry="1.4" />
        <path class="pet-mouth" d="M30 43.4Q32 45 34 43.4" />
      </template>
      <template v-else>
        <g class="pet-eyes">
          <template v-if="variant === 'anan'">
            <path class="pet-eye-line" d="M20.8 34.2Q24 37.6 27.2 34.2" />
            <path class="pet-eye-line" d="M36.8 34.2Q40 37.6 43.2 34.2" />
          </template>
          <template v-else-if="variant === 'xuexue'">
            <path class="pet-eye-line" d="M21 34.6Q24.2 37.8 27.4 34.6" />
            <path class="pet-eye-line" d="M36.6 34.6Q39.8 37.8 43 34.6" />
          </template>
          <template v-else-if="variant === 'jiaojiao'">
            <ellipse class="pet-ink" cx="24.2" cy="36" rx="1.8" ry="2.4" />
            <ellipse class="pet-ink" cx="39.8" cy="36" rx="1.8" ry="2.4" />
            <circle class="pet-highlight" cx="24.8" cy="35.2" r="0.6" />
            <circle class="pet-highlight" cx="40.4" cy="35.2" r="0.6" />
          </template>
          <template v-else-if="variant === 'dada'">
            <circle class="pet-ink" cx="24.4" cy="34.6" r="2.5" />
            <circle class="pet-ink" cx="39.6" cy="34.6" r="2.5" />
            <circle class="pet-highlight" cx="25.3" cy="33.8" r="0.8" />
            <circle class="pet-highlight" cx="40.5" cy="33.8" r="0.8" />
          </template>
          <template v-else>
            <circle class="pet-ink" cx="24.6" cy="35" r="2.6" />
            <circle class="pet-ink" cx="39.4" cy="35" r="2.6" />
            <circle class="pet-highlight" cx="25.5" cy="34.2" r="0.8" />
            <circle class="pet-highlight" cx="40.3" cy="34.2" r="0.8" />
          </template>
        </g>
      </template>

      <g class="pet-eyes-sad">
        <template v-if="variant === 'jiaojiao'">
          <path d="M22 34.2 26.4 38.6M26.4 34.2 22 38.6" />
          <path d="M37.6 34.2 42 38.6M42 34.2 37.6 38.6" />
        </template>
        <template v-else-if="variant === 'tongtong'">
          <path d="M21 30.4 27 36.4M27 30.4 21 36.4" />
          <path d="M37 30.4 43 36.4M43 30.4 37 36.4" />
        </template>
        <template v-else-if="variant === 'xuexue'">
          <path d="M22 32.6 26.4 37M26.4 32.6 22 37" />
          <path d="M37.6 32.6 42 37M42 32.6 37.6 37" />
        </template>
        <template v-else>
          <path d="M22.4 32 26.6 36.2M26.6 32 22.4 36.2" />
          <path d="M37.4 32 41.6 36.2M41.6 32 37.4 36.2" />
        </template>
      </g>

      <!-- Mouths. Dada's beak doubles as its mouth. -->
      <template v-if="variant === 'xixi'">
        <path class="pet-mouth-open" d="M29.2 39.4H34.8Q34.2 43.4 32 43.4Q29.8 43.4 29.2 39.4Z" />
      </template>
      <template v-else-if="variant === 'anan'">
        <path class="pet-ink" d="M30.8 39.6H33.2L32 41.6Z" />
        <path class="pet-mouth" d="M29.2 42.6Q30.6 44 32 42.6M32 42.6Q33.4 44 34.8 42.6" />
      </template>
      <template v-else-if="variant === 'jiaojiao'">
        <path class="pet-mouth" d="M29.4 41Q30.7 42.3 32 41M32 41Q33.3 42.3 34.6 41" />
      </template>
      <template v-else-if="variant === 'dada'">
        <path class="pet-accent" d="M28.4 37.8H35.6L32 43.4Z" />
      </template>
      <template v-else-if="variant === 'xuexue'">
        <path class="pet-mouth" d="M29.4 41Q32 43.2 34.6 41" />
      </template>

      <path v-if="variant === 'xixi'" class="pet-mouth-sad" d="M29.8 42Q32 40 34.2 42" />
      <path
        v-else-if="variant === 'anan'"
        class="pet-mouth-sad"
        d="M29.2 41.6Q30.6 40.2 32 41.6M32 41.6Q33.4 40.2 34.8 41.6"
      />
      <path
        v-else-if="variant === 'jiaojiao'"
        class="pet-mouth-sad"
        d="M29.4 41.6Q30.7 40.3 32 41.6M32 41.6Q33.3 40.3 34.6 41.6"
      />
      <path
        v-else-if="variant === 'xuexue'"
        class="pet-mouth-sad"
        d="M29.4 41.4Q32 39.4 34.6 41.4"
      />

      <ellipse
        v-if="variant !== 'tongtong'"
        class="pet-blush"
        :cx="variant === 'xixi' ? 18.6 : 19.2"
        cy="40.4"
        :rx="variant === 'jiaojiao' || variant === 'xuexue' ? 3.4 : 3.2"
        ry="1.9"
      />
      <ellipse
        v-if="variant !== 'tongtong'"
        class="pet-blush"
        :cx="variant === 'xixi' ? 45.4 : 44.8"
        cy="40.4"
        :rx="variant === 'jiaojiao' || variant === 'xuexue' ? 3.4 : 3.2"
        ry="1.9"
      />
      <ellipse
        v-if="variant === 'tongtong'"
        class="pet-blush"
        cx="19.2"
        cy="40.6"
        rx="2.8"
        ry="1.7"
      />
      <ellipse
        v-if="variant === 'tongtong'"
        class="pet-blush"
        cx="44.8"
        cy="40.6"
        rx="2.8"
        ry="1.7"
      />

      <!-- Anan's whiskers. -->
      <path
        v-if="variant === 'anan'"
        class="pet-line"
        d="M12.8 33.6H18M12.8 37.2H18M46 33.6H51.2M46 37.2H51.2"
      />
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
