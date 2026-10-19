<script setup lang="ts">
import { nextTick, ref } from 'vue';
import { Check, Pencil, X } from 'lucide-vue-next';
const props = defineProps<{
  value: string;
  placeholder: string;
  label: string;
  disabled?: boolean;
  maxlength?: number;
}>();
const emit = defineEmits<{ save: [value: string] }>();
const editing = ref(false);
const draft = ref('');
const input = ref<HTMLInputElement | null>(null);
const display = ref<HTMLButtonElement | null>(null);
async function begin() {
  draft.value = props.value;
  editing.value = true;
  await nextTick();
  input.value?.focus();
  input.value?.select();
}
async function close() {
  editing.value = false;
  await nextTick();
  display.value?.focus();
}
function save() {
  if (props.disabled) return;
  const value = draft.value.trim();
  if (value !== props.value) emit('save', value);
  void close();
}
</script>
<template>
  <form
    v-if="editing"
    class="inline-edit-form"
    @submit.prevent="save"
    @keydown.esc.stop.prevent="close"
  >
    <input
      ref="input"
      v-model="draft"
      class="inline-edit-input"
      :aria-label="label"
      :maxlength="maxlength"
      :placeholder="placeholder"
      :disabled="disabled"
    />
    <button class="btn ghost icon small" :disabled="disabled" aria-label="保存修改" title="保存">
      <Check :size="14" />
    </button>
    <button
      class="btn ghost icon small"
      type="button"
      aria-label="取消修改"
      title="取消"
      @click="close"
    >
      <X :size="14" />
    </button>
  </form>
  <button
    v-else
    ref="display"
    class="editable-value"
    :disabled="disabled"
    :aria-label="'修改' + label"
    @click="begin"
  >
    <span :class="{ muted: !value }">{{ value || placeholder }}</span
    ><Pencil :size="13" />
  </button>
</template>
