<script setup lang="ts">
import { computed, ref } from 'vue';
import { CircleAlert, Info, TriangleAlert, X } from 'lucide-vue-next';
import UiIconButton from './UiIconButton.vue';
import { dismissNotice, noticeDismissed } from '../../shared/notices';

/**
 * Section-level message strip (LAYOUT §8): info, warning, error, and the
 * "some courses failed" partial state. Errors and warnings never rely on colour
 * alone — each carries an icon and a text title.
 *
 * `noticeId` marks a standing explanation of what the app can do (not a live
 * state). Those get a close button and stay closed once dismissed.
 */
const props = withDefaults(
  defineProps<{
    tone?: 'info' | 'warning' | 'error' | 'partial';
    title?: string;
    dismissible?: boolean;
    noticeId?: string;
  }>(),
  { tone: 'info', title: '', dismissible: false, noticeId: '' },
);
const emit = defineEmits<{ dismiss: [] }>();
const closed = ref(false);
const hidden = computed(
  () => closed.value || (!!props.noticeId && noticeDismissed(props.noticeId)),
);
const closable = computed(() => props.dismissible || !!props.noticeId);

function close() {
  if (props.noticeId) dismissNotice(props.noticeId);
  closed.value = true;
  emit('dismiss');
}

const icon = computed(() =>
  props.tone === 'info' ? Info : props.tone === 'error' ? CircleAlert : TriangleAlert,
);
</script>

<template>
  <div
    v-if="!hidden"
    class="inline-alert"
    :class="`tone-${tone}`"
    :role="tone === 'error' ? 'alert' : 'status'"
  >
    <component :is="icon" :size="15" class="inline-alert-icon" aria-hidden="true" />
    <div class="inline-alert-copy">
      <strong v-if="title">{{ title }}</strong>
      <p v-if="$slots.default"><slot /></p>
      <div v-if="$slots.actions" class="inline-alert-actions"><slot name="actions" /></div>
    </div>
    <UiIconButton v-if="closable" variant="inherit" size="xs" label="关闭提示" @click="close">
      <X :size="14" />
    </UiIconButton>
  </div>
</template>

<style scoped>
.inline-alert {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 9px 12px;
  /* Keep the strip clear of the block underneath; several pages place it
     directly above a table or card that would otherwise touch it. */
  margin-bottom: 12px;
  border-radius: var(--radius);
  font-size: 12.5px;
  line-height: 1.55;
  overflow-wrap: anywhere;
}
.inline-alert-icon {
  margin-top: 2px;
  flex-shrink: 0;
}
.inline-alert-copy {
  flex: 1;
  min-width: 0;
}
.inline-alert-copy strong {
  display: block;
  font-weight: 600;
}
.inline-alert-copy p {
  margin: 0;
}
.inline-alert-copy strong + p {
  margin-top: 1px;
}
.inline-alert-actions {
  display: flex;
  gap: 6px;
  margin-top: 6px;
}
.tone-info {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.tone-warning,
.tone-partial {
  background: var(--warning-soft);
  color: var(--warning);
}
.tone-error {
  background: var(--danger-soft);
  color: var(--danger);
}
</style>
