<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ChevronDown } from 'lucide-vue-next';

export interface MenuOption {
  value: string;
  label: string;
}

const props = defineProps<{
  modelValue: string;
  options: MenuOption[];
  label: string;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const open = ref(false);
const root = ref<HTMLElement | null>(null);
const current = computed(
  () => props.options.find((option) => option.value === props.modelValue)?.label ?? '',
);
function choose(value: string) {
  emit('update:modelValue', value);
  open.value = false;
}
function onOutside(event: PointerEvent) {
  if (root.value && !root.value.contains(event.target as Node)) open.value = false;
}
watch(open, (value) => {
  if (value) document.addEventListener('pointerdown', onOutside, true);
  else document.removeEventListener('pointerdown', onOutside, true);
});
onBeforeUnmount(() => document.removeEventListener('pointerdown', onOutside, true));
</script>

<template>
  <div ref="root" class="menu-field" @keydown.esc="open = false">
    <div class="field-shell">
      <button
        type="button"
        class="menu-field-value"
        :aria-label="label"
        :aria-expanded="open"
        aria-haspopup="listbox"
        @click="open = !open"
      >
        <span>{{ current }}</span>
        <ChevronDown :size="14" />
      </button>
    </div>
    <ul v-if="open" class="menu-field-menu" role="listbox" :aria-label="label">
      <li v-for="option in options" :key="option.value">
        <button
          type="button"
          role="option"
          :aria-selected="option.value === modelValue"
          :class="{ active: option.value === modelValue }"
          @click="choose(option.value)"
        >
          {{ option.label }}
        </button>
      </li>
    </ul>
  </div>
</template>
