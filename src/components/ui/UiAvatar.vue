<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { UserRound } from 'lucide-vue-next';

/**
 * An avatar image with one fallback. The school profile picture lives behind
 * the campus login and often fails to load in the webview; when it does, the
 * user-configured avatar is shown instead of a broken image. Both failing
 * leaves the neutral silhouette.
 */
const props = withDefaults(
  defineProps<{
    src?: string | null;
    fallback?: string | null;
    size?: number;
    alt?: string;
  }>(),
  { src: null, fallback: null, size: 32, alt: '' },
);
const failedSrc = ref(false);
const failedFallback = ref(false);
watch(
  () => [props.src, props.fallback],
  () => {
    failedSrc.value = false;
    failedFallback.value = false;
  },
);
const usingFallback = computed(() => !props.src || failedSrc.value);
const shown = computed(() => {
  if (!usingFallback.value) return props.src;
  return props.fallback && !failedFallback.value ? props.fallback : null;
});
function onError() {
  if (usingFallback.value) failedFallback.value = true;
  else failedSrc.value = true;
}
</script>

<template>
  <span class="ui-avatar" :style="{ '--avatar-size': size + 'px' }">
    <img v-if="shown" :src="shown" :alt="alt" @error="onError" /><UserRound
      v-else
      :size="Math.round(size * 0.55)"
      :stroke-width="1.7"
    />
  </span>
</template>
