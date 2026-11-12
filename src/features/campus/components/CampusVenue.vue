<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { ChevronLeft, ChevronRight, MapPin } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import SlotMatrix, { type SlotCell, type SlotCourt } from './SlotMatrix.vue';
import { bridge } from '../../../shared/bridge';
import { siteQuery } from '../campusSession';
import { applyCached, cacheName } from '../../../shared/cache';
import {
  campusDateKey,
  campusParts,
  campusTimestamp,
  money,
  updatedInfo,
} from '../../../shared/domain';
import type {
  Venue,
  VenueList,
  VenueOrder,
  VenueOrders,
  VenueSlot,
  VenueSlots,
} from '../../../types';

/**
 * 场馆 (layout v2): venues and the quick date list on the left, the time ×
 * court grid in the centre and the picked slot plus my orders on the right.
 * The write protocol for booking is not wired yet, so cells only report state;
 * the page says where to book instead of offering a button that cannot work.
 */
const venues = ref<Venue[]>([]);
const orders = ref<VenueOrder[]>([]);
const selected = ref<Venue | null>(null);
const slots = ref<VenueSlot[]>([]);
const picked = ref<SlotCell | null>(null);
const date = ref(campusDateKey(Date.now()));
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
// The venue closes every endpoint outside 8:40–21:40. That is a service
// schedule, not a failure, so the page states it without an error strip.
const closedHours = computed(() => error.value.startsWith('晚上服务不可用'));

const WEEKDAYS = ['日', '一', '二', '三', '四', '五', '六'];
const venueOptions = computed(() =>
  venues.value.map((venue) => ({ value: String(venue.id), label: venue.name })),
);
const venueGroups = computed(() => {
  const groups = new Map<string, Venue[]>();
  for (const venue of venues.value) {
    const title = venue.category || '其他';
    const list = groups.get(title);
    if (list) list.push(venue);
    else groups.set(title, [venue]);
  }
  return [...groups.entries()].map(([title, items]) => ({ title, items }));
});
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
const dateLabel = (value: string) => {
  const parts = campusParts(`${value} 00:00`);
  if (!parts) return value;
  const month = String(parts.month).padStart(2, '0');
  const day = String(parts.day).padStart(2, '0');
  return `${month}-${day} 周${WEEKDAYS[parts.weekday]}`;
};
const quickDates = computed(() => {
  const today = campusDateKey(Date.now());
  const stamp = campusTimestamp(`${today} 00:00`);
  return Array.from({ length: 5 }, (_, index) => {
    const key = campusDateKey(stamp + index * 86400000);
    return {
      key,
      label:
        index === 0
          ? `今天 ${dateLabel(key)}`
          : index === 1
            ? `明天 ${dateLabel(key)}`
            : dateLabel(key),
    };
  });
});
const headerMeta = computed(() => {
  const venue = selected.value;
  const parts = venue
    ? [venue.category, venue.address, venue.advanceDays ? `可提前 ${venue.advanceDays} 天` : '']
    : [];
  parts.push(dateLabel(date.value));
  return parts.filter(Boolean).join(' · ');
});
const summary = computed(() => {
  if (!selected.value || !cells.value.length) return '';
  const seats = totalSeats.value ? ` · 已订 ${usedSeats.value} / ${totalSeats.value}` : '';
  return `${courts.value.length} 块场地 · ${availableCount.value} 个时段可约${seats}`;
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
const orderBadge = (order: VenueOrder) =>
  order.status === 1 ? 'ok' : order.status === 2 ? 'muted' : 'accent';
const pickedPrice = computed(() =>
  picked.value?.price == null ? '' : money(Math.round(picked.value.price * 100)),
);

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
    picked.value = null;
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

function pickDate(value: string) {
  if (date.value !== value) date.value = value;
}

function refresh() {
  void loadList(true);
}

watch(date, () => void loadSlots());
onMounted(() => void loadList());
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="loadedAt">更新于 {{ updatedInfo(loadedAt) }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <select
      class="cv-select"
      :value="selected ? String(selected.id) : ''"
      aria-label="场馆"
      @change="changeVenue(($event.target as HTMLSelectElement).value)"
    >
      <option value="" disabled>选择场馆</option>
      <option v-for="option in venueOptions" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
    <button type="button" class="cv-btn" :disabled="loading" @click="refresh">刷新</button>
  </CampusHeaderSlot>

  <InlineAlert v-if="error" :tone="closedHours ? 'info' : 'error'" :title="error" />

  <div class="cv-cols lcr">
    <div class="cv-panel cv-list">
      <div v-if="loading && !venues.length" class="cv-placeholder">正在读取场馆…</div>
      <div v-else-if="!venueGroups.length" class="cv-placeholder">
        {{ closedHours ? '预约系统当前未开放。' : '没有可查询的场馆' }}
      </div>
      <template v-else>
        <template v-for="group in venueGroups" :key="group.title">
          <div class="cv-lh">{{ group.title }}</div>
          <button
            v-for="venue in group.items"
            :key="venue.id"
            type="button"
            class="cv-item plain"
            :class="{ on: selected?.id === venue.id }"
            @click="changeVenue(String(venue.id))"
          >
            <b>{{ venue.name }}</b>
            <span class="cv-n">{{ venue.advanceDays ? `${venue.advanceDays} 天` : '' }}</span>
            <span v-if="venue.address" class="s">{{ venue.address }}</span>
          </button>
        </template>
      </template>

      <div class="cv-lh">日期</div>
      <button
        v-for="item in quickDates"
        :key="item.key"
        type="button"
        class="cv-item plain"
        :class="{ on: date === item.key }"
        @click="pickDate(item.key)"
      >
        <b>{{ item.label }}</b>
      </button>
    </div>

    <div class="cv-center">
      <div class="cv-ch">
        <b>{{ selected?.name || '场馆时段' }}</b>
        <span class="meta">{{ summary || headerMeta }}</span>
        <span class="end">
          <button type="button" class="cv-btn sm" aria-label="前一天" @click="shiftDay(-1)">
            <ChevronLeft :size="14" />
          </button>
          <input v-model="date" class="venue-date" type="date" aria-label="查询日期" />
          <button type="button" class="cv-btn sm" aria-label="后一天" @click="shiftDay(1)">
            <ChevronRight :size="14" />
          </button>
        </span>
      </div>

      <div class="cv-scroll">
        <InlineAlert notice-id="venue-booking" tone="info" title="预订尚未接入">
          应用内只能查看时段和我的订单；预订与取消请到场馆网站办理。
        </InlineAlert>

        <div v-if="loading && !slots.length" class="cv-empty">正在读取时段…</div>
        <div v-else-if="!selected" class="cv-placeholder">选择场馆后查看时段。</div>
        <div v-else-if="!cells.length" class="cv-placeholder">这一天没有开放时段。</div>
        <div v-else class="cv-panel venue-matrix">
          <SlotMatrix
            :times="times"
            :courts="courts"
            :cells="cells"
            label="场馆时段表"
            @select="picked = $event"
          />
          <div class="venue-legend">
            <span><i class="lg-free"></i>可约</span>
            <span><i class="lg-full"></i>已满</span>
            <span><i class="lg-closed"></i>不可订</span>
          </div>
        </div>
      </div>
    </div>

    <div class="cv-panel cv-detail">
      <div class="cv-type">时段信息<span v-if="picked" class="cv-badge accent">已选</span></div>
      <template v-if="picked">
        <h3>{{ [picked.court, picked.time].filter(Boolean).join(' · ') }}</h3>
        <dl class="cv-kv">
          <dt>场馆</dt>
          <dd>{{ selected?.name || '—' }}</dd>
          <dt>日期</dt>
          <dd class="cv-num">{{ dateLabel(date) }}</dd>
          <dt>费用</dt>
          <dd v-if="pickedPrice" class="cv-num">{{ pickedPrice }}</dd>
          <dd v-else>—</dd>
          <dt>状态</dt>
          <dd>
            {{
              picked.state === 'available'
                ? '可预约'
                : picked.state === 'full'
                  ? '已满'
                  : '不可预订'
            }}
          </dd>
        </dl>
        <div class="cv-blk">
          <p>应用内还不能提交预约；请到场馆网站办理。开场 15 分钟内未到场记为违约。</p>
        </div>
      </template>
      <div v-else class="cv-placeholder">
        <MapPin :size="16" aria-hidden="true" />点一个时段查看详情
      </div>

      <div class="cv-blk">
        <h4>
          我的预订<span>{{ orders.length }}</span>
        </h4>
        <div v-if="loading && !orders.length" class="cv-placeholder">正在读取预订…</div>
        <div v-else-if="!orders.length" class="cv-placeholder">还没有场馆预订</div>
        <template v-else>
          <div v-for="order in orders" :key="order.id" class="cv-order">
            <div class="l1">
              <b>{{ orderLine(order) || '场馆预订' }}</b>
              <span class="cv-badge" :class="orderBadge(order)">{{ order.statusText || '—' }}</span>
            </div>
            <span class="cv-muted">{{ order.created ? `下单于 ${order.created}` : '' }}</span>
            <span class="cv-sec cv-num">{{ money(order.price * 100) }}</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.venue-matrix {
  padding: 10px 12px;
  overflow: auto;
}
.venue-date {
  width: 138px;
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-size: 12px;
}
.venue-date:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.venue-legend {
  display: flex;
  gap: 14px;
  padding-top: 8px;
  color: var(--text-secondary);
  font-size: 11.5px;
}
.venue-legend span {
  display: inline-flex;
  align-items: center;
  gap: 5px;
}
.venue-legend i {
  display: block;
  width: 10px;
  height: 10px;
  border-radius: 3px;
}
.venue-legend .lg-free {
  background: var(--success-soft);
  border: 1px solid var(--success-text);
}
.venue-legend .lg-full {
  background: var(--surface-sunken);
  border: 1px solid var(--border);
}
.venue-legend .lg-closed {
  background: var(--surface-sunken);
  border: 1px dashed var(--border-strong);
}
</style>
