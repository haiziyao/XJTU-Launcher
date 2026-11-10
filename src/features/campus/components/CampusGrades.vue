<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ExternalLink } from 'lucide-vue-next';
import SectionHeader from '../../../components/ui/SectionHeader.vue';
import StatRow from '../../../components/ui/StatRow.vue';
import StatTile from '../../../components/ui/StatTile.vue';
import FilterBar from '../../../components/ui/FilterBar.vue';
import DataTable, { type DataColumn, type DataSort } from '../../../components/ui/DataTable.vue';
import MenuSelect from '../../../components/ui/MenuSelect.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import MoreMenu, { type MoreItem } from '../../../components/ui/MoreMenu.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { gradeStats } from '../../../shared/domain';
import type { GradeRow, Grades, Terms } from '../../../types';

/**
 * 成绩 (LAYOUT §9.4): term and category filters, weighted figures from the rows
 * on screen, and one sortable table. Nothing here is submitted anywhere.
 */
const terms = ref<Terms | null>(null);
const grades = ref<Grades | null>(null);
const term = ref('');
const category = ref('');
const query = ref('');
const sort = ref<DataSort | null>({ key: 'gpa', dir: 'desc' });
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
// Only offer terms that appear in the complete grade list, so choosing a term
// cannot land on an empty table.
const availableTermIds = ref<Set<string> | null>(null);

const termOptions = computed(() => [
  { value: '', label: '全部学期' },
  ...(terms.value?.rows ?? [])
    .filter((item) => !availableTermIds.value || availableTermIds.value.has(item.id))
    .map((item) => ({ value: item.id, label: item.name })),
]);
const rows = computed<GradeRow[]>(() => grades.value?.rows ?? []);
const categories = computed(() =>
  [...new Set(rows.value.map((row) => row.category).filter(Boolean))].sort((a, b) =>
    a.localeCompare(b, 'zh-Hans-CN'),
  ),
);
const categoryOptions = computed(() => [
  { value: '', label: '全部性质' },
  ...categories.value.map((value) => ({ value, label: value })),
]);
const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return rows.value.filter((row) => {
    if (category.value && row.category !== category.value) return false;
    if (!keyword) return true;
    return `${row.name} ${row.code}`.toLowerCase().includes(keyword);
  });
});
const tableRows = computed<Record<string, unknown>[]>(() =>
  visible.value.map((row) => ({ ...row, failed: row.passed === false })),
);
const stats = computed(() => gradeStats(visible.value));
const statNote = computed(() => {
  const parts = ['按本页数据计算，以学校系统为准'];
  if (stats.value.unscored) parts.push(`${stats.value.unscored} 门未计入均分`);
  return parts.join(' · ');
});
const subtitle = computed(() => {
  if (!rows.value.length) return '';
  if (visible.value.length !== rows.value.length)
    return `${visible.value.length} / ${rows.value.length} 门`;
  return `${rows.value.length} 门课程`;
});
const columns: DataColumn[] = [
  { key: 'name', label: '课程', sortable: true },
  { key: 'category', label: '性质', sortable: true },
  { key: 'credits', label: '学分', align: 'right', sortable: true },
  { key: 'score', label: '成绩', align: 'right', sortable: true },
  { key: 'gpa', label: '绩点', align: 'right', sortable: true },
  { key: 'term', label: '学期', sortable: true },
];
const moreItems: MoreItem[] = [{ id: 'jwxt', label: '在教务系统中打开', icon: ExternalLink }];

async function loadTerms(force = false) {
  try {
    await applyCached<Terms>('terms', undefined, (value) => (terms.value = value));
    terms.value = (await bridge.schoolQuery({ type: 'terms' }, 'terms', force)) as Terms;
  } catch (e) {
    console.error('[campus] grade terms', e);
  }
}

function rememberTerms(rows: GradeRow[]) {
  if (term.value) return;
  availableTermIds.value = new Set(rows.map((row) => row.term).filter(Boolean));
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  const params = { term: term.value || null };
  try {
    await applyCached<Grades>('grades', params, (value) => {
      grades.value = value;
      if (!params.term) rememberTerms(value.rows);
    });
    const fresh = (await bridge.schoolQuery(
      { type: 'grades', term: params.term },
      cacheName('grades', params),
      force,
    )) as Grades;
    grades.value = fresh;
    if (!params.term) rememberTerms(fresh.rows);
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
  <section class="campus-section">
    <SectionHeader
      title="成绩"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    >
      <template #filters>
        <MenuSelect
          :model-value="term"
          :options="termOptions"
          label="学期"
          placeholder="全部学期"
          fit
          @update:model-value="changeTerm"
        />
        <MenuSelect
          :model-value="category"
          :options="categoryOptions"
          label="课程性质"
          placeholder="全部性质"
          fit
          @update:model-value="changeCategory"
        />
      </template>
      <template #more><MoreMenu :items="moreItems" @select="onMore" /></template>
    </SectionHeader>

    <StatRow label="成绩概况">
      <StatTile
        label="加权绩点"
        :value="stats.gpa == null ? null : stats.gpa.toFixed(2)"
        :loading="loading && !rows.length"
      />
      <StatTile
        label="加权均分"
        :value="stats.average == null ? null : stats.average.toFixed(1)"
        :loading="loading && !rows.length"
      />
      <StatTile
        label="已修学分"
        :value="stats.credits ? stats.credits.toFixed(1) : null"
        :loading="loading && !rows.length"
      />
      <StatTile label="课程" :value="stats.courses" :loading="loading && !rows.length" />
      <StatTile
        label="不及格"
        :value="stats.failed"
        :loading="loading && !rows.length"
        :note="stats.failedCredits ? `${stats.failedCredits} 学分` : ''"
      />
    </StatRow>
    <p class="campus-stat-note">{{ statNote }}</p>

    <FilterBar
      :query="query"
      :active="Boolean(term || category)"
      placeholder="搜索课程名或课程号"
      label="搜索成绩"
      @update:query="query = $event"
      @clear="clear"
    />

    <InlineAlert v-if="error" tone="error" :title="error" />

    <DataTable
      v-model:sort="sort"
      :columns="columns"
      :rows="tableRows"
      row-key="id"
      :loading="loading && !rows.length"
      :empty-text="rows.length ? '没有符合筛选条件的课程' : '还没有成绩记录'"
      label="成绩表"
    >
      <template #cell-name="{ row }">
        <span class="grade-name">{{ row.name }}</span>
      </template>
      <template #cell-credits="{ row }">
        {{ row.credits == null ? '—' : row.credits }}
      </template>
      <template #cell-score="{ row }">
        <span :class="{ 'grade-failed': row.failed }">{{ row.score || '—' }}</span>
      </template>
      <template #cell-gpa="{ row }">{{ row.gpa == null ? '—' : row.gpa }}</template>
      <template #cell-term="{ row }">{{ row.term || '—' }}</template>
    </DataTable>
  </section>
</template>

<style scoped>
.campus-stat-note {
  margin: -8px 0 14px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.grade-name {
  font-weight: 500;
}
.grade-failed {
  color: var(--danger);
  font-weight: 600;
}
</style>
