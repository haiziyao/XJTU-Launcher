<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookOpenText } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import { bridge } from '../../bridge';
import type { Courses, Terms } from '../../types';

const terms = ref<Terms | null>(null);
const courses = ref<Courses | null>(null);
const term = ref('');
const name = ref('');
const loading = ref(false);
const error = ref('');

const termOptions = computed(() =>
  (terms.value?.rows ?? []).map((item) => ({ value: item.id, label: item.name })),
);
const rows = computed(() => (courses.value?.rows ?? []).slice(0, 200));
const truncated = computed(() => (courses.value?.rows.length ?? 0) > 200);

async function loadTerms() {
  loading.value = true;
  error.value = '';
  try {
    terms.value = (await bridge.schoolQuery({ type: 'terms' })) as Terms;
    term.value = terms.value.rows[0]?.id ?? '';
  } catch (e) {
    console.error('[campus] terms', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function search() {
  if (!name.value.trim()) return;
  loading.value = true;
  error.value = '';
  try {
    courses.value = (await bridge.schoolQuery({
      type: 'courses',
      term: term.value || null,
      name: name.value.trim(),
      campus: '',
    })) as Courses;
  } catch (e) {
    console.error('[campus] courses', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(loadTerms);
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>
  <SettingGroup title="全校课程">
    <SettingRow title="学期">
      <MenuSelect
        :model-value="term"
        :options="termOptions"
        label="学期"
        placeholder="选择学期"
        @update:model-value="term = $event"
      />
    </SettingRow>
    <SettingRow title="课程名">
      <input
        v-model="name"
        class="text-field"
        placeholder="输入课程名关键词"
        aria-label="课程名"
        maxlength="100"
        @keyup.enter="search"
      />
    </SettingRow>
    <div class="campus-form-actions">
      <button class="btn primary" :disabled="loading || !name.trim()" @click="search">
        {{ loading ? '查询中…' : '查询课程' }}
      </button>
    </div>
  </SettingGroup>
  <SettingGroup v-if="courses" :title="`查询结果 · ${courses.term}`">
    <EmptyState v-if="!courses.rows.length" compact :icon="BookOpenText" title="没有匹配的课程" />
    <div
      v-for="(row, index) in rows"
      :key="`${row.code}-${row.class}-${index}`"
      class="campus-record three"
    >
      <span class="campus-record-main">
        <strong>{{ row.name }}</strong>
        <small>{{ [row.code, row.teachers, row.department].filter(Boolean).join(' · ') }}</small>
      </span>
      <span class="campus-record-meta">{{ row.credits ?? '—' }} 学分</span>
      <span class="campus-record-meta">{{ row.timePlace || row.campus }}</span>
    </div>
    <p v-if="truncated" class="campus-truncated">仅显示前 200 条，请缩小课程名范围。</p>
  </SettingGroup>
</template>

<style scoped>
.campus-truncated {
  padding: 10px 16px;
  color: var(--text-tertiary);
  font-size: 12px;
}
</style>
