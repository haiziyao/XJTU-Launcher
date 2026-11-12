<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { BellOff, ExternalLink, Search } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { campusDateKey, updatedInfo } from '../../../shared/domain';
import type { NoticeChannels, NoticeList } from '../../../types';
import { campusNoticeCount } from '../campusNav';

/**
 * 通知 (layout v2): the public notice boards in one table, filtered by channel
 * on the left and read in the right column. Only the list page is fetched — the
 * article body needs a per-article request the backend does not make yet — so
 * the reader summarises the entry and offers the campus window for the rest.
 */
interface NoticeRow {
  key: string;
  title: string;
  category: string;
  date: string;
  url: string;
  channel: string;
  channelName: string;
}
interface DayGroup {
  id: string;
  title: string;
  items: NoticeRow[];
}
const LIMIT = 30;
const channels = ref<NoticeChannels['channels']>([]);
const channel = ref('all');
const rows = ref<NoticeRow[]>([]);
const channelCounts = ref<Record<string, number>>({});
const selectedKey = ref<string | null>(null);
const query = ref('');
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
const partial = ref('');
const now = ref(Date.now());

const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return rows.value;
  return rows.value.filter(
    (row) =>
      row.title.toLowerCase().includes(keyword) ||
      row.category.toLowerCase().includes(keyword) ||
      row.channelName.toLowerCase().includes(keyword),
  );
});

function dayId(date: string): { id: string; title: string } {
  const today = campusDateKey(now.value);
  const yesterday = campusDateKey(now.value - 86_400_000);
  if (date === today) return { id: 'today', title: '今天' };
  if (date === yesterday) return { id: 'yesterday', title: '昨天' };
  if (date && date >= today.slice(0, 8) + '01') return { id: 'month', title: '本月' };
  return { id: 'earlier', title: '更早' };
}
const dayGroups = computed<DayGroup[]>(() => {
  const order = ['today', 'yesterday', 'month', 'earlier'];
  const map = new Map<string, DayGroup>();
  for (const row of visible.value) {
    const { id, title } = dayId(row.date);
    const group = map.get(id);
    if (group) group.items.push(row);
    else map.set(id, { id, title, items: [row] });
  }
  return order.map((id) => map.get(id)).filter((group): group is DayGroup => Boolean(group));
});
const selected = computed(() => visible.value.find((row) => row.key === selectedKey.value) ?? null);
const meta = computed(() => {
  const updated = loadedAt.value ? ` · ${updatedInfo(loadedAt.value, now.value)}` : '';
  return `${channels.value.length || '—'} 个栏目 · ${rows.value.length} 条${updated}`;
});

function toRows(list: NoticeList[]): NoticeRow[] {
  const items: NoticeRow[] = [];
  for (const result of list) {
    const name = channels.value.find((entry) => entry.id === result.channel)?.name ?? result.name;
    for (const item of result.items) {
      items.push({
        ...item,
        key: `${result.channel}:${item.url}`,
        channel: result.channel,
        channelName: name,
      });
    }
  }
  return items.sort((left, right) => right.date.localeCompare(left.date));
}

async function fetchChannel(id: string, force = false): Promise<NoticeList> {
  await applyCached<NoticeList>('notices', id, (value) => {
    if (channel.value !== 'all' && id === channel.value) rows.value = toRows([value]);
  });
  return (await bridge.schoolQuery(
    { type: 'notices', channel: id, limit: LIMIT },
    cacheName('notices', id),
    force,
  )) as NoticeList;
}

async function loadChannels(force = false) {
  await applyCached<NoticeChannels>('noticeChannels', undefined, (value) => {
    channels.value = value.channels;
  });
  channels.value = (
    (await bridge.schoolQuery(
      { type: 'noticeChannels' },
      'noticeChannels',
      force,
    )) as NoticeChannels
  ).channels;
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  partial.value = '';
  try {
    if (!channels.value.length || force) await loadChannels(force);
    if (channel.value === 'all') {
      const results = await Promise.allSettled(
        channels.value.map((item) => fetchChannel(item.id, force)),
      );
      const lists = results
        .filter(
          (result): result is PromiseFulfilledResult<NoticeList> => result.status === 'fulfilled',
        )
        .map((result) => result.value);
      rows.value = toRows(lists);
      channelCounts.value = Object.fromEntries(
        lists.map((list) => [list.channel, list.items.length]),
      );
      const failed = results.length - lists.length;
      if (!lists.length) error.value = '通知栏目暂时都无法读取，请稍后重试';
      else if (failed) partial.value = `${failed} 个栏目暂时无法读取，已显示其余栏目。`;
      campusNoticeCount.value = rows.value.length;
    } else {
      const list = await fetchChannel(channel.value, force);
      rows.value = toRows([list]);
      channelCounts.value = { ...channelCounts.value, [channel.value]: list.items.length };
      campusNoticeCount.value = rows.value.length;
    }
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] notices', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeChannel(id: string) {
  if (channel.value === id) return;
  channel.value = id;
  selectedKey.value = null;
  query.value = '';
  void load();
}

function openExternal(row: NoticeRow | null) {
  if (row) void bridge.openUrl(row.url);
}

watch(
  visible,
  (list) => {
    if (!list.some((row) => row.key === selectedKey.value))
      selectedKey.value = list[0]?.key ?? null;
  },
  { immediate: true },
);

onMounted(() => void load());
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
    <span v-if="partial" class="cv-due-soon">· {{ partial }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <label class="cv-search">
      <Search :size="13" aria-hidden="true" />
      <input
        v-model="query"
        type="search"
        placeholder="搜索标题"
        aria-label="搜索通知"
        spellcheck="false"
      />
    </label>
    <button type="button" class="cv-btn" :disabled="loading" @click="load(true)">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols lcr">
    <div class="cv-panel cv-list">
      <div class="cv-lh">频道<span>条数</span></div>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: channel === 'all' }"
        @click="changeChannel('all')"
      >
        <b>全部</b><span class="cv-k accent">{{ rows.length }}</span>
      </button>
      <button
        v-for="item in channels"
        :key="item.id"
        type="button"
        class="cv-item plain"
        :class="{ on: channel === item.id }"
        @click="changeChannel(item.id)"
      >
        <b>{{ item.name }}</b>
        <span class="cv-n">{{ channelCounts[item.id] ?? '—' }}</span>
      </button>
    </div>

    <div class="cv-center">
      <div v-if="loading && !rows.length" class="cv-empty">
        <BellOff :size="22" /><b>正在读取通知…</b>
      </div>
      <div v-else-if="!visible.length" class="cv-empty">
        <BellOff :size="22" />
        <b>{{ rows.length ? '没有符合搜索条件的通知' : '这些栏目暂时没有通知' }}</b>
      </div>
      <div v-else class="cv-panel cv-tbl">
        <div class="cv-thead" style="--cols: 1fr 96px 74px">
          <span>标题</span><span>频道</span><span class="cv-r">日期</span>
        </div>
        <template v-for="group in dayGroups" :key="group.id">
          <div class="cv-grp">
            {{ group.title }}<span class="n">{{ group.items.length }}</span>
          </div>
          <button
            v-for="row in group.items"
            :key="row.key"
            type="button"
            class="cv-tr"
            style="--cols: 1fr 96px 74px"
            :class="{ sel: selectedKey === row.key }"
            @click="selectedKey = row.key"
          >
            <span class="cv-ellip">{{ row.title }}</span>
            <span class="cv-sec cv-ellip">{{ row.channelName }}</span>
            <span class="cv-r cv-num cv-muted">{{ row.date }}</span>
          </button>
        </template>
      </div>
    </div>

    <div class="cv-panel cv-detail">
      <template v-if="selected">
        <div class="cv-type">
          {{ selected.channelName }}
          <template v-if="selected.category"> · {{ selected.category }}</template>
        </div>
        <h3>{{ selected.title }}</h3>
        <dl class="cv-kv">
          <dt>日期</dt>
          <dd class="cv-num">{{ selected.date || '—' }}</dd>
          <dt>栏目</dt>
          <dd>{{ selected.channelName }}</dd>
          <dt>分类</dt>
          <dd>{{ selected.category || '—' }}</dd>
        </dl>
        <div class="cv-blk">
          <h4>正文</h4>
          <p>当前只读取通知的标题、分类和日期；正文与附件在校园网页中查看。</p>
        </div>
        <div class="cv-foot">
          <button type="button" class="cv-btn primary" @click="openExternal(selected)">
            <ExternalLink :size="13" />在校园网页中打开
          </button>
        </div>
      </template>
      <div v-else class="cv-placeholder">选择左侧的一则通知查看来源和日期。</div>
    </div>
  </div>
</template>
