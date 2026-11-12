<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ExternalLink, GraduationCap, Search } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { gradeStats, updatedInfo } from '../../../shared/domain';
import type { GradeRow, Grades, Terms } from '../../../types';

/**
 * 成绩 (layout v2): weighted figures and the semester / category filters in the
 * left column, one table in the centre. Nothing here is submitted anywhere.
 */
const terms = ref<Terms | null>(null);
const grades = ref<Grades | null>(null);
const term = ref('');
const category = ref('');
const query = ref('');
type SortKey = 'name' | 'code' | 'category' | 'credits' | 'score' | 'gpa' | 'term';
const sort = ref<{ key: SortKey; dir: 'asc' | 'desc' } | null>({ key: 'gpa', dir: 'desc' });
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
// Only offer semesters that appear in the complete grade list, so choosing a
// term cannot land on an empty table. The set is refreshed only from the
// unfiltered query; a per-term query returns that term's rows alone and must
// not shrink the list.
const availableTermIds = ref<Set<string>>(new Set());
const termOptions = computed(() => [
  { value: '', label: '全部学期' },
  ...(terms.value?.rows ?? [])
    .filter((item) => availableTermIds.value.has(item.id))
    .map((item) => ({ value: item.id, label: item.name })),
]);
const termName = computed(
  () => termOptions.value.find((item) => item.value === term.value)?.label ?? term.value,
);
const rows = computed<GradeRow[]>(() => grades.value?.rows ?? []);
const categories = computed(() =>
  [...new Set(rows.value.map((row) => row.category).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b, 'zh-Hans-CN'),
  ),
);
const categoryCounts = computed(() => {
  const counts: Record<string, number> = {};
  for (const row of rows.value) {
    if (row.category) counts[row.category] = (counts[row.category] ?? 0) + 1;
  }
  return counts;
});
const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return rows.value.filter((row) => {
    if (category.value && row.category !== category.value) return false;
    if (!keyword) return true;
    return `${row.name} ${row.code}`.toLowerCase().includes(keyword);
  });
});
const sorted = computed(() => {
  const current = sort.value;
  if (!current) return visible.value;
  const factor = current.dir === 'asc' ? 1 : -1;
  return [...visible.value].sort((left, right) => {
    const a = left[current.key];
    const b = right[current.key];
    if (a == null) return 1;
    if (b == null) return -1;
    if (current.key === 'credits' || current.key === 'gpa') {
      return (Number(a) - Number(b)) * factor;
    }
    const aNumber = Number(a);
    const bNumber = Number(b);
    if (current.key === 'score' && Number.isFinite(aNumber) && Number.isFinite(bNumber)) {
      return (aNumber - bNumber) * factor;
    }
    return String(a).localeCompare(String(b), 'zh-Hans-CN', { numeric: true }) * factor;
  });
});
const stats = computed(() => gradeStats(visible.value));
const summary = computed(() => {
  if (!visible.value.length) return '';
  return [
    `${visible.value.length} 门`,
    `${stats.value.credits.toFixed(1)} 学分`,
    stats.value.gpa == null ? '' : `绩点 ${stats.value.gpa.toFixed(2)}`,
    stats.value.average == null ? '' : `均分 ${stats.value.average.toFixed(1)}`,
  ]
    .filter(Boolean)
    .join(' · ');
});
const statNote = computed(() => {
  const parts = ['按本页数据计算，以学校系统为准'];
  if (stats.value.unscored) parts.push(`${stats.value.unscored} 门未计入均分`);
  return parts.join(' · ');
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
  try {
    await applyCached<Terms>('terms', undefined, (value) => (terms.value = value));
    terms.value = (await bridge.schoolQuery({ type: 'terms' }, 'terms', force)) as Terms;
  } catch (e) {
    console.error('[campus] grade terms', e);
  }
}

function rememberTerms(value: Grades | null, filtered: boolean) {
  if (filtered || !value) return;
  availableTermIds.value = new Set(value.rows.map((row) => row.term).filter(Boolean));
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  const params = { term: term.value || null };
  try {
    await applyCached<Grades>('grades', params, (value) => {
      grades.value = value;
      rememberTerms(value, params.term != null);
    });
    grades.value = (await bridge.schoolQuery(
      { type: 'grades', term: params.term },
      cacheName('grades', params),
      force,
    )) as Grades;
    rememberTerms(grades.value, params.term != null);
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] grades', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeTerm(value: string) {
  term.value = value;
  void load();
}

function changeCategory(value: string) {
  category.value = value;
}

function clear() {
  query.value = '';
  category.value = '';
  if (term.value) {
    term.value = '';
    void load();
  }
}

function onMore(id: string) {
  if (id === 'jwxt') void bridge.openUrl('https://jwxt.xjtu.edu.cn');
}

onMounted(async () => {
  await loadTerms();
  await load();
});
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
        placeholder="搜索课程名或课程号"
        aria-label="搜索成绩"
        spellcheck="false"
      />
    </label>
    <button type="button" class="cv-btn ghost" @click="onMore('jwxt')">
      <ExternalLink :size="13" />教务系统
    </button>
    <button type="button" class="cv-btn" :disabled="loading" @click="load(true)">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-figs">
        <div class="cv-fig">
          <span class="k2">加权绩点</span>
          <span class="v">{{ stats.gpa == null ? '—' : stats.gpa.toFixed(2) }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">加权均分</span>
          <span class="v">{{ stats.average == null ? '—' : stats.average.toFixed(1) }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">已修学分</span>
          <span class="v">{{ stats.credits ? stats.credits.toFixed(1) : '—' }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">课程</span>
          <span class="v">{{ stats.courses }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">不及格</span>
          <span class="v">
            {{ stats.failed
            }}<small v-if="stats.failedCredits">{{ stats.failedCredits }} 学分</small>
          </span>
        </div>
      </div>
      <div v-if="rows.length" class="cv-nodata g-note">{{ statNote }}</div>
      <div class="cv-hr"></div>

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

      <div class="cv-lh">课程性质</div>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: !category }"
        @click="changeCategory('')"
      >
        <b>全部</b><span class="cv-n">{{ rows.length }}</span>
      </button>
      <button
        v-for="value in categories"
        :key="value"
        type="button"
        class="cv-item plain"
        :class="{ on: category === value }"
        @click="changeCategory(value)"
      >
        <b>{{ value }}</b
        ><span class="cv-n">{{ categoryCounts[value] ?? 0 }}</span>
      </button>

      <div v-if="term || category || query" class="g-clear">
        <button type="button" class="cv-link" @click="clear">清除筛选</button>
      </div>
    </div>

    <div class="cv-center">
      <InlineAlert v-if="error" tone="error" :title="error" />
      <div class="cv-ch">
        <b>{{ termName }}</b>
        <span class="meta">{{ summary }}</span>
      </div>

      <div v-if="loading && !rows.length" class="cv-empty">
        <GraduationCap :size="22" /><b>正在读取成绩…</b>
      </div>
      <div v-else-if="!rows.length" class="cv-empty">
        <GraduationCap :size="22" /><b>还没有成绩记录</b>
      </div>
      <div v-else-if="!visible.length" class="cv-empty">
        <GraduationCap :size="22" /><b>没有符合筛选条件的课程</b>
      </div>
      <div v-else class="cv-panel cv-tbl">
        <div class="cv-thead" style="--cols: minmax(150px, 1fr) 88px 62px 50px 54px 48px 92px">
          <button type="button" class="th" @click="sortBy('name')">
            课程<span class="arw">{{ arrow('name') }}</span>
          </button>
          <span>课号</span>
          <button type="button" class="th" @click="sortBy('category')">
            性质<span class="arw">{{ arrow('category') }}</span>
          </button>
          <span class="cv-r">
            <button type="button" class="th" @click="sortBy('credits')">
              学分<span class="arw">{{ arrow('credits') }}</span>
            </button>
          </span>
          <span class="cv-r">
            <button type="button" class="th" @click="sortBy('score')">
              成绩<span class="arw">{{ arrow('score') }}</span>
            </button>
          </span>
          <span class="cv-r">
            <button type="button" class="th" @click="sortBy('gpa')">
              绩点<span class="arw">{{ arrow('gpa') }}</span>
            </button>
          </span>
          <button type="button" class="th" @click="sortBy('term')">
            学期<span class="arw">{{ arrow('term') }}</span>
          </button>
        </div>
        <div
          v-for="row in sorted"
          :key="row.id"
          class="cv-tr"
          style="--cols: minmax(150px, 1fr) 88px 62px 50px 54px 48px 92px"
        >
          <span class="cv-ellip g-name">{{ row.name }}</span>
          <span class="cv-muted cv-num cv-ellip">{{ row.code || '—' }}</span>
          <span class="cv-sec cv-ellip">{{ row.category || '—' }}</span>
          <span class="cv-r cv-num">{{ row.credits ?? '—' }}</span>
          <span class="cv-r cv-num" :class="{ 'g-fail': row.passed === false }">
            {{ row.score || '—' }}
          </span>
          <span class="cv-r cv-num">{{ row.gpa == null ? '—' : row.gpa }}</span>
          <span class="cv-ellip cv-muted">{{ row.term || '—' }}</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.g-note {
  padding: 0 8px 4px;
}
.g-clear {
  margin-top: 8px;
  padding: 0 8px 4px;
}
.g-name {
  font-weight: 500;
}
.g-fail {
  color: var(--danger);
  font-weight: 600;
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
