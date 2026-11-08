<script setup lang="ts">
import { computed } from 'vue';
import { Search } from 'lucide-vue-next';
import UiButton from './UiButton.vue';

/**
 * One filter row above a list or table (LAYOUT §4.2): search, the page's own
 * SegmentedTabs / MenuSelect controls, and 清除筛选 once anything is set.
 */
const props = withDefaults(
  defineProps<{
    query: string;
    placeholder?: string;
    /** Non-search filters that cannot be inferred from `query`. */
    active?: boolean;
    label?: string;
  }>(),
  { placeholder: '搜索', active: false, label: '搜索' },
);
const emit = defineEmits<{ 'update:query': [value: string]; clear: [] }>();

const showClear = computed(() => props.active || props.query.trim().length > 0);

function input(event: Event) {
  emit('update:query', (event.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="filter-bar">
    <label class="field-shell filter-search">
      <Search :size="14" aria-hidden="true" />
      <input
        type="search"
        :value="query"
        :placeholder="placeholder"
        :aria-label="label"
        spellcheck="false"
        @input="input"
      />
    </label>
    <div class="filter-controls"><slot /></div>
    <UiButton v-if="showClear" variant="ghost" size="sm" @click="emit('clear')">
      清除筛选
    </UiButton>
    <div v-if="$slots.actions" class="filter-actions"><slot name="actions" /></div>
  </div>
</template>

<style scoped>
.filter-bar {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  margin-bottom: 12px;
}
.filter-search {
  width: 200px;
  flex-shrink: 0;
}
.filter-controls {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
  min-width: 0;
}
.filter-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
}
</style>
