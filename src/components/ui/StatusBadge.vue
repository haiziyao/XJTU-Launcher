<script lang="ts">
export type StatusTone = 'done' | 'near' | 'urgent' | 'active' | 'neutral' | 'new';
</script>

<script setup lang="ts">
import { computed } from 'vue';
import { Circle, CircleAlert, CircleCheck, CircleDot, Clock } from 'lucide-vue-next';

/**
 * Status marker from LAYOUT §5.2. Colour is never the only signal: every
 * status carries an icon and a word, and `new` is a 6px dot.
 */
const props = withDefaults(
  defineProps<{
    status: StatusTone;
    /** 具体状态词（已交 / 可重交 / 已截止）；缺省用状态语义词。 */
    label?: string;
  }>(),
  { label: '' },
);

const ICONS = {
  done: CircleCheck,
  near: Clock,
  urgent: CircleAlert,
  active: CircleDot,
  neutral: Circle,
} as const;
const WORDS: Record<StatusTone, string> = {
  done: '完成',
  near: '临近',
  urgent: '紧急',
  active: '进行中',
  neutral: '未知',
  new: '新',
};
const icon = computed(() => (props.status === 'new' ? null : ICONS[props.status]));
const text = computed(() => props.label || WORDS[props.status]);
</script>

<template>
  <span v-if="status === 'new'" class="status-badge is-new">
    <i aria-hidden="true"></i>
    <span v-if="label">{{ label }}</span>
    <span v-else class="visually-hidden">新</span>
  </span>
  <span v-else class="status-badge" :class="`is-${status}`">
    <component :is="icon" :size="12" aria-hidden="true" />
    <span>{{ text }}</span>
  </span>
</template>

<style scoped>
.status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 18px;
  padding: 0 6px;
  border-radius: var(--radius-xs);
  font-size: 11px;
  font-weight: 500;
  line-height: 1;
  white-space: nowrap;
}
.status-badge svg {
  flex-shrink: 0;
}
.is-done {
  background: var(--success-soft);
  color: var(--success-text);
}
.is-near {
  background: var(--warning-soft);
  color: var(--warning);
}
.is-urgent {
  background: var(--danger-soft);
  color: var(--danger);
}
.is-active {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.is-neutral {
  background: var(--surface-sunken);
  color: var(--text-secondary);
}
/* “新”不用底色，只放一个圆点；有文字时跟在点后面。 */
.is-new {
  gap: 5px;
  height: auto;
  padding: 0;
  background: none;
  color: var(--accent-text);
  font-weight: 400;
}
.is-new i {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}
.visually-hidden {
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
</style>
