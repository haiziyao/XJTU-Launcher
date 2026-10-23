<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { MapPin } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import type { Venue, VenueList, VenueOrder, VenueOrders, VenueSlot, VenueSlots } from '../../types';

import UiButton from '../ui/UiButton.vue';
const venues = ref<Venue[]>([]);
const orders = ref<VenueOrder[]>([]);
const selected = ref<Venue | null>(null);
const slots = ref<VenueSlot[]>([]);
const date = ref(today());
const loading = ref(true);
const error = ref('');

function today() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(
    now.getDate(),
  ).padStart(2, '0')}`;
}

const availableCount = computed(() => slots.value.filter((slot) => slot.available).length);
const sortedSlots = computed(() =>
  [...slots.value].sort(
    (a, b) => a.time.localeCompare(b.time) || a.area.localeCompare(b.area, 'zh-CN'),
  ),
);

async function load() {
  loading.value = true;
  error.value = '';
  try {
    const [list, booked] = await Promise.all([
      siteQuery<VenueList>({ type: 'venueList' }, 'venue', () => bridge.refreshVenueSession()),
      siteQuery<VenueOrders>({ type: 'venueOrders' }, 'venue', () => bridge.refreshVenueSession()),
    ]);
    venues.value = list.venues;
    orders.value = booked.orders;
  } catch (e) {
    console.error('[campus] venues', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadSlots() {
  if (!selected.value) return;
  loading.value = true;
  error.value = '';
  try {
    const result = await siteQuery<VenueSlots>(
      { type: 'venueSlots', venue: selected.value.id, date: date.value },
      'venue',
      () => bridge.refreshVenueSession(),
    );
    slots.value = result.slots;
  } catch (e) {
    console.error('[campus] venue slots', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function openVenue(venue: Venue) {
  selected.value = venue;
  slots.value = [];
  date.value = today();
  void loadSlots();
}

function back() {
  selected.value = null;
  slots.value = [];
  error.value = '';
}

const detailLine = (order: VenueOrder) =>
  order.details
    .map((item) => [item.venue, item.area].filter(Boolean).join(' '))
    .filter(Boolean)
    .join(' / ');

onMounted(load);
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>

  <SettingGroup title="我的预订">
    <EmptyState v-if="loading && !orders.length" compact title="正在读取预订…" />
    <EmptyState v-else-if="!orders.length" compact :icon="MapPin" title="还没有场馆预订" />
    <div v-for="order in orders" :key="order.id" class="campus-record three">
      <span class="campus-record-main">
        <strong>{{ detailLine(order) || '场馆预订' }}</strong>
        <small>{{
          order.details
            .map((item) => [item.date, item.time].filter(Boolean).join(' '))
            .join(' · ') || order.created
        }}</small>
      </span>
      <span class="badge" :class="{ accent: order.status === 1 }">{{ order.statusText }}</span>
      <span class="campus-record-meta">¥{{ order.price.toFixed(2) }}</span>
    </div>
  </SettingGroup>

  <SettingGroup v-if="!selected" title="场馆">
    <EmptyState v-if="loading && !venues.length" compact title="正在读取场馆…" />
    <EmptyState v-else-if="!venues.length" compact :icon="MapPin" title="没有可查询的场馆" />
    <SettingRow
      v-for="venue in venues"
      :key="venue.id"
      clickable
      :disabled="loading"
      :title="venue.name"
      :description="[venue.address, venue.category].filter(Boolean).join(' · ')"
      @select="openVenue(venue)"
    >
      <span class="campus-record-meta">{{ venue.advanceDays }} 天内可约</span>
    </SettingRow>
  </SettingGroup>

  <SettingGroup v-if="selected" :title="`场馆时段 · ${selected.name}`">
    <template #actions>
      <UiButton variant="ghost" size="sm" @click="back">返回场馆</UiButton>
    </template>
    <SettingRow title="日期">
      <input
        v-model="date"
        class="text-field"
        type="date"
        aria-label="查询日期"
        @change="loadSlots"
      />
    </SettingRow>
    <div class="venue-summary">
      <span
        >共 {{ slots.length }} 个时段 · 可订 <strong>{{ availableCount }}</strong> 个</span
      >
      <UiButton size="sm" :disabled="loading" @click="loadSlots">刷新</UiButton>
    </div>
    <EmptyState v-if="loading && !slots.length" compact title="正在读取时段…" />
    <EmptyState v-else-if="!slots.length" compact title="这一天没有开放时段" />
    <div v-for="slot in sortedSlots" :key="slot.stockId" class="campus-record three">
      <span class="campus-record-main">
        <strong>{{ slot.area }}</strong>
        <small>{{ slot.time || '时段未知' }}</small>
      </span>
      <span class="campus-record-meta"
        >¥{{ slot.price.toFixed(2) }} · {{ slot.using }}/{{ slot.total }}</span
      >
      <span class="badge" :class="{ accent: slot.available }">{{
        slot.available ? '可订' : '不可订'
      }}</span>
    </div>
  </SettingGroup>
</template>

<style scoped>
.venue-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 16px;
  color: var(--text-secondary);
  font-size: 12.5px;
}
.venue-summary strong {
  color: var(--success);
  font-variant-numeric: tabular-nums;
}
</style>
