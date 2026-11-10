<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { ClipboardCheck, ExternalLink, Search } from 'lucide-vue-next';
import SectionHeader from '../../../components/ui/SectionHeader.vue';
import EmptyState from '../../../components/ui/EmptyState.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import MoreMenu, { type MoreItem } from '../../../components/ui/MoreMenu.vue';
import UiButton from '../../../components/ui/UiButton.vue';
import { bridge } from '../../../shared/bridge';
import { loadLmsTodos } from '../campusOverview';
import { applyCached } from '../../../shared/cache';
import { campusTimestamp, dueInfo, todoBuckets } from '../../../shared/domain';
import type { EvaluationList, LmsTodoItem, LmsTodos } from '../../../types';
import { campusTodoCount } from '../campusNav';

/**
 * 待办 (LAYOUT §9.2, version B): the homework the school's official todo list
 * still keeps open, grouped into one dense table. Submissions stay on the
 * campus website; this page only reads and groups what is due.
 */
interface TodoRow extends LmsTodoItem {
  dueAt: number | null;
}
type Filter = 'all' | 'overdue' | 'today' | 'week';
type Tone = 'overdue' | 'today' | 'soon' | 'later';

const emit = defineEmits<{ 'open-section': [id: string] }>();
const rows = ref<TodoRow[]>([]);
const semester = ref('');
const failed = ref(0);
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
const now = ref(Date.now());
const query = ref('');
const filter = ref<Filter>('all');
const evaluationCount = ref<number | null>(null);
const moreItems: MoreItem[] = [{ id: 'lms', label: '在思源学堂中打开', icon: ExternalLink }];

const allBuckets = computed(() => todoBuckets(rows.value, now.value));
const matched = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return rows.value;
  return rows.value.filter((item) =>
    `${item.title} ${item.course}`.toLowerCase().includes(keyword),
  );
});
const visibleBuckets = computed(() => {
  const groups = todoBuckets(matched.value, now.value);
  if (filter.value === 'all') return groups;
  if (filter.value === 'week')
    return groups.filter((group) => ['today', 'tomorrow', 'week'].includes(group.id));
  return groups.filter((group) => group.id === filter.value);
});
const subtitle = computed(() =>
  rows.value.length ? `${semester.value || '当前学期'} · ${rows.value.length} 项待交` : '',
);

function countBucket(id: string): number {
  return allBuckets.value.find((group) => group.id === id)?.items.length ?? 0;
}
const todayCount = computed(() => countBucket('today'));
const overdueCount = computed(() => countBucket('overdue'));
const courseCount = computed(() => new Set(rows.value.map((item) => item.course)).size);
const todaySub = computed(() => {
  const first = allBuckets.value.find((group) => group.id === 'today')?.items[0];
  return first ? dueInfo(first.dueAt, now.value).countdown : '今天没有截止';
});
const stats = computed(() => [
  {
    key: 'open' as const,
    label: '未交',
    value: rows.value.length,
    sub: `${courseCount.value} 门课程`,
    filter: 'all' as Filter,
  },
  {
    key: 'today' as const,
    label: '今天截止',
    value: todayCount.value,
    sub: todaySub.value,
    filter: 'today' as Filter,
    hot: todayCount.value > 0,
  },
  {
    key: 'week' as const,
    label: '本周',
    value: todayCount.value + countBucket('tomorrow') + countBucket('week'),
    sub: todayCount.value ? '含今天' : '本周内',
    filter: 'week' as Filter,
  },
  {
    key: 'overdue' as const,
    label: '逾期',
    value: overdueCount.value,
    sub: overdueCount.value ? '已过截止时间' : '没有逾期',
    filter: 'overdue' as Filter,
    danger: overdueCount.value > 0,
  },
  {
    key: 'evaluations' as const,
    label: '待评教',
    value: evaluationCount.value ?? '—',
    sub: evaluationCount.value == null ? '未读取' : evaluationCount.value ? '未完成' : '全部完成',
    section: 'evaluations',
    link: true,
  },
]);

const tone = (id: string): Tone => {
  if (id === 'overdue') return 'overdue';
  if (id === 'today') return 'today';
  if (id === 'tomorrow' || id === 'week') return 'soon';
  return 'later';
};
const due = (item: TodoRow) => dueInfo(item.dueAt, now.value);
const bandMeta = (group: { id: string; items: TodoRow[] }): string => {
  if (group.id === 'overdue') return '已过截止时间';
  if (group.id === 'today') return `最晚${due(group.items[group.items.length - 1]).text}`;
  if (group.id === 'tomorrow') return '明天截止';
  if (group.id === 'week') return '本周内';
  if (group.id === 'none') return '无截止时间';
  return `共 ${group.items.length} 项`;
};

function toRows(list: LmsTodoItem[]): TodoRow[] {
  return list.map((item) => {
    const at = campusTimestamp(item.end);
    return { ...item, dueAt: Number.isFinite(at) ? at : null };
  });
}

async function loadEvaluations(force = false) {
  try {
    const list = (await bridge.schoolQuery(
      { type: 'evaluations', finished: false },
      'evaluations',
      force,
    )) as EvaluationList;
    evaluationCount.value = list.rows.length;
  } catch (e) {
    // The tile is a shortcut, not the page; a failed count must not hide 待办.
    console.error('[campus] todo evaluations', e);
    evaluationCount.value = null;
  }
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  void loadEvaluations(force);
  try {
    await applyCached<LmsTodos>('lmsTodos', undefined, (value) => {
      if (!rows.value.length) rows.value = toRows(value.items);
      if (!semester.value) semester.value = value.semester;
      failed.value = value.failures?.length ?? 0;
      campusTodoCount.value = value.items.length;
    });
    const fresh = await loadLmsTodos(force);
    rows.value = toRows(fresh.items);
    semester.value = fresh.semester;
    failed.value = fresh.failures?.length ?? 0;
    loadedAt.value = Date.now();
    campusTodoCount.value = fresh.items.length;
  } catch (e) {
    console.error('[campus] lms todos', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function onStat(stat: (typeof stats.value)[number]) {
  if ('section' in stat && stat.section) {
    emit('open-section', stat.section);
    return;
  }
  if ('filter' in stat && stat.filter) {
    filter.value = filter.value === stat.filter ? 'all' : stat.filter;
  }
}

function clearFilters() {
  filter.value = 'all';
  query.value = '';
}

function onMore(id: string) {
  if (id === 'lms') void bridge.openUrl('https://lms.xjtu.edu.cn');
}

async function openLms() {
  error.value = '';
  try {
    await bridge.openUrl('https://lms.xjtu.edu.cn');
  } catch (e) {
    console.error('[campus] open lms', e);
    error.value = e instanceof Error ? e.message : String(e);
  }
}

let clock = 0;
onMounted(() => {
  void load();
  clock = window.setInterval(() => (now.value = Date.now()), 60_000);
});
onUnmounted(() => window.clearInterval(clock));
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="待办"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    >
      <template #filters>
        <label class="field-shell search todo-search">
          <Search :size="13" aria-hidden="true" />
          <input
            v-model="query"
            type="search"
            placeholder="搜索作业或课程"
            aria-label="搜索待办"
            spellcheck="false"
          />
        </label>
        <UiButton v-if="filter !== 'all' || query" variant="ghost" size="sm" @click="clearFilters">
          清除筛选
        </UiButton>
      </template>
      <template #more><MoreMenu :items="moreItems" @select="onMore" /></template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="failed" tone="partial" title="部分内容未能读取">
      {{ failed }} 个课程或作业请求失败，列表可能不完整；刷新可重试。
    </InlineAlert>

    <div v-if="rows.length" class="campus-stat-row">
      <button
        v-for="stat in stats"
        :key="stat.key"
        type="button"
        class="campus-stat"
        :class="{
          hot: 'hot' in stat && stat.hot,
          danger: 'danger' in stat && stat.danger,
          link: 'link' in stat && stat.link,
          active: 'filter' in stat && stat.filter !== 'all' && filter === stat.filter,
        }"
        @click="onStat(stat)"
      >
        <span class="k">{{ stat.label }}</span>
        <span class="v">{{ stat.value }}</span>
        <span class="s">{{ stat.sub }}</span>
      </button>
    </div>

    <EmptyState
      v-if="loading && !rows.length"
      :icon="ClipboardCheck"
      compact
      busy
      title="正在读取思源学堂待办…"
      description="直接使用学校的官方待办列表，之后会先使用缓存。"
    />
    <EmptyState
      v-else-if="!rows.length && !error"
      :icon="ClipboardCheck"
      title="当前学期没有待交作业"
      description="这里只显示尚未提交、且老师尚未截止的作业。"
    />
    <EmptyState
      v-else-if="!visibleBuckets.length"
      :icon="ClipboardCheck"
      compact
      title="没有符合筛选条件的待办"
      description="清除筛选或换一个关键词试试。"
    />

    <div v-if="visibleBuckets.length" class="campus-table">
      <div class="campus-table-head">
        <span></span>
        <span>作业</span>
        <span>课程</span>
        <span class="num">截止</span>
        <span class="num remain">剩余</span>
        <span class="num">操作</span>
      </div>
      <section
        v-for="group in visibleBuckets"
        :key="group.id"
        class="campus-band"
        :data-tone="tone(group.id)"
      >
        <div class="campus-band-head">
          <span>{{ group.title }}</span>
          <span class="campus-band-count">{{ group.items.length }}</span>
          <span class="campus-band-meta">{{ bandMeta(group) }}</span>
        </div>
        <div v-for="item in group.items" :key="item.id" class="campus-row">
          <span><i class="campus-dot" :data-tone="tone(group.id)" aria-hidden="true"></i></span>
          <span class="name">{{ item.title }}</span>
          <span class="course">{{ item.course || '未标注课程' }}</span>
          <span class="num">{{ due(item).text }}</span>
          <span class="num muted remain">{{ due(item).countdown || '—' }}</span>
          <button type="button" class="row-action" title="在思源学堂网页打开" @click="openLms">
            打开
          </button>
        </div>
      </section>
    </div>
  </section>
</template>

<style scoped>
.todo-search {
  width: 220px;
}
</style>
