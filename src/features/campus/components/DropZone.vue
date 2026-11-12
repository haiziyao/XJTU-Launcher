<script setup lang="ts">
import { computed, ref } from 'vue';
import { UploadCloud } from 'lucide-vue-next';
import UiButton from '../../../components/ui/UiButton.vue';
import { fileSize } from '../../../shared/domain';

/**
 * Homework upload area (LAYOUT §4.2, §9.3). Validates the school limits here
 * so the user learns about them before a preview is prepared; the actual file
 * list stays with the page. The window runs with `dragDropEnabled: false`, so
 * HTML5 drag events reach this element.
 */
const props = withDefaults(
  defineProps<{
    maxFiles?: number;
    maxFileSize?: number;
    maxTotal?: number;
    accept?: string;
    disabled?: boolean;
    /** Overrides the generated “最多 10 个，单个 100 MB” line. */
    hint?: string;
    label?: string;
  }>(),
  {
    maxFiles: 10,
    maxFileSize: 100 * 1024 * 1024,
    maxTotal: 200 * 1024 * 1024,
    accept: '',
    disabled: false,
    hint: '',
    label: '拖入文件，或选择文件',
  },
);
const emit = defineEmits<{ files: [files: File[]]; reject: [message: string] }>();

const active = ref(false);
const input = ref<HTMLInputElement | null>(null);
let depth = 0;

const hintText = computed(
  () => props.hint || `最多 ${props.maxFiles} 个，单个 ${fileSize(props.maxFileSize)}`,
);

function check(files: File[]) {
  if (!files.length) return;
  if (files.length > props.maxFiles) {
    emit('reject', `一次最多 ${props.maxFiles} 个文件`);
    return;
  }
  const tooBig = files.find((file) => file.size > props.maxFileSize);
  if (tooBig) {
    emit('reject', `${tooBig.name} 超过 ${fileSize(props.maxFileSize)}`);
    return;
  }
  const total = files.reduce((sum, file) => sum + file.size, 0);
  if (total > props.maxTotal) {
    emit('reject', `合计大小超过 ${fileSize(props.maxTotal)}`);
    return;
  }
  emit('files', files);
}
function enter() {
  if (props.disabled) return;
  depth++;
  active.value = true;
}
function leave() {
  depth = Math.max(0, depth - 1);
  if (!depth) active.value = false;
}
function drop(event: DragEvent) {
  depth = 0;
  active.value = false;
  if (props.disabled) return;
  check([...(event.dataTransfer?.files ?? [])]);
}
function pick(event: Event) {
  const target = event.target as HTMLInputElement;
  check([...(target.files ?? [])]);
  target.value = '';
}
function open() {
  if (props.disabled) return;
  input.value?.click();
}
</script>

<template>
  <div
    class="drop-zone"
    :class="{ active, disabled }"
    @dragenter.prevent="enter"
    @dragover.prevent
    @dragleave.prevent="leave"
    @drop.prevent="drop"
    @click="open"
  >
    <UploadCloud :size="22" aria-hidden="true" />
    <p class="drop-title">{{ label }}</p>
    <p class="drop-hint">{{ hintText }}</p>
    <UiButton size="sm" :disabled="disabled" @click.stop="open">选择文件</UiButton>
    <input
      ref="input"
      class="drop-input"
      type="file"
      multiple
      :accept="accept || undefined"
      :disabled="disabled"
      tabindex="-1"
      aria-hidden="true"
      @change="pick"
    />
  </div>
</template>

<style scoped>
.drop-zone {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 18px 16px 16px;
  border: 1px dashed var(--border-strong);
  border-radius: var(--radius-lg);
  background: var(--surface-subtle);
  color: var(--text-tertiary);
  text-align: center;
  cursor: pointer;
  transition:
    border-color var(--duration-fast) ease,
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}
.drop-zone:hover:not(.disabled) {
  border-color: var(--accent-outline);
  background: var(--accent-wash);
  color: var(--accent-text);
}
.drop-zone.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent-text);
}
.drop-zone.disabled {
  opacity: 0.55;
  cursor: default;
}
.drop-title {
  color: var(--text);
  font-size: 12.5px;
  font-weight: 500;
}
.drop-zone.active .drop-title,
.drop-zone:hover:not(.disabled) .drop-title {
  color: var(--accent-text);
}
.drop-hint {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.drop-input {
  display: none;
}
</style>
