<script setup lang="ts">
import { computed } from 'vue';
import { CircleAlert, Clock } from 'lucide-vue-next';
import { dueInfo } from '../../domain';

/**
 * Deadline plus countdown (LAYOUT §5.3). Pass a ticking `now` from the page when
 * the countdown has to stay live; the default reads the clock once per change.
 * `done` keeps the date but drops the countdown and the urgency colour.
 */
const props = withDefaults(
  defineProps<{
    at: number | null;
    now?: number;
    done?: boolean;
    countdown?: boolean;
  }>(),
  { now: 0, done: false, countdown: true },
);

const info = computed(() => dueInfo(props.at, props.now || Date.now()));
const tone = computed(() => (props.done ? 'neutral' : info.value.tone));
const showCountdown = computed(
  () => props.countdown && !props.done && Boolean(info.value.countdown),
);
const icon = computed(() => {
  if (tone.value === 'danger') return CircleAlert;
  if (tone.value === 'warning') return Clock;
  return null;
});
const aria = computed(() =>
  [info.value.text, showCountdown.value ? info.value.countdown : ''].filter(Boolean).join('，'),
);
</script>

<template>
  <span class="due-label" :class="`tone-${tone}`" :aria-label="aria">
    <component :is="icon" v-if="icon" :size="12" aria-hidden="true" />
    <span>{{ info.text }}</span>
    <template v-if="showCountdown">
      <span class="due-sep" aria-hidden="true">·</span>
      <span class="due-countdown">{{ info.countdown }}</span>
    </template>
  </span>
</template>

<style scoped>
.due-label {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.4;
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}
.tone-warning {
  color: var(--warning);
}
.tone-danger {
  color: var(--danger);
}
.due-countdown {
  font-weight: 500;
}
.due-sep {
  color: var(--text-tertiary);
}
</style>
