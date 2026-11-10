<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { CreditCard, ExternalLink } from 'lucide-vue-next';
import SectionHeader from '../../../components/ui/SectionHeader.vue';
import StatRow from '../../../components/ui/StatRow.vue';
import StatTile from '../../../components/ui/StatTile.vue';
import FilterBar from '../../../components/ui/FilterBar.vue';
import SegmentedTabs from '../../../components/ui/SegmentedTabs.vue';
import DataTable, { type DataColumn, type DataSort } from '../../../components/ui/DataTable.vue';
import MiniBars from '../../../components/ui/MiniBars.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import MoreMenu, { type MoreItem } from '../../../components/ui/MoreMenu.vue';
import UiButton from '../../../components/ui/UiButton.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { directQuery } from '../campusSession';
import {
  campusDateKey,
  campusParts,
  campusTimestamp,
  cardSummary,
  money,
} from '../../../shared/domain';
import type { CardInfo, CardTransaction, CardTransactions } from '../../../types';

/**
 * 校园卡 (LAYOUT §9.7): balance, spend for the chosen window, a seven-day bar
 * chart and the transaction table. The card service rejects WebVPN sessions,
 * so the page queries through the direct session the workspace opened first.
 */
const WINDOWS = [
  { id: 'week', title: '本周' },
  { id: 'month', title: '本月' },
  { id: '30', title: '近 30 天' },
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
const sort = ref<DataSort | null>({ key: 'time', dir: 'desc' });

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
const tableRows = computed<Record<string, unknown>[]>(() =>
  visible.value.map((row, index) => ({
    ...row,
    key: `${row.time}-${index}`,
    income: row.amount > 0,
  })),
);
const columns: DataColumn[] = [
  { key: 'time', label: '时间', sortable: true },
  { key: 'merchant', label: '商户', sortable: true },
  { key: 'amount', label: '金额', align: 'right', sortable: true },
  { key: 'balance', label: '余额', align: 'right', sortable: true },
];
const total = computed(() => transactions.value?.total ?? 0);
const morePages = computed(() => page.value * 30 < total.value);
const statNote = computed(() => `支出与收入按本页 ${rows.value.length} 笔流水统计`);
const subtitle = computed(() =>
  info.value ? [info.value.name, info.value.account].filter(Boolean).join(' · ') : '',
);
const signed = (cents: number) => `${cents < 0 ? '-' : '+'}${money(Math.abs(cents))}`;
const moreItems: MoreItem[] = [{ id: 'ncard', label: '在校园卡网站中打开', icon: ExternalLink }];

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

function onMore(id: string) {
  if (id === 'ncard') void bridge.openUrl('https://ncard.xjtu.edu.cn');
}

watch(rangeId, () => void load(true));
onMounted(() => void load());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="校园卡"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(false, true)"
    >
      <template #filters>
        <SegmentedTabs v-model="rangeId" compact :tabs="WINDOWS" label="统计区间" />
      </template>
      <template #more><MoreMenu :items="moreItems" @select="onMore" /></template>
    </SectionHeader>

    <InlineAlert v-if="info?.lost" tone="warning" title="这张卡已挂失">
      挂失期间不能刷卡消费，补卡后余额会转到新卡。
    </InlineAlert>
    <InlineAlert v-else-if="info?.frozen" tone="warning" title="这张卡已冻结">
      请到校园卡服务点解冻后再使用。
    </InlineAlert>
    <InlineAlert v-if="error" tone="error" :title="error" />

    <StatRow label="校园卡概况">
      <StatTile
        label="余额"
        :value="info ? money(info.balance) : null"
        :loading="loading && !info"
        :note="info?.cardType || ''"
      />
      <StatTile
        label="待入账"
        :value="info ? money(info.pending) : null"
        :loading="loading && !info"
      />
      <StatTile
        label="支出"
        :value="money(summary.spend)"
        :loading="loading && !transactions"
        :note="statNote"
      />
      <StatTile
        label="收入"
        :value="`+${money(summary.income)}`"
        :loading="loading && !transactions"
      />
      <StatTile label="流水" :value="total" :loading="loading && !transactions" note="笔" />
    </StatRow>

    <section class="card-chart" aria-label="每日支出">
      <div class="card-chart-head">
        <h3>每日支出</h3>
        <span>单位：元</span>
      </div>
      <MiniBars
        :values="bars"
        :height="64"
        label="每日支出"
        :format="(value: number) => `¥${value.toFixed(2)}`"
      />
    </section>

    <FilterBar
      :query="query"
      :active="flow !== 'all'"
      placeholder="搜索商户或说明"
      label="搜索流水"
      @update:query="query = $event"
      @clear="clear"
    >
      <SegmentedTabs
        v-model="flow"
        compact
        :tabs="[
          { id: 'all', title: '全部' },
          { id: 'spend', title: '支出' },
          { id: 'income', title: '收入' },
        ]"
        label="流水类型"
      />
    </FilterBar>

    <DataTable
      v-model:sort="sort"
      :columns="columns"
      :rows="tableRows"
      row-key="key"
      :loading="loading && !transactions"
      :empty-text="rows.length ? '没有符合筛选条件的流水' : '这段时间没有流水'"
      label="校园卡流水"
    >
      <template #cell-time="{ row }">{{ row.time || '—' }}</template>
      <template #cell-merchant="{ row }">
        <span class="card-merchant">{{ row.merchant || row.type || '校园卡交易' }}</span>
        <small v-if="row.description" class="card-description">{{ row.description }}</small>
      </template>
      <template #cell-amount="{ row }">
        <span :class="{ 'card-income': row.income }">{{ signed(Number(row.amount)) }}</span>
      </template>
      <template #cell-balance="{ row }">{{ money(Number(row.balance)) }}</template>
    </DataTable>

    <div class="campus-pager">
      <UiButton size="sm" :disabled="page <= 1 || loading" @click="turn(-1)">上一页</UiButton>
      <span>第 {{ transactions?.page ?? page }} 页 · 共 {{ total }} 笔</span>
      <UiButton size="sm" :disabled="loading || !morePages" @click="turn(1)">下一页</UiButton>
    </div>
  </section>
</template>

<style scoped>
.card-chart {
  margin-bottom: 16px;
  padding: 12px 14px 8px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.card-chart-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  margin-bottom: 8px;
}
.card-chart-head h3 {
  font-size: 12.5px;
  font-weight: 600;
}
.card-chart-head span {
  color: var(--text-tertiary);
  font-size: 11px;
}
.card-merchant {
  font-weight: 500;
}
.card-description {
  display: block;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.card-income {
  color: var(--success-text);
  font-weight: 600;
}
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
