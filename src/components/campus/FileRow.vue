<script setup lang="ts">
import { computed } from 'vue';
import { CircleCheck } from 'lucide-vue-next';
import IconTile from '../ui/IconTile.vue';
import UiButton from '../ui/UiButton.vue';
import { fileSize } from '../../domain';
import { fileKind } from '../../fileType';

/**
 * One file in a course or notice list (LAYOUT §4.2, §9.3). The type label is
 * spelled out next to the size so the file kind is never icon-only; batch mode
 * adds the checkbox and the page adds the bottom action bar.
 */
const props = withDefaults(
  defineProps<{
    name: string;
    size?: number | null;
    /** Explicit type label (from the backend) overrides the extension guess. */
    kind?: string;
    downloaded?: boolean;
    selectable?: boolean;
    checked?: boolean;
    previewable?: boolean;
    downloadable?: boolean;
    busy?: boolean;
    /** Extra line, e.g. “第 6 次作业 · 10月28日”. */
    detail?: string;
  }>(),
  {
    size: null,
    kind: '',
    downloaded: false,
    selectable: false,
    checked: false,
    previewable: true,
    downloadable: true,
    busy: false,
    detail: '',
  },
);
const emit = defineEmits<{ preview: []; download: []; 'update:checked': [value: boolean] }>();

const kind = computed(() => fileKind(props.name));
const typeLabel = computed(() => props.kind || kind.value.label);
const meta = computed(() =>
  [typeLabel.value, fileSize(props.size), props.detail].filter(Boolean).join(' · '),
);

function toggle(event: Event) {
  emit('update:checked', (event.target as HTMLInputElement).checked);
}
</script>

<template>
  <div class="file-row">
    <input
      v-if="selectable"
      type="checkbox"
      class="file-check"
      :checked="checked"
      :aria-label="`选择 ${name}`"
      @change="toggle"
    />
    <IconTile :tone="kind.tone" :size="30">
      <component :is="kind.icon" :size="16" />
    </IconTile>
    <span class="file-copy">
      <span class="file-name" :title="name">{{ name }}</span>
      <span class="file-meta">
        <span>{{ meta }}</span>
        <span v-if="downloaded" class="file-downloaded">
          <CircleCheck :size="12" aria-hidden="true" />已下载
        </span>
      </span>
    </span>
    <span class="file-actions">
      <UiButton
        v-if="previewable"
        size="sm"
        variant="ghost"
        :disabled="busy"
        @click="emit('preview')"
      >
        预览
      </UiButton>
      <UiButton
        v-if="downloadable"
        size="sm"
        variant="ghost"
        :disabled="busy"
        @click="emit('download')"
      >
        {{ busy ? '下载中…' : '下载' }}
      </UiButton>
      <slot name="actions" />
    </span>
  </div>
</template>

<style scoped>
.file-row {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 48px;
  padding: 8px 12px;
  border-radius: var(--radius-sm);
  transition: background var(--duration-fast) ease;
}
.file-row:hover {
  background: var(--surface-hover);
}
.file-check {
  width: 14px;
  height: 14px;
  margin: 0;
  accent-color: var(--accent);
}
.file-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.file-name {
  overflow: hidden;
  font-size: 12.5px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.file-meta {
  display: flex;
  align-items: center;
  gap: 10px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.file-downloaded {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  color: var(--success-text);
}
.file-actions {
  display: flex;
  align-items: center;
  gap: 2px;
  flex-shrink: 0;
}
</style>
