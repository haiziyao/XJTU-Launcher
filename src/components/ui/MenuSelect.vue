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
  placeholder?: string;
  /** Inside a grouped control (e.g. the week stepper): no field shell of its own. */
  bare?: boolean;
  disabled?: boolean;
  /**
   * Keep the field as wide as its widest option instead of following the
   * current value, so switching values never resizes it and no option is
   * clipped in the menu.
   */
  fit?: boolean;
}>();
const emit = defineEmits<{ 'update:modelValue': [value: string] }>();

const open = ref(false);
const root = ref<HTMLElement | null>(null);
const current = computed(
  () =>
    props.options.find((option) => option.value === props.modelValue)?.label ??
    props.placeholder ??
    '',
);
// Every label is rendered hidden next to the current one; the grid keeps the
// widest of them, so digits and CJK are measured by the browser, not estimated.
const sizerLabels = computed(() => [
  ...new Set([props.placeholder ?? '', ...props.options.map((option) => option.label)]),
]);
function choose(value: string) {
  emit('update:modelValue', value);
  open.value = false;
}
function toggle() {
  if (!props.disabled) open.value = !open.value;
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
  <div ref="root" class="menu-field" :class="{ bare }" @keydown.esc="open = false">
    <div class="field-shell">
      <button
        type="button"
        class="menu-field-value"
        :aria-label="label"
        :aria-expanded="open"
        aria-haspopup="listbox"
        :disabled="disabled"
        @click="toggle"
      >
        <span class="menu-field-text">
          <span class="menu-field-current">{{ current }}</span>
          <span v-if="fit" class="menu-field-sizer" aria-hidden="true">
            <span v-for="label in sizerLabels" :key="label">{{ label }}</span>
          </span>
        </span>
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
