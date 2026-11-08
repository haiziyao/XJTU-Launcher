<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import SectionHeader from '../ui/SectionHeader.vue';
import PageTabs from '../ui/PageTabs.vue';
import StatRow from '../ui/StatRow.vue';
import StatTile from '../ui/StatTile.vue';
import SegmentedTabs from '../ui/SegmentedTabs.vue';
import DataTable, { type DataColumn, type DataSort } from '../ui/DataTable.vue';
import StatusBadge, { type StatusTone } from '../ui/StatusBadge.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import UiButton from '../ui/UiButton.vue';
import { bridge } from '../../bridge';
import { applyCached, cacheName } from '../../cache';
import { attendanceStats, campusDateKey, campusTimestamp, percent } from '../../domain';
import type {
  AttendanceRecord,
  AttendanceRecords,
  AttendanceStream,
  AttendanceStreams,
} from '../../types';

/**
 * 考勤 (LAYOUT §9.12): course records for the last 30 days with their rates,
 * plus the swipe-stream table. The attendance service does not return a course
 * name, so the per-course view stays unavailable instead of guessing one.
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
const sort = ref<DataSort | null>({ key: 'date', dir: 'desc' });

const range = computed(() => {
  const end = campusTimestamp(`${campusDateKey(Date.now())} 00:00`);
  return {
    start: campusDateKey(end - 29 * 86400000),
    end: campusDateKey(end),
  };
});
const recordRows = computed<AttendanceRecord[]>(() => records.value?.rows ?? []);
const stats = computed(() => attendanceStats(recordRows.value));
const visible = computed(() =>
  recordRows.value.filter((row) => {
    if (filter.value === 'all') return true;
    return row.status === 'late' || row.status === 'absent' || row.status === 'leave';
  }),
);
const tableRows = computed<Record<string, unknown>[]>(() =>
  visible.value.map((row) => ({ ...row })),
);
const columns: DataColumn[] = [
  { key: 'date', label: '日期', sortable: true },
  { key: 'sections', label: '节次' },
  { key: 'place', label: '地点' },
  { key: 'teacher', label: '教师' },
  { key: 'status', label: '状态' },
];
const streamColumns: DataColumn[] = [
  { key: 'time', label: '时间', sortable: true },
  { key: 'place', label: '地点', sortable: true },
  { key: 'valid', label: '结果' },
];
const streamRows = computed<Record<string, unknown>[]>(() =>
  (streams.value?.rows ?? []).map((row) => ({ ...row })),
);
const STATUS: Record<AttendanceRecord['status'], { tone: StatusTone; label: string }> = {
  normal: { tone: 'done', label: '正常' },
  late: { tone: 'near', label: '迟到' },
  absent: { tone: 'urgent', label: '缺勤' },
  leave: { tone: 'neutral', label: '请假' },
  pending: { tone: 'active', label: '未记录' },
  notRequired: { tone: 'neutral', label: '无需考勤' },
  unknown: { tone: 'neutral', label: '未知' },
};
const rateNote = computed(() => `按本页 ${recordRows.value.length} 条记录统计`);
const streamsTotal = computed(() => streams.value?.total ?? 0);
const recordsTotal = computed(() => records.value?.total ?? 0);
const moreRecords = computed(() => states.value.records * 50 < recordsTotal.value);
const moreStreams = computed(() => states.value.streams * 50 < streamsTotal.value);
const subtitle = computed(() =>
  tab.value === 'records' ? `${range.value.start} 至 ${range.value.end}` : '',
);

function statusOf(status: unknown) {
  return STATUS[String(status) as AttendanceRecord['status']] ?? STATUS.unknown;
}
function sections(row: Record<string, unknown>) {
  const start = row.start;
  const end = row.end;
  if (start == null && end == null) return '—';
  return `${start ?? '?'}–${end ?? '?'} 节`;
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
  <section class="campus-section">
    <SectionHeader
      title="考勤"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    />

    <PageTabs v-model="tab" :tabs="TABS" label="考勤视图" />

    <template v-if="tab === 'records'">
      <StatRow label="考勤概况">
        <StatTile
          label="到课率"
          :value="percent(stats.rate)"
          :loading="loading && !records"
          :note="rateNote"
        />
        <StatTile
          label="准时率"
          :value="percent(stats.onTimeRate)"
          :loading="loading && !records"
        />
        <StatTile label="迟到" :value="stats.late" :loading="loading && !records" />
        <StatTile label="缺勤" :value="stats.absent" :loading="loading && !records" />
        <StatTile label="请假" :value="stats.leave" :loading="loading && !records" />
      </StatRow>

      <InlineAlert v-if="error" tone="error" :title="error" />
      <InlineAlert tone="info" title="按课程汇总暂不可用">
        考勤接口目前只返回日期、节次和地点，没有课程名称，这里先按记录展示。
      </InlineAlert>

      <SegmentedTabs
        v-model="filter"
        compact
        :tabs="[
          { id: 'all', title: '全部' },
          { id: 'odd', title: '只看异常' },
        ]"
        label="考勤筛选"
      />

      <DataTable
        v-model:sort="sort"
        :columns="columns"
        :rows="tableRows"
        row-key="id"
        :loading="loading && !records"
        :empty-text="recordRows.length ? '没有异常记录' : '这段时间没有考勤记录'"
        label="考勤明细"
      >
        <template #cell-date="{ row }">{{ row.date || '—' }}</template>
        <template #cell-sections="{ row }">{{ sections(row) }}</template>
        <template #cell-place="{ row }">{{ row.place || '—' }}</template>
        <template #cell-teacher="{ row }">{{ row.teacher || '—' }}</template>
        <template #cell-status="{ row }">
          <StatusBadge :status="statusOf(row.status).tone" :label="statusOf(row.status).label" />
        </template>
      </DataTable>

      <div class="campus-pager">
        <UiButton size="sm" :disabled="states.records <= 1 || loading" @click="turn(-1)">
          上一页
        </UiButton>
        <span>第 {{ records?.page ?? states.records }} 页 · 共 {{ recordsTotal }} 条</span>
        <UiButton size="sm" :disabled="loading || !moreRecords" @click="turn(1)">下一页</UiButton>
      </div>
    </template>

    <template v-else>
      <InlineAlert v-if="error" tone="error" :title="error" />
      <DataTable
        :columns="streamColumns"
        :rows="streamRows"
        row-key="id"
        :loading="loading && !streams"
        :empty-text="'还没有刷卡流水'"
        label="刷卡流水"
      >
        <template #cell-time="{ row }">{{ row.time || '—' }}</template>
        <template #cell-place="{ row }">{{ row.place || '未知地点' }}</template>
        <template #cell-valid="{ row }">
          <StatusBadge
            :status="row.valid ? 'done' : 'neutral'"
            :label="row.valid ? '有效' : '无效'"
          />
        </template>
      </DataTable>

      <div class="campus-pager">
        <UiButton size="sm" :disabled="states.streams <= 1 || loading" @click="turn(-1)">
          上一页
        </UiButton>
        <span>第 {{ streams?.page ?? states.streams }} 页 · 共 {{ streamsTotal }} 条</span>
        <UiButton size="sm" :disabled="loading || !moreStreams" @click="turn(1)">下一页</UiButton>
      </div>
    </template>
  </section>
</template>

<style scoped>
.campus-pager {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 2px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
</style>
