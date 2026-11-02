<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import SectionHeader from '../ui/SectionHeader.vue';
import SplitView from '../ui/SplitView.vue';
import ListItem from '../ui/ListItem.vue';
import StatRow from '../ui/StatRow.vue';
import StatTile from '../ui/StatTile.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import Skeleton from '../ui/Skeleton.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import SeatMap from './SeatMap.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import { applyCached, cacheName } from '../../cache';
import { percent } from '../../domain';
import type { LibraryArea, LibraryAreas, LibraryFloor, LibrarySeats } from '../../types';

/**
 * 图书馆 (LAYOUT §9.8): areas on the left with their live free/total counts,
 * the seat map on the right. The seat service is plain HTTP; WebVPN reaches it
 * through its port mapping, so both network modes can query it.
 */
const floors = ref<LibraryFloor[]>([]);
const campus = ref('');
const floor = ref('');
const areas = ref<LibraryArea[]>([]);
const area = ref('');
const seats = ref<LibrarySeats | null>(null);
const listWidth = ref(320);
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');

const campuses = computed(() => [...new Set(floors.value.map((item) => item.campus))]);
const campusOptions = computed(() => campuses.value.map((name) => ({ value: name, label: name })));
const floorOptions = computed(() =>
  floors.value
    .filter((item) => item.campus === campus.value)
    .map((item) => ({ value: item.code, label: item.name })),
);
const floorName = computed(
  () => floors.value.find((item) => item.code === floor.value)?.name ?? '',
);
const areaName = computed(() => areas.value.find((item) => item.code === area.value)?.name ?? '');
const areaRows = computed(() => {
  const stats = new Map((seats.value?.stats ?? []).map((entry) => [entry.code, entry]));
  return areas.value
    .map((item) => {
      const stat = stats.get(item.code);
      const total = stat?.total ?? 0;
      const free = stat?.free ?? 0;
      return {
        code: item.code,
        name: item.name,
        total,
        free,
        ratio: total ? free / total : 0,
      };
    })
    .sort(
      (left, right) =>
        right.ratio - left.ratio || left.name.localeCompare(right.name, 'zh-Hans-CN'),
    );
});
const current = computed(() => areaRows.value.find((row) => row.code === area.value) ?? null);
const freeCount = computed(() =>
  seats.value ? seats.value.seats.filter((seat) => seat.free).length : 0,
);
const totalSeats = computed(() => seats.value?.seats.length ?? 0);
const subtitle = computed(() => [campus.value, floorName.value].filter(Boolean).join(' · '));

async function loadFloors(force = false) {
  loading.value = true;
  error.value = '';
  try {
    const stored = await applyCached<LibraryFloor[]>('libraryFloors', undefined, (value) => {
      floors.value = value;
      campus.value ||= value[0]?.campus ?? '';
      floor.value ||= value.find((item) => item.campus === campus.value)?.code ?? '';
    });
    if (stored && floor.value) void loadAreas(force);
    const fresh = (await bridge.schoolQuery(
      { type: 'libraryFloors' },
      'libraryFloors',
      force,
    )) as LibraryFloor[];
    floors.value = fresh;
    campus.value ||= fresh[0]?.campus ?? '';
    const next = fresh.find((item) => item.campus === campus.value)?.code ?? '';
    if (next && next !== floor.value) {
      floor.value = next;
      await loadAreas(force);
    }
  } catch (e) {
    console.error('[campus] library floors', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadAreas(force = false) {
  if (!floor.value) return;
  const params = { floor: floor.value };
  areas.value = [];
  area.value = '';
  seats.value = null;
  loading.value = true;
  error.value = '';
  try {
    const cached = await applyCached<LibraryAreas>('libraryAreas', params, (result) => {
      areas.value = result.areas;
      area.value = result.areas[0]?.code ?? '';
    });
    const result = await siteQuery<LibraryAreas>(
      { type: 'libraryAreas', floor: floor.value },
      'library',
      () => bridge.refreshLibrarySession(),
      cacheName('libraryAreas', params),
      force,
    );
    areas.value = result.areas;
    if (!area.value) area.value = result.areas[0]?.code ?? '';
    if (result.areas.length) await loadSeats(force);
    else if (cached) await loadSeats(force);
  } catch (e) {
    console.error('[campus] library areas', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadSeats(force = false) {
  if (!area.value) return;
  loading.value = true;
  error.value = '';
  try {
    seats.value = await siteQuery<LibrarySeats>(
      { type: 'librarySeats', area: area.value },
      'library',
      () => bridge.refreshLibrarySession(),
      cacheName('librarySeats', { area: area.value }),
      force,
    );
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] library seats', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeCampus(value: string) {
  campus.value = value;
  floor.value = floors.value.find((item) => item.campus === value)?.code ?? '';
  void loadAreas();
}

function changeFloor(value: string) {
  floor.value = value;
  void loadAreas();
}

function openArea(code: string) {
  area.value = code;
  void loadSeats();
}

onMounted(loadFloors);
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="图书馆"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="loadSeats(true)"
    >
      <template #filters>
        <MenuSelect
          :model-value="campus"
          :options="campusOptions"
          label="校区"
          placeholder="选择校区"
          @update:model-value="changeCampus"
        />
        <MenuSelect
          :model-value="floor"
          :options="floorOptions"
          label="楼层"
          placeholder="选择楼层"
          @update:model-value="changeFloor"
        />
      </template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />

    <StatRow label="当前区域">
      <StatTile label="空余座位" :value="area ? freeCount : null" :loading="loading && !seats" />
      <StatTile label="座位总数" :value="area ? totalSeats : null" :loading="loading && !seats" />
      <StatTile
        label="空余率"
        :value="totalSeats ? percent(freeCount / totalSeats) : null"
        :loading="loading && !seats"
      />
      <StatTile label="区域" :value="areas.length" :loading="loading && !areas.length" />
    </StatRow>

    <SplitView
      v-model:width="listWidth"
      :detail-open="Boolean(area)"
      list-label="阅览区域"
      detail-label="座位图"
      @back="area = ''"
    >
      <template #list>
        <Skeleton v-if="loading && !areas.length" :lines="6" height="18px" label="正在读取区域" />
        <p v-else-if="!areas.length" class="library-empty">这一层没有可查询的区域</p>
        <div v-else class="library-areas" role="listbox" aria-label="阅览区域">
          <ListItem
            v-for="row in areaRows"
            :key="row.code"
            :title="row.name"
            :selected="row.code === area"
            @select="openArea(row.code)"
          >
            <template #subtitle>
              <span class="library-free"> 空余 {{ row.free }} / {{ row.total }} </span>
            </template>
            <template #meta>{{ row.total ? percent(row.free / row.total) : '—' }}</template>
          </ListItem>
        </div>
      </template>

      <template #detail>
        <div v-if="!area" class="library-placeholder">
          <p>选择一个区域查看座位分布。</p>
        </div>
        <Skeleton v-else-if="loading && !seats" :lines="6" height="24px" label="正在读取座位" />
        <div v-else-if="seats" class="library-detail">
          <header class="library-detail-head">
            <h3>{{ areaName || '座位' }}</h3>
            <p>
              空余 <strong>{{ current?.free ?? freeCount }}</strong> /
              {{ current?.total ?? totalSeats }}
            </p>
          </header>
          <SeatMap :seats="seats.seats" label="图书馆座位图" />
        </div>
        <div v-else class="library-placeholder">
          <p>暂时无法读取这个区域的座位。</p>
        </div>
      </template>
    </SplitView>
  </section>
</template>

<style scoped>
.library-empty,
.library-placeholder {
  padding: 32px 16px;
  color: var(--text-tertiary);
  font-size: 12.5px;
  text-align: center;
}
.library-areas {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  overflow: hidden;
}
.library-free {
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}
.library-detail {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.library-detail-head {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
}
.library-detail-head h3 {
  font-size: 14px;
  font-weight: 600;
}
.library-detail-head p {
  color: var(--text-secondary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.library-detail-head strong {
  color: var(--success-text);
}
</style>
