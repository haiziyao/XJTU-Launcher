<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { ClipboardCheck, ExternalLink, Search } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import { bridge } from '../../../shared/bridge';
import { loadLmsTodos } from '../campusOverview';
import { applyCached } from '../../../shared/cache';
import {
  campusTimestamp,
  courseHue,
  dueInfo,
  todoBuckets,
  updatedInfo,
} from '../../../shared/domain';
import type { EvaluationList, LmsTodoItem, LmsTodos } from '../../../types';
import { campusTodoCount } from '../campusNav';

/**
 * 待办 (layout v2): the homework the school's official todo list still keeps
 * open, in one dense table with a filter index on the left and the selected
 * item on the right. Submissions stay on the campus website; this page only
 * reads and groups what is due.
 */
interface TodoRow extends LmsTodoItem {
  dueAt: number | null;
}
type Filter = 'all' | 'overdue' | 'today' | 'week' | 'later';
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
const courseFilter = ref<string | null>(null);
const selectedId = ref<number | null>(null);
const evaluationCount = ref<number | null>(null);

const allBuckets = computed(() => todoBuckets(rows.value, now.value));
const matched = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return rows.value.filter(
    (item) =>
      (!keyword || `${item.title} ${item.course}`.toLowerCase().includes(keyword)) &&
      (!courseFilter.value || item.course === courseFilter.value),
  );
});
const visibleBuckets = computed(() => {
  const groups = todoBuckets(matched.value, now.value);
  if (filter.value === 'all') return groups;
  if (filter.value === 'week')
    return groups.filter((group) => ['today', 'tomorrow', 'week'].includes(group.id));
  if (filter.value === 'later') return groups.filter((group) => group.id === 'later');
  return groups.filter((group) => group.id === filter.value);
});
const meta = computed(() => {
  if (!rows.value.length) return semester.value || '';
  const updated = loadedAt.value ? ` · ${updatedInfo(loadedAt.value, now.value)}` : '';
  return `${semester.value || '当前学期'} · ${rows.value.length} 项未交${updated}`;
});

function countBucket(...ids: string[]): number {
  return allBuckets.value
    .filter((group) => ids.includes(group.id))
    .reduce((total, group) => total + group.items.length, 0);
}
const todayCount = computed(() => countBucket('today'));
const weekCount = computed(() => countBucket('today', 'tomorrow', 'week'));
const laterCount = computed(() => countBucket('later', 'none'));
const overdueCount = computed(() => countBucket('overdue'));
const courses = computed(() => {
  const counts = new Map<string, number>();
  for (const item of rows.value) {
    const name = item.course || '未标注课程';
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [...counts.entries()]
    .map(([name, count]) => ({ name, count }))
    .sort((left, right) => right.count - left.count || left.name.localeCompare(right.name));
});
const selected = computed(() => rows.value.find((item) => item.id === selectedId.value) ?? null);

const tone = (id: string): Tone => {
  if (id === 'overdue') return 'overdue';
  if (id === 'today') return 'today';
  if (id === 'tomorrow' || id === 'week') return 'soon';
  return 'later';
};
const due = (item: TodoRow) => dueInfo(item.dueAt, now.value);
const courseClass = (name: string): string => `c${courseHue(name) + 1}`;
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

function pickFilter(next: Filter) {
  filter.value = filter.value === next ? 'all' : next;
}

function pickCourse(name: string) {
  courseFilter.value = courseFilter.value === name ? null : name;
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
    // The shortcut is not the page; a failed count must not hide 待办.
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

async function openLms() {
  try {
    await bridge.openUrl('https://lms.xjtu.edu.cn');
  } catch (e) {
    console.error('[campus] open lms', e);
    error.value = e instanceof Error ? e.message : String(e);
  }
}

// Keep one row selected so the detail column is never an empty third of the page.
watch(
  visibleBuckets,
  (groups) => {
    const first = groups.flatMap((group) => group.items)[0];
    if (!first) {
      selectedId.value = null;
    } else if (
      !selectedId.value ||
      !groups.some((g) => g.items.some((i) => i.id === selectedId.value))
    ) {
      selectedId.value = first.id;
    }
  },
  { immediate: true },
);

let clock = 0;
onMounted(() => {
  void load();
  clock = window.setInterval(() => (now.value = Date.now()), 60_000);
});
onUnmounted(() => window.clearInterval(clock));
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
    <span v-if="failed" class="cv-due-soon">· {{ failed }} 个请求失败</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <label class="cv-search">
      <Search :size="13" aria-hidden="true" />
      <input
        v-model="query"
        type="search"
        placeholder="搜索作业或课程"
        aria-label="搜索待办"
        spellcheck="false"
      />
    </label>
    <button type="button" class="cv-btn" :disabled="loading" @click="load(true)">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols lcr">
    <div class="cv-panel cv-list">
      <div class="cv-lh">截止</div>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: filter === 'all' }"
        @click="
          filter = 'all';
          courseFilter = null;
        "
      >
        <b>全部未交</b><span class="cv-k accent">{{ rows.length }}</span>
      </button>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: filter === 'today' }"
        @click="pickFilter('today')"
      >
        <b>今天</b>
        <span :class="['cv-k', todayCount ? 'danger' : 'muted']">{{ todayCount }}</span>
      </button>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: filter === 'week' }"
        @click="pickFilter('week')"
      >
        <b>本周</b>
        <span :class="['cv-k', weekCount ? 'warn' : 'muted']">{{ weekCount }}</span>
      </button>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: filter === 'later' }"
        @click="pickFilter('later')"
      >
        <b>以后</b><span class="cv-n n">{{ laterCount }}</span>
      </button>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: filter === 'overdue' }"
        @click="pickFilter('overdue')"
      >
        <b>逾期</b>
        <span :class="['cv-k', overdueCount ? 'danger' : 'muted']">{{ overdueCount }}</span>
      </button>

      <div class="cv-lh">课程</div>
      <button
        v-for="course in courses"
        :key="course.name"
        type="button"
        class="cv-item"
        :class="{ on: courseFilter === course.name }"
        @click="pickCourse(course.name)"
      >
        <i class="cv-cs" :class="courseClass(course.name)" aria-hidden="true"></i>
        <b>{{ course.name }}</b>
        <span class="cv-n">{{ course.count }}</span>
      </button>

      <div class="cv-lh">其他</div>
      <button type="button" class="cv-item plain" @click="emit('open-section', 'evaluations')">
        <b>待评教</b>
        <span class="cv-link">{{ evaluationCount ?? '—' }} ›</span>
      </button>
    </div>

    <div class="cv-center">
      <div v-if="loading && !rows.length" class="cv-empty">
        <ClipboardCheck :size="22" /><b>正在读取思源学堂待办…</b>
        <span>直接使用学校的官方待办列表，之后会先使用缓存。</span>
      </div>
      <div v-else-if="error && !rows.length" class="cv-empty">
        <ClipboardCheck :size="22" /><b>待办没有读取成功</b><span>{{ error }}</span>
      </div>
      <div v-else-if="!rows.length" class="cv-empty">
        <ClipboardCheck :size="22" /><b>当前学期没有待交作业</b>
        <span>这里只显示尚未提交、且老师尚未截止的作业。</span>
      </div>
      <div v-else-if="!visibleBuckets.length" class="cv-empty">
        <ClipboardCheck :size="22" /><b>没有符合筛选条件的待办</b>
        <span>换一个截止范围、课程或关键词试试。</span>
      </div>

      <div v-else class="cv-panel cv-tbl">
        <div class="cv-thead" style="--cols: 1fr 96px 60px">
          <span>作业</span><span class="cv-r">截止</span><span class="cv-r">剩余</span>
        </div>
        <template v-for="group in visibleBuckets" :key="group.id">
          <div
            class="cv-grp"
            :class="{
              today: group.id === 'today' || group.id === 'overdue',
              soon: group.id === 'tomorrow' || group.id === 'week',
            }"
          >
            {{ group.title }}<span class="n">{{ group.items.length }}</span>
            <span class="cv-muted" style="margin-left: auto; font-weight: 400">{{
              bandMeta(group)
            }}</span>
          </div>
          <button
            v-for="item in group.items"
            :key="item.id"
            type="button"
            class="cv-tr"
            style="--cols: 1fr 96px 60px"
            :class="{ sel: selectedId === item.id }"
            @click="selectedId = item.id"
          >
            <span class="c">
              <i
                class="cv-cs"
                :class="courseClass(item.course || '未标注课程')"
                aria-hidden="true"
              ></i>
              <span class="cv-tt">
                <span class="cv-ellip">{{ item.title }}</span>
                <small>{{ item.course || '未标注课程' }}</small>
              </span>
            </span>
            <span
              class="cv-r cv-num"
              :class="{
                'cv-due-today': tone(group.id) === 'today' || tone(group.id) === 'overdue',
                'cv-due-soon': tone(group.id) === 'soon',
              }"
            >
              {{ due(item).text }}
            </span>
            <span
              class="cv-r cv-num"
              :class="{
                'cv-due-today': tone(group.id) === 'today' || tone(group.id) === 'overdue',
                'cv-due-soon': tone(group.id) === 'soon',
                'cv-muted': tone(group.id) === 'later',
              }"
            >
              {{ due(item).countdown || '—' }}
            </span>
          </button>
        </template>
      </div>
    </div>

    <div class="cv-panel cv-detail">
      <template v-if="selected">
        <div class="cv-type">
          <i
            class="cv-cs"
            :class="courseClass(selected.course || '未标注课程')"
            aria-hidden="true"
          ></i>
          {{ selected.course || '未标注课程' }} · 作业
          <span class="cv-badge" :class="selected.state === 'overdue' ? 'danger' : 'accent'">
            {{ selected.state === 'overdue' ? '已逾期' : '未交' }}
          </span>
        </div>
        <h3>{{ selected.title }}</h3>
        <div
          style="font-size: 12px"
          :class="
            due(selected).countdown
              ? selected.dueAt && selected.dueAt < now
                ? 'cv-due-today'
                : 'cv-sec'
              : 'cv-muted'
          "
        >
          {{ selected.dueAt ? `${due(selected).text} 截止` : '没有截止时间' }}
          <template v-if="due(selected).countdown"> · 还剩 {{ due(selected).countdown }}</template>
        </div>
        <dl class="cv-kv">
          <dt>学期</dt>
          <dd>{{ selected.semester || semester || '—' }}</dd>
          <dt>课程号</dt>
          <dd class="cv-num">{{ selected.courseCode || '—' }}</dd>
          <dt>截止</dt>
          <dd class="cv-num">{{ selected.end || '—' }}</dd>
        </dl>
        <div class="cv-blk">
          <h4>提交</h4>
          <p>应用内只读。作业在思源学堂网页提交，提交后这里会自动更新。</p>
        </div>
        <div class="cv-foot">
          <button type="button" class="cv-btn primary" @click="openLms">
            <ExternalLink :size="13" />在思源学堂提交
          </button>
        </div>
      </template>
      <div v-else class="cv-placeholder">选择左侧的一项作业查看详情。</div>
    </div>
  </div>
</template>
