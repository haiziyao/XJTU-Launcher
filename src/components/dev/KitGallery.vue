<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  Bell,
  BookOpen,
  CalendarDays,
  CreditCard,
  FolderOpen,
  GraduationCap,
  Inbox,
  Library,
  Link2,
  Trash2,
} from 'lucide-vue-next';
import PageHeader from '../ui/PageHeader.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import SegmentedTabs from '../ui/SegmentedTabs.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import MoreMenu from '../ui/MoreMenu.vue';
import StatusBadge from '../ui/StatusBadge.vue';
import DueLabel from '../ui/DueLabel.vue';
import StatRow from '../ui/StatRow.vue';
import StatTile from '../ui/StatTile.vue';
import SectionHeader from '../ui/SectionHeader.vue';
import FilterBar from '../ui/FilterBar.vue';
import DataTable from '../ui/DataTable.vue';
import ListItem from '../ui/ListItem.vue';
import SplitView from '../ui/SplitView.vue';
import Skeleton from '../ui/Skeleton.vue';
import MiniBars from '../ui/MiniBars.vue';
import Sparkline from '../ui/Sparkline.vue';
import CampusRail from '../campus/CampusRail.vue';
import FileRow from '../campus/FileRow.vue';
import DropZone from '../campus/DropZone.vue';
import FilePreviewDialog from '../campus/FilePreviewDialog.vue';
import WritePreviewPanel from '../campus/WritePreviewPanel.vue';
import ReaderView from '../campus/ReaderView.vue';
import SlotMatrix from '../campus/SlotMatrix.vue';
import SeatMap from '../campus/SeatMap.vue';
import { CAMPUS_OFFSET_MS } from '../../domain';
import type { RailItem } from '../campus/CampusRail.vue';
import type { SlotCell, SlotCourt } from '../campus/SlotMatrix.vue';
import type { DataColumn, DataSort } from '../ui/DataTable.vue';
import type { FilePreview, WriteOutcome, WritePreview } from '../../types';

/**
 * 开发用组件画廊：只在 `npm run tauri dev` 下出现，不是产品页面。
 * 所有课程、姓名、金额、座位都是示例数据（LAYOUT §11），不要当成真实接口结果。
 */

const now = Date.now();
// 用校园时间取的“今天/明天”，和运行机器的时区无关。
const campusNoon = (offsetDays: number, hour: number, minute = 0) => {
  const date = new Date(now + offsetDays * 86400000 + CAMPUS_OFFSET_MS);
  return (
    Date.UTC(date.getUTCFullYear(), date.getUTCMonth(), date.getUTCDate(), hour, minute) -
    CAMPUS_OFFSET_MS
  );
};

/* 分区导航 */
const rail = ref('todo');
const railItems: RailItem[] = [
  { id: 'todo', title: '待办', icon: Inbox, badge: 4 },
  { id: 'notices', title: '通知', icon: Bell, badge: 12 },
  { id: 'lms', title: '思源学堂', icon: BookOpen, group: '学习' },
  {
    id: 'files',
    title: '资料库',
    icon: FolderOpen,
    group: '学习',
    disabled: true,
    reason: 'P1 待接入',
  },
  { id: 'grades', title: '成绩', icon: GraduationCap, group: '学业' },
  { id: 'exams', title: '考试', icon: CalendarDays, group: '学业' },
  { id: 'card', title: '校园卡', icon: CreditCard, group: '生活' },
  {
    id: 'library',
    title: '图书馆',
    icon: Library,
    group: '生活',
    disabled: true,
    reason: '仅支持校园网直连',
  },
];

/* 列表 + 详情 */
const listWidth = ref(320);
const selected = ref('6');
const assignments = [
  {
    id: '6',
    title: '第 6 次作业',
    subtitle: '数据结构 · 今天 23:59',
    status: 'active' as const,
    label: '未交',
  },
  {
    id: '5',
    title: '第 5 次作业',
    subtitle: '10月25日 · 已交 · 92 分',
    status: 'done' as const,
    label: '已交',
  },
  {
    id: '4',
    title: '第 4 次作业',
    subtitle: '10月18日 · 已交 · 88 分',
    status: 'done' as const,
    label: '已交',
  },
  {
    id: '3',
    title: '第 3 次作业',
    subtitle: '10月11日 · 已交 · 85 分',
    status: 'done' as const,
    label: '已交',
  },
];

/* 筛选与表格 */
const query = ref('');
const tab = ref('all');
const term = ref('2026');
const tabOptions = [
  { id: 'all', title: '全部' },
  { id: 'failed', title: '不及格' },
];
const termOptions = [
  { value: '2026', label: '2026 秋' },
  { value: '2025', label: '2025 秋' },
];
const sort = ref<DataSort | null>({ key: 'score', dir: 'desc' });
const picked = ref<(string | number)[]>([]);
const columns: DataColumn[] = [
  { key: 'course', label: '课程', sortable: true },
  { key: 'type', label: '性质' },
  { key: 'credits', label: '学分', align: 'right', sortable: true },
  { key: 'score', label: '成绩', align: 'right', sortable: true },
  { key: 'gpa', label: '绩点', align: 'right', sortable: true },
];
const gradeRows = [
  { id: 'g1', course: '数据结构', type: '必修', credits: 4, score: 92, gpa: 4 },
  { id: 'g2', course: '大学物理', type: '必修', credits: 3, score: 85, gpa: 3.7 },
  { id: 'g3', course: '线性代数', type: '必修', credits: 2, score: 55, gpa: 1 },
  { id: 'g4', course: '高等数学', type: '必修', credits: 5, score: 'A＋', gpa: 4.3 },
];
const visibleRows = computed(() =>
  gradeRows.filter(
    (row) =>
      (!query.value || row.course.includes(query.value.trim())) &&
      (tab.value !== 'failed' || row.score === 55),
  ),
);

/* 文件 */
const preview = ref<FilePreview | null>(null);
const previewOpen = ref(false);
const imageData =
  'data:image/svg+xml,' +
  encodeURIComponent(
    '<svg xmlns="http://www.w3.org/2000/svg" width="480" height="300"><rect width="480" height="300" fill="#eaf1fd"/><text x="240" y="150" font-family="sans-serif" font-size="20" fill="#1a5ec6" text-anchor="middle">示例图片</text></svg>',
  );
function openPreview(kind: 'text' | 'image') {
  preview.value =
    kind === 'text'
      ? {
          kind: 'text',
          name: '第六章习题.txt',
          mime: 'text/plain',
          size: 2048,
          text: '示例文本预览：\n1. 解释红黑树的插入过程。\n2. 给出最坏情况下的复杂度。',
          truncated: false,
        }
      : { kind: 'image', name: '示例图片.png', mime: 'image/png', size: 32768, dataUrl: imageData };
  previewOpen.value = true;
}

/* 写操作预览 */
const writePreview: WritePreview = {
  id: 'demo',
  kind: 'lmsHomework',
  accountId: 'demo',
  expiresInSeconds: 300,
  title: '数据结构 · 第 6 次作业',
  summary: [
    { label: '方式', value: '正式提交（已交 1 / 可交 3 次）' },
    { label: '文字', value: '作业见附件。' },
  ],
  files: [
    { name: 'hw6.pdf', size: 1258291 },
    { name: 'main.cpp', size: 8192 },
  ],
  warnings: ['这是第 2 次提交，老师可能只看最后一次。'],
  irreversible: true,
};
const writeResult = ref<WriteOutcome | null>(null);
const writeBusy = ref(false);
function showResult(status: WriteOutcome['status']) {
  writeResult.value = {
    status,
    message:
      status === 'done'
        ? '已提交 16:21'
        : status === 'rejected'
          ? '截止时间已过，学校拒绝提交。'
          : '',
  };
}

/* 阅读器 */
const noticeHtml = `
  <p>各学院：现将 2026–2027 学年第一学期期末考试安排通知如下，请同学们按时参加。</p>
  <h2>一、考试时间</h2>
  <p>第 17–18 教学周，具体场次见附件。</p>
  <p><a href="https://dean.xjtu.edu.cn">示例链接：教务处</a></p>
`;

/* 场馆与座位 */
const times = ['18:30', '19:30', '20:30'];
const courts: SlotCourt[] = [
  { id: '1', name: '1 号' },
  { id: '2', name: '2 号' },
  { id: '3', name: '3 号' },
  { id: '4', name: '4 号' },
];
const cells: SlotCell[] = [
  { time: '18:30', court: '1', stockId: 1, price: 20, state: 'available' },
  { time: '18:30', court: '2', state: 'full' },
  { time: '18:30', court: '3', stockId: 3, price: 20, state: 'available' },
  { time: '18:30', court: '4', state: 'closed' },
  { time: '19:30', court: '1', state: 'full' },
  { time: '19:30', court: '2', stockId: 6, price: 20, state: 'available' },
  { time: '19:30', court: '3', stockId: 7, price: 25, state: 'available' },
  { time: '19:30', court: '4', stockId: 8, price: 25, state: 'available' },
  { time: '20:30', court: '1', stockId: 9, price: 15, state: 'available' },
  { time: '20:30', court: '2', state: 'closed' },
  { time: '20:30', court: '3', state: 'full' },
  { time: '20:30', court: '4', stockId: 12, price: 15, state: 'available' },
];
const slotPicked = ref<number[]>([]);
const seats = Array.from({ length: 48 }, (_, index) => ({
  id: `${index + 1}`,
  free: ![2, 5, 6, 9, 14, 15, 21, 22, 27, 30, 36, 40, 44].includes(index),
}));
const seat = ref<string | null>(null);

/* 指标与图表 */
const spending = [
  { label: '10-24', value: 12.5 },
  { label: '10-25', value: 21 },
  { label: '10-26', value: 8 },
  { label: '10-27', value: 32.5 },
  { label: '10-28', value: 18 },
  { label: '10-29', value: 6 },
  { label: '10-30', value: 26, highlight: true },
];
const trend = [
  { label: '2024 春', value: 3.42 },
  { label: '2024 秋', value: 3.51 },
  { label: '2025 春', value: 3.55 },
  { label: '2025 秋', value: 3.6 },
  { label: '2026 春', value: 3.62 },
];
const yuan = (value: number) => `¥${value.toFixed(2)}`;
</script>

<template>
  <div class="gallery">
    <PageHeader title="组件画廊" description="开发预览，仅 npm run tauri dev 可见">
      <template #actions>
        <MoreMenu
          :items="[
            { id: 'export', label: '导出示例数据', icon: Link2 },
            { id: 'clear', label: '清空示例', icon: Trash2, danger: true },
          ]"
          @select="() => undefined"
        />
      </template>
    </PageHeader>

    <InlineAlert tone="warning" title="以下全部是示例数据">
      课程、成绩、金额、座位和通知正文均为演示用，不是真实校园数据。
    </InlineAlert>

    <!-- 分区导航 -->
    <section class="block">
      <SectionHeader title="CampusRail · 分区导航" subtitle="示例" />
      <div class="rail-demo">
        <div class="rail-frame">
          <CampusRail v-model="rail" :items="railItems" />
        </div>
        <div class="rail-frame narrow">
          <CampusRail v-model="rail" :items="railItems" collapsed />
        </div>
      </div>
    </section>

    <!-- 标题、指标、状态 -->
    <section class="block">
      <SectionHeader
        title="SectionHeader · 分区标题"
        subtitle="2026 秋 · 示例"
        :updated-at="now - 5000"
        :refreshing="false"
        @refresh="() => undefined"
      >
        <template #filters>
          <SegmentedTabs v-model="tab" compact :tabs="tabOptions" label="示例筛选" />
        </template>
        <template #more>
          <MoreMenu :items="[{ id: 'web', label: '在校园网页中打开', icon: Link2 }]" />
        </template>
      </SectionHeader>

      <StatRow label="示例指标">
        <StatTile label="余额" value="¥126.50" note="示例金额" clickable />
        <StatTile label="待交" value="3 项" clickable />
        <StatTile label="考试" value="5 天" note="下一场 11月6日" />
        <StatTile label="未读" value="8" />
        <StatTile label="读取中" loading />
        <StatTile label="读取失败" failed @retry="() => undefined" />
      </StatRow>

      <div class="row">
        <StatusBadge status="done" label="已交" />
        <StatusBadge status="near" label="临近截止" />
        <StatusBadge status="urgent" label="已逾期" />
        <StatusBadge status="active" label="未交" />
        <StatusBadge status="neutral" label="已截止" />
        <StatusBadge status="new" />
        <DueLabel :at="campusNoon(0, 23, 59)" :now="now" />
        <DueLabel :at="campusNoon(1, 8)" :now="now" />
        <DueLabel :at="campusNoon(-2, 23, 59)" :now="now" />
        <DueLabel :at="null" :now="now" />
      </div>
    </section>

    <!-- 筛选 + 表格 -->
    <section class="block">
      <SectionHeader title="FilterBar · DataTable" subtitle="示例" />
      <FilterBar v-model:query="query" active placeholder="搜索课程" @clear="query = ''">
        <SegmentedTabs v-model="tab" compact :tabs="tabOptions" label="示例筛选" />
        <MenuSelect v-model="term" :options="termOptions" label="学期" />
      </FilterBar>
      <DataTable
        v-model:selected="picked"
        v-model:sort="sort"
        selectable
        :columns="columns"
        :rows="visibleRows"
        row-key="id"
        label="示例成绩表"
      >
        <template #cell-score="{ value }">
          <span :class="{ fail: value === 55 }">{{ value }}</span>
        </template>
      </DataTable>
      <p class="hint">已选中 {{ picked.length }} 行。排序：{{ sort?.key }} {{ sort?.dir }}</p>
    </section>

    <!-- 列表 + 详情 -->
    <section class="block">
      <SectionHeader title="SplitView · ListItem" subtitle="可拖动分栏" />
      <div class="split-frame">
        <SplitView
          v-model:width="listWidth"
          :detail-open="true"
          list-label="示例作业列表"
          detail-label="示例作业详情"
        >
          <template #list>
            <ListItem
              v-for="item in assignments"
              :key="item.id"
              :title="item.title"
              :subtitle="item.subtitle"
              :selected="selected === item.id"
              @select="selected = item.id"
            >
              <template #status>
                <StatusBadge :status="item.status" :label="item.label" />
              </template>
            </ListItem>
          </template>
          <template #detail>
            <h3 class="detail-title">第 6 次作业（示例）</h3>
            <p class="hint">完成教材第 6 章习题 1–8，提交 PDF。</p>
            <FileRow
              name="第六章习题.pdf"
              :size="327680"
              downloaded
              @preview="openPreview('text')"
            />
          </template>
        </SplitView>
      </div>
    </section>

    <!-- 文件 -->
    <section class="block">
      <SectionHeader title="FileRow · DropZone" subtitle="示例" />
      <FileRow
        name="第六章 图.pptx"
        :size="8598323"
        downloaded
        selectable
        :checked="false"
        @preview="openPreview('text')"
      />
      <FileRow name="图算法补充.pdf" :size="1153433" @preview="openPreview('text')" />
      <FileRow name="示例图片.png" :size="32768" @preview="openPreview('image')" />
      <DropZone class="drop" @files="() => undefined" @reject="() => undefined" />
    </section>

    <!-- 写操作 -->
    <section class="block">
      <SectionHeader title="WritePreviewPanel" subtitle="示例，非真实提交" />
      <div class="write-frame">
        <WritePreviewPanel
          :preview="writePreview"
          :busy="writeBusy"
          :result="writeResult"
          context="截止 今天 23:59 · 还剩 3 小时（示例）"
          confirm-label="确认提交"
          @confirm="showResult('done')"
          @cancel="writeResult = null"
          @view-records="() => undefined"
          @done="writeResult = null"
        />
      </div>
      <div class="row">
        <button class="demo-button" type="button" @click="writeResult = null">预览状态</button>
        <button class="demo-button" type="button" @click="showResult('done')">成功</button>
        <button class="demo-button" type="button" @click="showResult('rejected')">被拒绝</button>
        <button class="demo-button" type="button" @click="showResult('unknown')">结果不明</button>
      </div>
    </section>

    <!-- 阅读器 -->
    <section class="block">
      <SectionHeader title="ReaderView · 通知阅读" subtitle="示例正文" />
      <ReaderView
        :html="noticeHtml"
        title="关于 2026–2027 学年第一学期期末考试安排的通知（示例）"
        source="教务处"
        date="2026-10-29"
        :attachments="[{ name: '考试安排.xlsx', url: '#', size: 88064 }]"
        @open-link="() => undefined"
        @download="() => undefined"
      />
    </section>

    <!-- 场馆与座位 -->
    <section class="block">
      <SectionHeader title="SlotMatrix · SeatMap" subtitle="示例" />
      <SlotMatrix v-model="slotPicked" selectable :times="times" :courts="courts" :cells="cells" />
      <div class="seat-frame">
        <SeatMap :selected="seat" selectable :seats="seats" @select="seat = $event.id" />
      </div>
    </section>

    <!-- 图表与占位 -->
    <section class="block">
      <SectionHeader title="MiniBars · Sparkline · Skeleton" subtitle="示例" />
      <div class="charts">
        <div>
          <h3 class="detail-title">每日支出（示例）</h3>
          <MiniBars :values="spending" :format="yuan" label="示例每日支出" />
        </div>
        <div>
          <h3 class="detail-title">绩点趋势（示例）</h3>
          <Sparkline :points="trend" :format="(value) => value.toFixed(2)" label="示例绩点趋势" />
        </div>
      </div>
      <Skeleton :lines="3" height="14px" label="骨架屏示例" />
    </section>

    <FilePreviewDialog
      :open="previewOpen"
      :preview="preview"
      @close="previewOpen = false"
      @download="() => undefined"
    />
  </div>
</template>

<style scoped>
.gallery {
  max-width: 900px;
  padding-bottom: 40px;
}
.block {
  margin-top: 28px;
  padding-top: 4px;
  border-top: 1px solid var(--border);
}
.block:first-of-type {
  margin-top: 20px;
}
.row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}
.rail-demo {
  display: flex;
  gap: 24px;
  align-items: flex-start;
}
.rail-frame {
  height: 260px;
}
.rail-frame.narrow {
  width: 48px;
}
.split-frame {
  height: 260px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 12px;
  overflow: hidden;
}
.detail-title {
  margin-bottom: 6px;
  font-size: 14px;
  font-weight: 600;
}
.drop {
  margin-top: 12px;
}
.write-frame {
  max-width: 420px;
}
.seat-frame {
  margin-top: 16px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
}
.charts {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 20px;
  margin-bottom: 18px;
}
.hint {
  margin-top: 8px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.demo-button {
  height: 26px;
  padding: 0 9px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  font-size: 12px;
}
.fail {
  color: var(--danger);
  font-weight: 600;
}
</style>
