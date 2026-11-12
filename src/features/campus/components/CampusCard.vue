<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { CreditCard, ExternalLink, Search } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import MiniBars from '../../../components/ui/MiniBars.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { directQuery } from '../campusSession';
import {
  campusDateKey,
  campusParts,
  campusTimestamp,
  cardSummary,
  money,
  updatedInfo,
} from '../../../shared/domain';
import type { CardInfo, CardTransaction, CardTransactions } from '../../../types';

/**
 * 校园卡 (layout v2): balance and the chosen window on the left, spend and the
 * transaction table in the centre. The card service rejects WebVPN sessions, so
 * the page queries through the direct session the workspace opened first.
 */
const WINDOWS = [
  { id: 'week', title: '本周' },
  { id: 'month', title: '本月' },
  { id: '30', title: '近 30 天' },
];
type CardSortKey = 'time' | 'merchant' | 'amount' | 'balance';
const columns: { key: CardSortKey; label: string; align?: 'right' }[] = [
  { key: 'time', label: '时间' },
  { key: 'merchant', label: '商户' },
  { key: 'amount', label: '金额', align: 'right' },
  { key: 'balance', label: '余额', align: 'right' },
];
const rangeId = ref('week');
const page = ref(1);
const info = ref<CardInfo | null>(null);
const transactions = ref<CardTransactions | null>(null);
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
const query = ref('');
const flow = ref('all');
const sort = ref<{ key: CardSortKey; dir: 'asc' | 'desc' } | null>({
  key: 'time',
  dir: 'desc',
});

const todayKey = () => campusDateKey(Date.now());
function windowStart(id: string): string {
  const today = todayKey();
  const todayStamp = campusTimestamp(`${today} 00:00`);
  if (id === '30') return campusDateKey(todayStamp - 29 * 86400000);
  if (id === 'month') {
    const parts = campusParts(Date.now());
    if (!parts) return today;
    const month = String(parts.month).padStart(2, '0');
    return `${parts.year}-${month}-01`;
  }
  const parts = campusParts(Date.now());
  const weekday = parts?.weekday ?? 1;
  return campusDateKey(todayStamp - ((weekday + 6) % 7) * 86400000);
}
const windowDays = computed(() => {
  const start = campusTimestamp(`${windowStart(rangeId.value)} 00:00`);
  const end = campusTimestamp(`${todayKey()} 00:00`);
  const days = Math.round((end - start) / 86400000) + 1;
  return Math.min(31, Math.max(1, days));
});
const rows = computed<CardTransaction[]>(() => transactions.value?.rows ?? []);
const summary = computed(() => cardSummary(rows.value, windowDays.value));
const bars = computed(() =>
  summary.value.days.map((day, index) => ({
    label: day.date.slice(5).replace('-', '月') + '日',
    value: day.spend / 100,
    highlight: index === summary.value.days.length - 1,
  })),
);
const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return rows.value.filter((row) => {
    if (flow.value === 'spend' && row.amount >= 0) return false;
    if (flow.value === 'income' && row.amount <= 0) return false;
    if (!keyword) return true;
    return `${row.merchant} ${row.type} ${row.description}`.toLowerCase().includes(keyword);
  });
});
const sortedRows = computed(() => {
  const active = sort.value;
  if (!active) return visible.value;
  const factor = active.dir === 'asc' ? 1 : -1;
  return [...visible.value].sort((left, right) => {
    if (active.key === 'amount') return (left.amount - right.amount) * factor;
    if (active.key === 'balance') return (left.balance - right.balance) * factor;
    const a = active.key === 'time' ? left.time : left.merchant;
    const b = active.key === 'time' ? right.time : right.merchant;
    return a.localeCompare(b, 'zh-Hans-CN', { numeric: true }) * factor;
  });
});
const flowCounts = computed(() => ({
  all: rows.value.length,
  spend: rows.value.filter((row) => row.amount < 0).length,
  income: rows.value.filter((row) => row.amount > 0).length,
}));
const total = computed(() => transactions.value?.total ?? 0);
const morePages = computed(() => page.value * 30 < total.value);
const statNote = computed(() => `支出与收入按本页 ${rows.value.length} 笔流水统计`);
const dailyAverage = computed(() => Math.round(summary.value.spend / windowDays.value));
const rangeTitle = computed(() => WINDOWS.find((item) => item.id === rangeId.value)?.title ?? '');
const subtitle = computed(() =>
  info.value ? [info.value.name, info.value.account].filter(Boolean).join(' · ') : '',
);
const meta = computed(() =>
  [subtitle.value, loadedAt.value ? `更新于 ${updatedInfo(loadedAt.value)}` : '']
    .filter(Boolean)
    .join(' · '),
);
const signed = (cents: number) => `${cents < 0 ? '-' : '+'}${money(Math.abs(cents))}`;

async function loadInfo(force = false) {
  const params = { type: 'card' as const };
  await applyCached<CardInfo>('card', undefined, (value) => (info.value = value));
  info.value = await directQuery<CardInfo>(params, 'card', force);
}

async function loadTransactions(force = false) {
  const params = {
    start: windowStart(rangeId.value),
    end: todayKey(),
    page: page.value,
  };
  await applyCached<CardTransactions>('cardTransactions', params, (value) => {
    transactions.value = value;
  });
  transactions.value = await directQuery<CardTransactions>(
    { type: 'cardTransactions', ...params },
    cacheName('cardTransactions', params),
    force,
  );
}

async function load(reset = false, force = false) {
  if (reset) page.value = 1;
  loading.value = true;
  error.value = '';
  try {
    await Promise.all([loadInfo(force), loadTransactions(force)]);
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] card', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function turn(delta: number) {
  page.value += delta;
  loading.value = true;
  error.value = '';
  try {
    await loadTransactions();
  } catch (e) {
    console.error('[campus] card page', e);
    error.value = e instanceof Error ? e.message : String(e);
    page.value -= delta;
  } finally {
    loading.value = false;
  }
}

function clear() {
  query.value = '';
  flow.value = 'all';
}

function sortBy(key: CardSortKey) {
  const dir = sort.value?.key === key && sort.value.dir === 'asc' ? 'desc' : 'asc';
  sort.value = { key, dir };
}

function onMore(id: string) {
  if (id === 'ncard') void bridge.openUrl('https://ncard.xjtu.edu.cn');
}

watch(rangeId, () => void load(true));
onMounted(() => void load());
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <label class="cv-search">
      <Search :size="13" aria-hidden="true" />
      <input
        v-model="query"
        type="search"
        placeholder="搜索商户或说明"
        aria-label="搜索流水"
        spellcheck="false"
      />
      <button v-if="query" type="button" class="cv-link" aria-label="清除搜索" @click="clear">
        清除
      </button>
    </label>
    <button type="button" class="cv-btn" :disabled="loading" @click="load(false, true)">
      刷新
    </button>
    <button type="button" class="cv-btn" @click="onMore('ncard')">
      <ExternalLink :size="13" aria-hidden="true" />校园卡网站
    </button>
  </CampusHeaderSlot>

  <InlineAlert v-if="info?.lost" tone="warning" title="这张卡已挂失">
    挂失期间不能刷卡消费，补卡后余额会转到新卡。
  </InlineAlert>
  <InlineAlert v-else-if="info?.frozen" tone="warning" title="这张卡已冻结">
    请到校园卡服务点解冻后再使用。
  </InlineAlert>
  <InlineAlert v-if="error" tone="error" :title="error" />

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-figs" style="grid-template-columns: 1fr">
        <div class="cv-fig">
          <span class="k2">余额</span>
          <span class="v">{{ info ? money(info.balance) : '—' }}</span>
          <span class="k2">
            待入账 {{ info ? money(info.pending) : '—' }}
            <template v-if="info?.cardType"> · {{ info.cardType }}</template>
          </span>
        </div>
      </div>
      <div class="cv-hr"></div>
      <div class="cv-lh">时间<span>支出</span></div>
      <button
        v-for="item in WINDOWS"
        :key="item.id"
        type="button"
        class="cv-item plain"
        :class="{ on: rangeId === item.id }"
        @click="rangeId = item.id"
      >
        <b>{{ item.title }}</b>
        <span v-if="rangeId === item.id" class="cv-n">{{ money(summary.spend) }}</span>
      </button>
      <div class="cv-lh">类型<span>本页</span></div>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: flow === 'all' }"
        @click="flow = 'all'"
      >
        <b>全部</b><span class="cv-n">{{ flowCounts.all }}</span>
      </button>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: flow === 'spend' }"
        @click="flow = 'spend'"
      >
        <b>支出</b><span class="cv-n">{{ flowCounts.spend }}</span>
      </button>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: flow === 'income' }"
        @click="flow = 'income'"
      >
        <b>收入</b><span class="cv-n">{{ flowCounts.income }}</span>
      </button>
    </div>

    <div class="cv-center">
      <div class="cv-scroll">
        <div class="cv-panel card-chart">
          <div class="card-chart-head">
            <b>{{ rangeTitle }}支出</b>
            <span>
              {{ money(summary.spend) }} · 日均 {{ money(dailyAverage) }} · {{ statNote }}
            </span>
          </div>
          <MiniBars
            :values="bars"
            :height="64"
            label="每日支出"
            :format="(value: number) => `¥${value.toFixed(2)}`"
          />
        </div>

        <div v-if="loading && !transactions" class="cv-empty">
          <CreditCard :size="22" /><b>正在读取校园卡流水…</b>
        </div>
        <div v-else-if="!visible.length" class="cv-empty">
          <CreditCard :size="22" />
          <b>{{ rows.length ? '没有符合筛选条件的流水' : '这段时间没有流水' }}</b>
        </div>
        <div v-else class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 92px minmax(0, 1fr) 84px 84px">
            <button
              v-for="column in columns"
              :key="column.key"
              type="button"
              class="cv-th"
              :class="{ 'cv-r': column.align === 'right' }"
              @click="sortBy(column.key)"
            >
              {{ column.label
              }}<span v-if="sort?.key === column.key" aria-hidden="true">{{
                sort.dir === 'asc' ? '↑' : '↓'
              }}</span>
            </button>
          </div>
          <div
            v-for="(row, index) in sortedRows"
            :key="`${row.time}-${index}`"
            class="cv-tr"
            style="--cols: 92px minmax(0, 1fr) 84px 84px"
          >
            <span class="cv-num cv-muted">{{ row.time || '—' }}</span>
            <span class="cv-tt">
              <span class="cv-ellip">{{ row.merchant || row.type || '校园卡交易' }}</span>
              <small v-if="row.description">{{ row.description }}</small>
            </span>
            <span class="cv-r cv-num" :class="{ 'card-income': row.amount > 0 }">
              {{ signed(row.amount) }}
            </span>
            <span class="cv-r cv-num cv-muted">{{ money(row.balance) }}</span>
          </div>
        </div>

        <div class="cv-pager">
          <button
            type="button"
            class="cv-btn sm"
            :disabled="page <= 1 || loading"
            @click="turn(-1)"
          >
            上一页
          </button>
          <span>第 {{ transactions?.page ?? page }} 页 · 共 {{ total }} 笔</span>
          <button
            type="button"
            class="cv-btn sm"
            :disabled="loading || !morePages"
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
.card-chart {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px 14px 10px;
}
.card-chart-head {
  display: flex;
  align-items: baseline;
  gap: 8px;
  font-size: 12px;
}
.card-chart-head b {
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
}
.card-chart-head span {
  overflow: hidden;
  color: var(--text-tertiary);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.card-income {
  color: var(--success-text);
}
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
.cv-th.cv-r {
  justify-content: flex-end;
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
