<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookOpen, Search } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { currentAcademicStart, updatedInfo } from '../../../shared/domain';
import type { Courses, Terms } from '../../../types';

/**
 * 全校课程 (layout v2): terms in the left column, the whole published timetable
 * for one term in the centre, filtered and sorted in place. The school returns
 * thousands of rows, so the table caps at 300 and the header says the honest
 * total.
 */
const LIMIT = 300;
const terms = ref<Terms | null>(null);
const courses = ref<Courses | null>(null);
const term = ref('');
const query = ref('');
type SortKey = 'name' | 'code' | 'teachers' | 'credits';
const sort = ref<{ key: SortKey; dir: 'asc' | 'desc' } | null>({ key: 'name', dir: 'asc' });
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');

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
const sorted = computed(() => {
  const current = sort.value;
  if (!current) return filtered.value;
  const factor = current.dir === 'asc' ? 1 : -1;
  return [...filtered.value].sort((left, right) => {
    const a = left[current.key];
    const b = right[current.key];
    if (a == null) return 1;
    if (b == null) return -1;
    if (current.key === 'credits') return (Number(a) - Number(b)) * factor;
    return String(a).localeCompare(String(b), 'zh-Hans-CN', { numeric: true }) * factor;
  });
});
const rows = computed(() => sorted.value.slice(0, LIMIT));
const truncated = computed(() => filtered.value.length > LIMIT);
const summary = computed(() => {
  if (!courses.value) return '';
  const total = `${filtered.value.length} 门`;
  return truncated.value ? `${total} · 显示前 ${LIMIT} 条` : total;
});

function arrow(key: SortKey): string {
  if (sort.value?.key !== key) return '';
  return sort.value.dir === 'asc' ? '↑' : '↓';
}

function sortBy(key: SortKey) {
  sort.value =
    sort.value?.key === key && sort.value.dir === 'asc'
      ? { key, dir: 'desc' }
      : { key, dir: 'asc' };
}

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
  <CampusHeaderSlot slot="meta">
    <span v-if="loadedAt">更新于 {{ updatedInfo(loadedAt) }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <label class="cv-search">
      <Search :size="13" aria-hidden="true" />
      <input
        v-model="query"
        type="search"
        placeholder="搜索课程、课号、教师或院系"
        aria-label="搜索全校课程"
        spellcheck="false"
      />
    </label>
    <button type="button" class="cv-btn" :disabled="loading" @click="loadCourses(true)">
      刷新
    </button>
  </CampusHeaderSlot>

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-lh">学期</div>
      <button
        v-for="option in termOptions"
        :key="option.value"
        type="button"
        class="cv-item plain"
        :class="{ on: term === option.value }"
        @click="changeTerm(option.value)"
      >
        <b>{{ option.label }}</b>
      </button>
    </div>

    <div class="cv-center">
      <InlineAlert v-if="error" tone="error" :title="error" />
      <div class="cv-ch">
        <b>{{ termName || '全校课程' }}</b>
        <span class="meta">{{ summary }}</span>
      </div>

      <div v-if="loading && !courses" class="cv-empty">
        <BookOpen :size="22" /><b>正在读取全校课程…</b>
      </div>
      <div v-else-if="!rows.length" class="cv-empty">
        <BookOpen :size="22" />
        <b>{{ query.trim() ? '没有匹配的课程' : '这个学期还没有发布课程' }}</b>
      </div>
      <div v-else class="cv-panel cv-tbl">
        <div
          class="cv-thead"
          style="--cols: minmax(160px, 1fr) 86px minmax(90px, 1fr) 56px 64px 130px"
        >
          <button type="button" class="th" @click="sortBy('name')">
            课程<span class="arw">{{ arrow('name') }}</span>
          </button>
          <button type="button" class="th" @click="sortBy('code')">
            课号<span class="arw">{{ arrow('code') }}</span>
          </button>
          <button type="button" class="th" @click="sortBy('teachers')">
            教师<span class="arw">{{ arrow('teachers') }}</span>
          </button>
          <span class="cv-r">
            <button type="button" class="th" @click="sortBy('credits')">
              学分<span class="arw">{{ arrow('credits') }}</span>
            </button>
          </span>
          <span>校区</span>
          <span>时间地点</span>
        </div>
        <div
          v-for="row in rows"
          :key="String(row.key)"
          class="cv-tr course-row"
          style="--cols: minmax(160px, 1fr) 86px minmax(90px, 1fr) 56px 64px 130px"
        >
          <span class="c">
            <span class="cv-tt">
              <span class="cv-ellip">{{ row.name }}</span>
              <small v-if="row.class || row.department">
                {{ [row.class, row.department].filter(Boolean).join(' · ') }}
              </small>
            </span>
          </span>
          <span class="cv-muted cv-num cv-ellip">{{ row.code || '—' }}</span>
          <span class="cv-sec cv-ellip">{{ row.teachers || '—' }}</span>
          <span class="cv-r cv-num">{{ row.credits ?? '—' }}</span>
          <span class="cv-muted cv-ellip">{{ row.campus || '—' }}</span>
          <span class="cv-muted cv-ellip">{{ row.timePlace || '—' }}</span>
        </div>
        <div
          v-if="truncated"
          class="cv-tr"
          style="--cols: minmax(160px, 1fr) 86px minmax(90px, 1fr) 56px 64px 130px"
        >
          <span class="cv-sec cv-ellip">还有 {{ filtered.length - rows.length }} 条</span>
          <span></span><span></span><span></span><span></span>
          <span class="cv-r cv-muted">缩小筛选或搜索课号</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cv-tr.course-row {
  height: 40px;
}
.th {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 0;
  border: 0;
  background: none;
  color: inherit;
  font: inherit;
  white-space: nowrap;
  cursor: pointer;
}
.th:hover {
  color: var(--text);
}
.arw {
  color: var(--text-tertiary);
}
</style>
