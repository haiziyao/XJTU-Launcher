<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { ChevronLeft, ChevronRight, MapPin } from 'lucide-vue-next';
import SectionHeader from '../ui/SectionHeader.vue';
import StatRow from '../ui/StatRow.vue';
import StatTile from '../ui/StatTile.vue';
import ListItem from '../ui/ListItem.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import StatusBadge, { type StatusTone } from '../ui/StatusBadge.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import Skeleton from '../ui/Skeleton.vue';
import UiButton from '../ui/UiButton.vue';
import SlotMatrix, { type SlotCell, type SlotCourt } from './SlotMatrix.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import { applyCached, cacheName } from '../../cache';
import { campusDateKey, campusTimestamp, money } from '../../domain';
import type { Venue, VenueList, VenueOrder, VenueOrders, VenueSlot, VenueSlots } from '../../types';

/**
 * 场馆 (LAYOUT §9.9): my orders and the time × court matrix for one venue and
 * day. Booking is not wired to the write protocol yet, so cells are read-only
 * and the page says where to book.
 */
const venues = ref<Venue[]>([]);
const orders = ref<VenueOrder[]>([]);
const selected = ref<Venue | null>(null);
const slots = ref<VenueSlot[]>([]);
const date = ref(campusDateKey(Date.now()));
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');

const venueOptions = computed(() =>
  venues.value.map((venue) => ({ value: String(venue.id), label: venue.name })),
);
const courts = computed<SlotCourt[]>(() => {
  const names = [...new Set(slots.value.map((slot) => slot.area))].sort((a, b) =>
    a.localeCompare(b, 'zh-Hans-CN', { numeric: true }),
  );
  return names.map((name) => ({ id: name, name }));
});
const times = computed(() =>
  [...new Set(slots.value.map((slot) => slot.time))].sort((a, b) =>
    a.localeCompare(b, 'zh-Hans-CN', { numeric: true }),
  ),
);
const cells = computed<SlotCell[]>(() => {
  const map = new Map<string, SlotCell>();
  for (const slot of slots.value) {
    const key = `${slot.time}|${slot.area}`;
    if (map.has(key)) continue;
    const state = slot.available
      ? 'available'
      : slot.total > 0 && slot.using >= slot.total
        ? 'full'
        : 'closed';
    map.set(key, {
      time: slot.time,
      court: slot.area,
      stockId: slot.stockId,
      price: slot.price,
      state,
    });
  }
  return [...map.values()];
});
const availableCount = computed(
  () => cells.value.filter((cell) => cell.state === 'available').length,
);
const usedSeats = computed(() => slots.value.reduce((sum, slot) => sum + slot.using, 0));
const totalSeats = computed(() => slots.value.reduce((sum, slot) => sum + slot.total, 0));
const subtitle = computed(() => (selected.value ? selected.value.name : '场馆与时段'));
const dateLabel = computed(() => {
  const parts = campusTimestamp(`${date.value} 00:00`);
  const parsed = new Date(parts + 8 * 3600000);
  if (!Number.isFinite(parts)) return date.value;
  return `${parsed.getUTCMonth() + 1}月${parsed.getUTCDate()}日`;
});
const orderLine = (order: VenueOrder) =>
  order.details
    .map((item) =>
      [item.date, item.time, [item.venue, item.area].filter(Boolean).join(' ')]
        .filter(Boolean)
        .join(' '),
    )
    .filter(Boolean)
    .join(' / ');
const orderTone = (order: VenueOrder): StatusTone =>
  order.status === 1 ? 'done' : order.status === 2 ? 'neutral' : 'active';

async function loadList(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await Promise.all([
      applyCached<VenueList>('venueList', undefined, (value) => (venues.value = value.venues)),
      applyCached<VenueOrders>('venueOrders', undefined, (value) => (orders.value = value.orders)),
    ]);
    const [list, booked] = await Promise.all([
      siteQuery<VenueList>(
        { type: 'venueList' },
        'venue',
        () => bridge.refreshVenueSession(),
        'venueList',
        force,
      ),
      siteQuery<VenueOrders>(
        { type: 'venueOrders' },
        'venue',
        () => bridge.refreshVenueSession(),
        'venueOrders',
        force,
      ),
    ]);
    venues.value = list.venues;
    orders.value = booked.orders;
    if (!selected.value) selected.value = venues.value[0] ?? null;
    if (selected.value) await loadSlots(force);
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] venues', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadSlots(force = false) {
  if (!selected.value) return;
  const params = { venue: selected.value.id, date: date.value };
  loading.value = true;
  error.value = '';
  try {
    const result = await siteQuery<VenueSlots>(
      { type: 'venueSlots', ...params },
      'venue',
      () => bridge.refreshVenueSession(),
      cacheName('venueSlots', params),
      force,
    );
    slots.value = result.slots;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] venue slots', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeVenue(id: string) {
  const next = venues.value.find((venue) => String(venue.id) === id);
  if (!next) return;
  selected.value = next;
  void loadSlots();
}

function shiftDay(days: number) {
  const stamp = campusTimestamp(`${date.value} 00:00`);
  if (!Number.isFinite(stamp)) return;
  date.value = campusDateKey(stamp + days * 86400000);
}

function refresh() {
  void loadList(true);
}

watch(date, () => void loadSlots());
onMounted(() => void loadList());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="场馆"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="refresh"
    >
      <template #filters>
        <MenuSelect
          :model-value="selected ? String(selected.id) : ''"
          :options="venueOptions"
          label="场馆"
          placeholder="选择场馆"
          @update:model-value="changeVenue"
        />
      </template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />

    <section class="venue-orders" aria-label="我的预订">
      <h3 class="venue-subtitle">我的预订</h3>
      <Skeleton v-if="loading && !orders.length" :lines="3" height="18px" label="正在读取预订" />
      <p v-else-if="!orders.length" class="venue-empty">
        <MapPin :size="18" aria-hidden="true" />还没有场馆预订
      </p>
      <div v-else class="venue-order-list">
        <ListItem
          v-for="order in orders"
          :key="order.id"
          :title="orderLine(order) || '场馆预订'"
          :subtitle="order.created ? `下单于 ${order.created}` : ''"
        >
          <template #status>
            <StatusBadge :status="orderTone(order)" :label="order.statusText" />
          </template>
          <template #meta>{{ money(order.price * 100) }}</template>
        </ListItem>
      </div>
    </section>

    <section class="venue-slots" aria-label="场馆时段">
      <header class="venue-slot-head">
        <h3 class="venue-subtitle">{{ selected?.name || '场馆时段' }} · {{ dateLabel }}</h3>
        <div class="venue-date">
          <UiButton size="sm" variant="ghost" aria-label="前一天" @click="shiftDay(-1)">
            <ChevronLeft :size="14" />
          </UiButton>
          <input
            v-model="date"
            class="text-field venue-date-input"
            type="date"
            aria-label="查询日期"
          />
          <UiButton size="sm" variant="ghost" aria-label="后一天" @click="shiftDay(1)">
            <ChevronRight :size="14" />
          </UiButton>
        </div>
      </header>

      <StatRow label="当日时段">
        <StatTile label="可订时段" :value="availableCount" :loading="loading && !slots.length" />
        <StatTile label="全部时段" :value="cells.length" :loading="loading && !slots.length" />
        <StatTile
          label="已订座位"
          :value="totalSeats ? `${usedSeats} / ${totalSeats}` : null"
          :loading="loading && !slots.length"
        />
        <StatTile label="我的订单" :value="orders.length" :loading="loading && !orders.length" />
      </StatRow>

      <InlineAlert tone="info" title="预订尚未接入">
        应用内只能查看时段和我的订单；预订与取消请到场馆网站办理。
      </InlineAlert>

      <Skeleton v-if="loading && !slots.length" :lines="5" height="26px" label="正在读取时段" />
      <p v-else-if="!selected" class="venue-empty">选择场馆后查看时段。</p>
      <p v-else-if="!cells.length" class="venue-empty">这一天没有开放时段。</p>
      <SlotMatrix v-else :times="times" :courts="courts" :cells="cells" label="场馆时段表" />
    </section>
  </section>
</template>

<style scoped>
.venue-orders,
.venue-slots {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 20px;
}
.venue-subtitle {
  font-size: 12.5px;
  font-weight: 600;
}
.venue-order-list {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  overflow: hidden;
}
.venue-empty {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 24px 16px;
  color: var(--text-tertiary);
  font-size: 12.5px;
}
.venue-slot-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.venue-date {
  display: flex;
  align-items: center;
  gap: 4px;
}
.venue-date-input {
  width: 138px;
  height: 26px;
  padding: 0 8px;
  font-size: 12px;
}
</style>
