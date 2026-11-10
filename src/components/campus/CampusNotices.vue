<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ExternalLink } from 'lucide-vue-next';
import SectionHeader from '../ui/SectionHeader.vue';
import SplitView from '../ui/SplitView.vue';
import ListItem from '../ui/ListItem.vue';
import FilterBar from '../ui/FilterBar.vue';
import SegmentedTabs from '../ui/SegmentedTabs.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import Skeleton from '../ui/Skeleton.vue';
import UiButton from '../ui/UiButton.vue';
import MoreMenu, { type MoreItem } from '../ui/MoreMenu.vue';
import { bridge } from '../../bridge';
import { applyCached, cacheName } from '../../cache';
import type { NoticeChannels, NoticeList } from '../../types';

/**
 * 通知 (LAYOUT §9.10): the seven public notice boards in one list. Only the
 * list page is fetched — the article body needs a per-article request the
 * backend does not make yet — so the reader pane summarises the entry and
 * offers the campus window for the rest.
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
const LIMIT = 30;
const channels = ref<NoticeChannels['channels']>([]);
const channel = ref('all');
const rows = ref<NoticeRow[]>([]);
const selected = ref<NoticeRow | null>(null);
const query = ref('');
const listWidth = ref(320);
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
const partial = ref('');
const moreItems: MoreItem[] = [{ id: 'open', label: '在校园网页中打开', icon: ExternalLink }];

const tabs = computed(() => [
  { id: 'all', title: '全部' },
  ...channels.value.map((item) => ({ id: item.id, title: item.name })),
]);
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
const subtitle = computed(() =>
  channel.value === 'all'
    ? `${channels.value.length} 个栏目 · ${rows.value.length} 条`
    : `${rows.value.length} 条`,
);

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
      const failed = results.length - lists.length;
      if (!lists.length) error.value = '通知栏目暂时都无法读取，请稍后重试';
      else if (failed) partial.value = `${failed} 个栏目暂时无法读取，已显示其余栏目。`;
    } else {
      const list = await fetchChannel(channel.value, force);
      rows.value = toRows([list]);
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
  channel.value = id;
  selected.value = null;
  query.value = '';
  void load();
}

function openExternal(row: NoticeRow | null) {
  if (row) void bridge.openUrl(row.url);
}

function onMore(id: string) {
  if (id === 'open') openExternal(selected.value);
}

onMounted(() => void load());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="通知"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    >
      <template #more
        ><MoreMenu :items="moreItems" :disabled="!selected" @select="onMore"
      /></template>
    </SectionHeader>

    <div class="notice-channels">
      <SegmentedTabs
        :model-value="channel"
        compact
        :tabs="tabs"
        label="通知栏目"
        @update:model-value="changeChannel"
      />
    </div>

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="partial" tone="partial" :title="partial" />

    <FilterBar v-model:query="query" placeholder="搜索标题、分类或栏目" label="搜索通知" />

    <SplitView v-model:width="listWidth" :detail-open="Boolean(selected)" @back="selected = null">
      <template #list>
        <Skeleton v-if="loading && !rows.length" :lines="8" height="18px" label="正在读取通知" />
        <p v-else-if="!visible.length" class="notice-empty">
          {{ rows.length ? '没有符合搜索条件的通知' : '这些栏目暂时没有通知' }}
        </p>
        <div v-else class="notice-list" role="listbox" aria-label="通知列表">
          <ListItem
            v-for="row in visible"
            :key="row.key"
            :title="row.title"
            :subtitle="[row.channelName, row.category].filter(Boolean).join(' · ')"
            :selected="selected?.key === row.key"
            @select="selected = row"
          >
            <template #meta>{{ row.date || '—' }}</template>
          </ListItem>
        </div>
      </template>

      <template #detail>
        <article v-if="selected" class="notice-detail">
          <header>
            <p class="notice-meta">
              {{ selected.channelName }}
              <template v-if="selected.category"> · {{ selected.category }}</template>
              <template v-if="selected.date"> · {{ selected.date }}</template>
            </p>
            <h3>{{ selected.title }}</h3>
          </header>
          <InlineAlert notice-id="notice-body" tone="info" title="正文需要在校园网页中查看">
            当前只读取通知的标题、分类和日期；正文与附件请打开校园网页。
          </InlineAlert>
          <UiButton variant="primary" size="sm" @click="openExternal(selected)">
            <ExternalLink :size="14" />在校园网页中打开
          </UiButton>
        </article>
        <div v-else class="notice-placeholder">
          <p>选择左侧的一则通知查看来源和日期。</p>
        </div>
      </template>
    </SplitView>
  </section>
</template>

<style scoped>
.notice-channels {
  margin-bottom: 12px;
}
.notice-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  overflow: hidden;
}
.notice-empty,
.notice-placeholder {
  padding: 32px 16px;
  color: var(--text-tertiary);
  font-size: 12.5px;
  text-align: center;
}
.notice-detail {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 14px;
  padding: 4px 2px;
}
.notice-meta {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.notice-detail h3 {
  margin-top: 4px;
  font-size: 16px;
  font-weight: 600;
  line-height: 1.45;
}
</style>
