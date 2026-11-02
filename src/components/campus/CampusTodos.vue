<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import { ClipboardCheck, ExternalLink } from 'lucide-vue-next';
import SectionHeader from '../ui/SectionHeader.vue';
import DueLabel from '../ui/DueLabel.vue';
import EmptyState from '../ui/EmptyState.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import MoreMenu, { type MoreItem } from '../ui/MoreMenu.vue';
import { bridge } from '../../bridge';
import { loadLmsOverview } from '../../campusOverview';
import { applyCached } from '../../cache';
import { campusTimestamp, todoBuckets } from '../../domain';
import type { LmsOverview, LmsTodo } from '../../types';

/**
 * 待办 (LAYOUT §9.3): homework that is still unsubmitted in the current
 * semester, aggregated across courses. Submissions stay on the campus website;
 * this page only reads and groups what is due.
 */
interface TodoRow extends LmsTodo {
  dueAt: number | null;
}
const rows = ref<TodoRow[]>([]);
const semester = ref('');
const failed = ref(0);
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
const now = ref(Date.now());
const moreItems: MoreItem[] = [{ id: 'lms', label: '在思源学堂中打开', icon: ExternalLink }];

const buckets = computed(() => todoBuckets(rows.value, now.value));
const subtitle = computed(() =>
  rows.value.length ? `${semester.value || '当前学期'} · ${rows.value.length} 项待交` : '',
);

function toRows(list: LmsTodo[]): TodoRow[] {
  return list.map((item) => {
    const at = campusTimestamp(item.end);
    return { ...item, dueAt: Number.isFinite(at) ? at : null };
  });
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<LmsOverview>('lmsOverview', undefined, (value) => {
      if (!rows.value.length) rows.value = toRows(value.rows);
      if (!semester.value) semester.value = value.semester;
      failed.value = value.failed ?? 0;
    });
    const fresh = await loadLmsOverview(force);
    rows.value = toRows(fresh.rows);
    semester.value = fresh.semester;
    failed.value = fresh.failed;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] lms todos', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function onMore(id: string) {
  if (id === 'lms') void bridge.openUrl('https://lms.xjtu.edu.cn');
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
      <template #more><MoreMenu :items="moreItems" @select="onMore" /></template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="failed" tone="partial" title="部分内容未能读取">
      {{ failed }} 个课程或作业请求失败，列表可能不完整；刷新可重试。
    </InlineAlert>
    <p v-if="rows.length" class="todo-source">
      汇总思源学堂本学期尚未提交的作业；提交仍在思源学堂网页完成。
    </p>

    <EmptyState
      v-if="loading && !rows.length"
      :icon="ClipboardCheck"
      compact
      busy
      title="正在汇总本学期待办…"
      description="第一次会扫描本学期全部课程，之后会先使用缓存。"
    />
    <EmptyState
      v-else-if="!rows.length && !error"
      :icon="ClipboardCheck"
      title="当前学期没有待交作业"
      description="这里只显示尚未提交、且老师尚未截止的作业。"
    />

    <section v-for="group in buckets" :key="group.id" class="todo-group">
      <h3 class="todo-group-title">
        {{ group.title }}
        <span>{{ group.items.length }}</span>
      </h3>
      <ul class="todo-list">
        <li v-for="item in group.items" :key="item.id" class="todo-row">
          <span class="todo-copy">
            <span class="todo-title">{{ item.title }}</span>
            <span class="todo-meta">
              <span>{{ item.course || '未标注课程' }}</span>
              <span v-if="item.submitByGroup">小组作业</span>
              <span v-if="item.submitTimes">最多提交 {{ item.submitTimes }} 次</span>
            </span>
          </span>
          <DueLabel :at="item.dueAt" :now="now" />
        </li>
      </ul>
    </section>
  </section>
</template>

<style scoped>
.todo-source {
  margin: 0 0 14px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.todo-group + .todo-group {
  margin-top: 18px;
}
.todo-group-title {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 6px;
  padding: 0 4px;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
}
.todo-group-title span {
  color: var(--text-tertiary);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}
.todo-list {
  margin: 0;
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  list-style: none;
}
.todo-row {
  display: flex;
  align-items: center;
  gap: 12px;
  min-height: 52px;
  padding: 8px 10px;
  border-radius: var(--radius-sm);
}
.todo-row + .todo-row {
  border-top: 1px solid var(--border);
}
.todo-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}
.todo-title {
  overflow: hidden;
  font-size: 13px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.todo-meta {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 10px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
</style>
