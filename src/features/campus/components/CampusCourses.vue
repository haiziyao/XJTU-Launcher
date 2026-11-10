<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import SectionHeader from '../../../components/ui/SectionHeader.vue';
import FilterBar from '../../../components/ui/FilterBar.vue';
import MenuSelect from '../../../components/ui/MenuSelect.vue';
import DataTable, { type DataColumn, type DataSort } from '../../../components/ui/DataTable.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { currentAcademicStart } from '../../../shared/domain';
import type { Courses, Terms } from '../../../types';

/**
 * 全校课程 (LAYOUT §9.6): the whole published timetable for one term, filtered
 * and sorted in place. The school returns thousands of rows, so the table caps
 * at 300 and the header says the honest total.
 */
const LIMIT = 300;
const terms = ref<Terms | null>(null);
const courses = ref<Courses | null>(null);
const term = ref('');
const query = ref('');
const sort = ref<DataSort | null>({ key: 'name', dir: 'asc' });
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');

const columns: DataColumn[] = [
  { key: 'name', label: '课程', sortable: true },
  { key: 'code', label: '课号', sortable: true },
  { key: 'class', label: '教学班' },
  { key: 'teachers', label: '教师', sortable: true },
  { key: 'credits', label: '学分', align: 'right', sortable: true },
  { key: 'campus', label: '校区' },
  { key: 'department', label: '院系' },
  { key: 'timePlace', label: '时间地点' },
];
// Hide future academic years; they have no published timetable yet.
const academicStart = currentAcademicStart();
const termOptions = computed(() =>
  (terms.value?.rows ?? [])
    .filter((item) => {
      const year = Number(item.id.slice(0, 4));
      return !Number.isFinite(year) || year <= academicStart;
    })
    .map((item) => ({ value: item.id, label: item.name })),
);
const termName = computed(
  () => terms.value?.rows.find((item) => item.id === term.value)?.name ?? courses.value?.term ?? '',
);
const allRows = computed<Record<string, unknown>[]>(() =>
  (courses.value?.rows ?? []).map((row, index) => ({
    ...row,
    key: `${row.code}-${row.class}-${index}`,
  })),
);
const filtered = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return allRows.value;
  return allRows.value.filter((row) =>
    ['name', 'code', 'class', 'teachers', 'department', 'timePlace'].some((key) =>
      String(row[key] ?? '')
        .toLowerCase()
        .includes(keyword),
    ),
  );
});
const rows = computed(() => filtered.value.slice(0, LIMIT));
const truncated = computed(() => filtered.value.length > LIMIT);
const subtitle = computed(() =>
  courses.value
    ? [termName.value, `${filtered.value.length} 门`].filter(Boolean).join(' · ')
    : termName.value,
);

async function loadTerms(force = false) {
  await applyCached<Terms>('terms', undefined, (value) => {
    terms.value = value;
    term.value ||= value.rows[0]?.id ?? '';
  });
  terms.value = (await bridge.schoolQuery({ type: 'terms' }, 'terms', force)) as Terms;
  term.value ||= terms.value.rows[0]?.id ?? '';
}

async function loadCourses(force = false) {
  loading.value = true;
  error.value = '';
  try {
    if (!terms.value?.rows.length || force) await loadTerms(force);
    if (!term.value) return;
    const params = { term: term.value };
    await applyCached<Courses>('courses', params, (value) => {
      if (value.term === term.value) courses.value = value;
    });
    courses.value = (await bridge.schoolQuery(
      { type: 'courses', term: params.term, name: '', campus: '' },
      cacheName('courses', params),
      force,
    )) as Courses;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] courses', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeTerm(id: string) {
  term.value = id;
  query.value = '';
  void loadCourses();
}

onMounted(() => void loadCourses());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="全校课程"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="loadCourses(true)"
    >
      <template #filters>
        <MenuSelect
          :model-value="term"
          :options="termOptions"
          label="学期"
          placeholder="选择学期"
          fit
          @update:model-value="changeTerm"
        />
      </template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />

    <FilterBar
      v-model:query="query"
      placeholder="搜索课程、课号、教师或院系"
      label="搜索全校课程"
    />

    <DataTable
      v-model:sort="sort"
      :columns="columns"
      :rows="rows"
      row-key="key"
      :loading="loading && !courses"
      :empty-text="query.trim() ? '没有匹配的课程' : '这个学期还没有发布课程'"
      label="全校课程"
    >
      <template #cell-name="{ row }">
        <strong class="course-name">{{ row.name }}</strong>
      </template>
      <template #cell-code="{ row }">{{ row.code || '—' }}</template>
      <template #cell-class="{ row }">{{ row.class || '—' }}</template>
      <template #cell-teachers="{ row }">{{ row.teachers || '—' }}</template>
      <template #cell-credits="{ row }">{{ row.credits ?? '—' }}</template>
      <template #cell-campus="{ row }">{{ row.campus || '—' }}</template>
      <template #cell-department="{ row }">{{ row.department || '—' }}</template>
      <template #cell-timePlace="{ row }">{{ row.timePlace || '—' }}</template>
    </DataTable>

    <p v-if="truncated" class="course-truncated">
      共 {{ filtered.length }} 门，显示前 {{ LIMIT }} 门；用搜索缩小范围。
    </p>
  </section>
</template>

<style scoped>
.course-name {
  font-weight: 500;
}
.course-truncated {
  padding: 10px 2px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
</style>
