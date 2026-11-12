<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import SeatMap from './SeatMap.vue';
import { bridge } from '../../../shared/bridge';
import { siteQuery } from '../campusSession';
import { applyCached, cacheName } from '../../../shared/cache';
import { percent, updatedInfo } from '../../../shared/domain';
import type { LibraryArea, LibraryAreas, LibraryFloor, LibrarySeats } from '../../../types';

/**
 * 图书馆 (layout v2): areas on the left with their live free/total counts and
 * occupancy bars, the seat map in the centre. The seat service is plain HTTP;
 * WebVPN reaches it through its port mapping, so both network modes can query.
 */
const floors = ref<LibraryFloor[]>([]);
const campus = ref('');
const floor = ref('');
const areas = ref<LibraryArea[]>([]);
const area = ref('');
const seats = ref<LibrarySeats | null>(null);
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
const subtitle = computed(() => [campus.value, floorName.value].filter(Boolean).join(' · '));
const meta = computed(() =>
  [subtitle.value, loadedAt.value ? `更新于 ${updatedInfo(loadedAt.value)}` : '']
    .filter(Boolean)
    .join(' · '),
);

const freeWidth = (row: { free: number; total: number }) =>
  `${row.total ? Math.round((row.free / row.total) * 100) : 0}%`;

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
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <select
      class="cv-select"
      :value="campus"
      aria-label="校区"
      @change="changeCampus(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="option in campusOptions" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
    <select
      class="cv-select"
      :value="floor"
      aria-label="楼层"
      @change="changeFloor(($event.target as HTMLSelectElement).value)"
    >
      <option v-for="option in floorOptions" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
    <button type="button" class="cv-btn" :disabled="loading" @click="loadSeats(true)">刷新</button>
  </CampusHeaderSlot>

  <InlineAlert v-if="error" tone="error" :title="error" />

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-lh">{{ floorName || '阅览区域' }}<span>空余</span></div>
      <div v-if="loading && !areas.length" class="cv-placeholder">正在读取区域…</div>
      <div v-else-if="!areas.length" class="cv-placeholder">这一层没有可查询的区域</div>
      <template v-else>
        <button
          v-for="row in areaRows"
          :key="row.code"
          type="button"
          class="cv-item plain"
          :class="{ on: row.code === area }"
          @click="openArea(row.code)"
        >
          <b>{{ row.name }}</b>
          <span class="cv-n">{{ row.total ? `${row.free} / ${row.total}` : '—' }}</span>
          <span class="s cv-occ" aria-hidden="true"
            ><i :style="{ width: freeWidth(row) }"></i
          ></span>
        </button>
      </template>
    </div>

    <div class="cv-center">
      <div class="cv-ch">
        <b>{{ areaName || floorName || '座位图' }}</b>
        <span v-if="current && seats" class="meta">
          空余 {{ current.free }} / {{ current.total }} · {{ percent(current.ratio) }}
        </span>
      </div>
      <div v-if="!area" class="cv-placeholder">选择一个区域查看座位分布。</div>
      <div v-else-if="loading && !seats" class="cv-empty">正在读取座位…</div>
      <div v-else-if="seats" class="cv-panel cv-scroll library-map">
        <SeatMap :seats="seats.seats" label="图书馆座位图" />
      </div>
      <div v-else class="cv-placeholder">暂时无法读取这个区域的座位。</div>
    </div>
  </div>
</template>

<style scoped>
.library-map {
  padding: 14px 16px;
}
.cv-occ {
  height: 3px;
  margin-top: 3px;
  border-radius: 2px;
  background: var(--surface-sunken);
  overflow: hidden;
}
.cv-occ i {
  display: block;
  height: 100%;
  border-radius: 2px;
  background: var(--success);
}
</style>
