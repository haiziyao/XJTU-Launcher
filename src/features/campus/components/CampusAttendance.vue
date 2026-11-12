<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import {
  attendanceStats,
  campusDateKey,
  campusTimestamp,
  percent,
  updatedInfo,
} from '../../../shared/domain';
import type { AttendanceRecord, AttendanceRecords, AttendanceStreams } from '../../../types';

/**
 * 考勤 (layout v2): course records for the last 30 days with their rates in the
 * left column, the record table in the centre, and the swipe stream behind the
 * same header switch. The attendance service has no course name; the backend
 * fills it from the synced timetable by weekday, week and sections.
 */
const TABS = [
  { id: 'records', title: '明细' },
  { id: 'streams', title: '刷卡流水' },
];
const states = ref({ records: 1, streams: 1 });
const tab = ref('records');
const records = ref<AttendanceRecords | null>(null);
const streams = ref<AttendanceStreams | null>(null);
const filter = ref('all');
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
const sort = ref<{ key: 'date'; dir: 'asc' | 'desc' } | null>({ key: 'date', dir: 'desc' });

const range = computed(() => {
  const end = campusTimestamp(`${campusDateKey(Date.now())} 00:00`);
  return {
    start: campusDateKey(end - 29 * 86400000),
    end: campusDateKey(end),
  };
});
const recordRows = computed<AttendanceRecord[]>(() => records.value?.rows ?? []);
const stats = computed(() => attendanceStats(recordRows.value));
const exceptionCount = computed(
  () =>
    recordRows.value.filter(
      (row) => row.status === 'late' || row.status === 'absent' || row.status === 'leave',
    ).length,
);
const visible = computed(() =>
  recordRows.value.filter((row) => {
    if (filter.value === 'all') return true;
    return row.status === 'late' || row.status === 'absent' || row.status === 'leave';
  }),
);
const sortedRecords = computed(() => {
  const active = sort.value;
  if (!active) return visible.value;
  const factor = active.dir === 'asc' ? 1 : -1;
  return [...visible.value].sort((left, right) => left.date.localeCompare(right.date) * factor);
});
const STATUS: Record<AttendanceRecord['status'], { tone: string; label: string }> = {
  normal: { tone: 'ok', label: '正常' },
  late: { tone: 'warn', label: '迟到' },
  absent: { tone: 'danger', label: '缺勤' },
  leave: { tone: 'muted', label: '请假' },
  pending: { tone: 'accent', label: '未记录' },
  notRequired: { tone: 'muted', label: '无需考勤' },
  unknown: { tone: 'muted', label: '未知' },
};
const rateNote = computed(() => `按本页 ${recordRows.value.length} 条记录统计`);
const streamsTotal = computed(() => streams.value?.total ?? 0);
const recordsTotal = computed(() => records.value?.total ?? 0);
const moreRecords = computed(() => states.value.records * 50 < recordsTotal.value);
const moreStreams = computed(() => states.value.streams * 50 < streamsTotal.value);
const subtitle = computed(() =>
  tab.value === 'records' ? `${range.value.start} 至 ${range.value.end}` : '',
);
const meta = computed(() =>
  [subtitle.value, loadedAt.value ? `更新于 ${updatedInfo(loadedAt.value)}` : '']
    .filter(Boolean)
    .join(' · '),
);

function statusOf(status: unknown) {
  return STATUS[String(status) as AttendanceRecord['status']] ?? STATUS.unknown;
}
function sections(row: AttendanceRecord) {
  const start = row.start;
  const end = row.end;
  if (start == null && end == null) return '—';
  return `${start ?? '?'}–${end ?? '?'} 节`;
}
function sortBy(key: 'date') {
  const dir = sort.value?.key === key && sort.value.dir === 'asc' ? 'desc' : 'asc';
  sort.value = { key, dir };
}

async function loadRecords(force = false) {
  const params = { start: range.value.start, end: range.value.end, page: states.value.records };
  await applyCached<AttendanceRecords>('attendanceRecords', params, (value) => {
    records.value = value;
  });
  records.value = (await bridge.schoolQuery(
    { type: 'attendanceRecords', ...params },
    cacheName('attendanceRecords', params),
    force,
  )) as AttendanceRecords;
}

async function loadStreams(force = false) {
  const params = { page: states.value.streams };
  await applyCached<AttendanceStreams>('attendanceStreams', params, (value) => {
    streams.value = value;
  });
  streams.value = (await bridge.schoolQuery(
    { type: 'attendanceStreams', page: params.page },
    cacheName('attendanceStreams', params),
    force,
  )) as AttendanceStreams;
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  try {
    if (tab.value === 'streams') await loadStreams(force);
    else await loadRecords(force);
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] attendance', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function turn(delta: number) {
  const key = tab.value === 'streams' ? 'streams' : 'records';
  states.value[key] += delta;
  loading.value = true;
  error.value = '';
  try {
    if (key === 'streams') await loadStreams();
    else await loadRecords();
  } catch (e) {
    console.error('[campus] attendance page', e);
    error.value = e instanceof Error ? e.message : String(e);
    states.value[key] -= delta;
  } finally {
    loading.value = false;
  }
}

watch(tab, () => void load());
onMounted(() => void load());
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <button type="button" class="cv-btn" :disabled="loading" @click="load(true)">刷新</button>
  </CampusHeaderSlot>

  <InlineAlert v-if="error" tone="error" :title="error" />

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-figs">
        <div class="cv-fig">
          <span class="k2">到课率</span>
          <span class="v">{{ percent(stats.rate) }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">准时率</span>
          <span class="v">{{ percent(stats.onTimeRate) }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">迟到</span>
          <span class="v">{{ stats.late }}</span>
        </div>
        <div class="cv-fig">
          <span class="k2">缺勤 · 请假</span>
          <span class="v"
            >{{ stats.absent }}<small>· {{ stats.leave }}</small></span
          >
        </div>
      </div>
      <div class="cv-lh">{{ rateNote }}</div>
      <div class="cv-hr"></div>
      <template v-if="tab === 'records'">
        <div class="cv-lh">筛选<span>条数</span></div>
        <button
          type="button"
          class="cv-item plain"
          :class="{ on: filter === 'all' }"
          @click="filter = 'all'"
        >
          <b>全部记录</b><span class="cv-n">{{ recordRows.length }}</span>
        </button>
        <button
          type="button"
          class="cv-item plain"
          :class="{ on: filter === 'odd' }"
          @click="filter = 'odd'"
        >
          <b>只看异常</b>
          <span :class="['cv-k', exceptionCount ? 'danger' : 'muted']">{{ exceptionCount }}</span>
        </button>
      </template>
    </div>

    <div class="cv-center">
      <div class="cv-ch">
        <b>{{ tab === 'records' ? '刷卡记录' : '门禁流水' }}</b>
        <span v-if="tab === 'records'" class="meta">
          {{ recordRows.length }} 条 · 异常 {{ exceptionCount }} 条
        </span>
        <span v-else class="meta">{{ streamsTotal }} 条</span>
        <span class="end">
          <span class="cv-seg">
            <button
              v-for="item in TABS"
              :key="item.id"
              type="button"
              :class="{ on: tab === item.id }"
              @click="tab = item.id"
            >
              {{ item.title }}
            </button>
          </span>
        </span>
      </div>

      <div v-if="tab === 'records'" class="cv-scroll">
        <div v-if="loading && !records" class="cv-empty">正在读取考勤记录…</div>
        <div v-else-if="!sortedRecords.length" class="cv-empty">
          {{ recordRows.length ? '没有异常记录' : '这段时间没有考勤记录' }}
        </div>
        <div v-else class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 92px 76px 1.4fr 1.1fr 84px 62px">
            <button type="button" class="cv-th" @click="sortBy('date')">
              日期<span v-if="sort?.key === 'date'" aria-hidden="true">{{
                sort.dir === 'asc' ? '↑' : '↓'
              }}</span>
            </button>
            <span>节次</span><span>课程</span><span>地点</span><span>教师</span
            ><span class="cv-r">状态</span>
          </div>
          <div
            v-for="row in sortedRecords"
            :key="row.id"
            class="cv-tr"
            style="--cols: 92px 76px 1.4fr 1.1fr 84px 62px"
          >
            <span class="cv-num">{{ row.date || '—' }}</span>
            <span class="cv-num cv-sec">{{ sections(row) }}</span>
            <span class="cv-ellip">{{ row.course || '—' }}</span>
            <span class="cv-ellip">{{ row.place || '—' }}</span>
            <span class="cv-sec cv-ellip">{{ row.teacher || '—' }}</span>
            <span class="cv-r">
              <span class="cv-badge" :class="statusOf(row.status).tone">
                {{ statusOf(row.status).label }}
              </span>
            </span>
          </div>
        </div>
        <div class="cv-pager">
          <button
            type="button"
            class="cv-btn sm"
            :disabled="states.records <= 1 || loading"
            @click="turn(-1)"
          >
            上一页
          </button>
          <span>第 {{ records?.page ?? states.records }} 页 · 共 {{ recordsTotal }} 条</span>
          <button
            type="button"
            class="cv-btn sm"
            :disabled="loading || !moreRecords"
            @click="turn(1)"
          >
            下一页
          </button>
        </div>
      </div>

      <div v-else class="cv-scroll">
        <div v-if="loading && !streams" class="cv-empty">正在读取刷卡流水…</div>
        <div v-else-if="!streams?.rows.length" class="cv-empty">还没有刷卡流水</div>
        <div v-else class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 1fr 160px 74px">
            <span>时间</span><span>地点</span><span class="cv-r">结果</span>
          </div>
          <div
            v-for="row in streams?.rows ?? []"
            :key="row.id"
            class="cv-tr"
            style="--cols: 1fr 160px 74px"
          >
            <span class="cv-num">{{ row.time || '—' }}</span>
            <span class="cv-ellip">{{ row.place || '未知地点' }}</span>
            <span class="cv-r">
              <span class="cv-badge" :class="row.valid ? 'ok' : 'muted'">
                {{ row.valid ? '有效' : '无效' }}
              </span>
            </span>
          </div>
        </div>
        <div class="cv-pager">
          <button
            type="button"
            class="cv-btn sm"
            :disabled="states.streams <= 1 || loading"
            @click="turn(-1)"
          >
            上一页
          </button>
          <span>第 {{ streams?.page ?? states.streams }} 页 · 共 {{ streamsTotal }} 条</span>
          <button
            type="button"
            class="cv-btn sm"
            :disabled="loading || !moreStreams"
            @click="turn(1)"
          >
            下一页
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.cv-th {
  display: inline-flex;
  align-items: center;
  gap: 3px;
  padding: 0;
  border: 0;
  background: transparent;
  color: inherit;
  font: inherit;
  text-align: inherit;
}
.cv-pager {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 2px 2px 0;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
</style>
