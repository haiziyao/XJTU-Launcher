<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookMarked } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import type { LibraryArea, LibraryAreas, LibraryFloor, LibrarySeats } from '../../types';

import UiButton from '../ui/UiButton.vue';
const floors = ref<LibraryFloor[]>([]);
const campus = ref('');
const floor = ref('');
const areas = ref<LibraryArea[]>([]);
const area = ref('');
const seats = ref<LibrarySeats | null>(null);
const loading = ref(false);
const error = ref('');

const campuses = computed(() => [...new Set(floors.value.map((item) => item.campus))]);
const campusOptions = computed(() => campuses.value.map((name) => ({ value: name, label: name })));
const floorOptions = computed(() =>
  floors.value
    .filter((item) => item.campus === campus.value)
    .map((item) => ({ value: item.code, label: item.name })),
);
const areaName = computed(() => areas.value.find((item) => item.code === area.value)?.name ?? '');
const freeCount = computed(() => (seats.value?.seats ?? []).filter((seat) => seat.free).length);

async function loadFloors() {
  loading.value = true;
  error.value = '';
  try {
    floors.value = (await bridge.schoolQuery({ type: 'libraryFloors' })) as LibraryFloor[];
    campus.value = floors.value[0]?.campus ?? '';
    floor.value = floors.value.find((item) => item.campus === campus.value)?.code ?? '';
    if (floor.value) await loadAreas();
  } catch (e) {
    console.error('[campus] library floors', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadAreas() {
  if (!floor.value) return;
  areas.value = [];
  seats.value = null;
  area.value = '';
  loading.value = true;
  error.value = '';
  try {
    const result = await siteQuery<LibraryAreas>(
      { type: 'libraryAreas', floor: floor.value },
      'library',
      () => bridge.refreshLibrarySession(),
    );
    areas.value = result.areas;
    if (areas.value[0]) {
      area.value = areas.value[0].code;
      await loadSeats();
    }
  } catch (e) {
    console.error('[campus] library areas', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadSeats() {
  if (!area.value) return;
  seats.value = null;
  loading.value = true;
  error.value = '';
  try {
    seats.value = await siteQuery<LibrarySeats>(
      { type: 'librarySeats', area: area.value },
      'library',
      () => bridge.refreshLibrarySession(),
    );
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
  <p v-if="error" class="notice" role="alert">{{ error }}</p>

  <SettingGroup
    title="图书馆座位"
    footnote="座位状态来自图书馆实时系统，预约与签到请到原网站办理。"
  >
    <SettingRow title="校区">
      <MenuSelect
        :model-value="campus"
        :options="campusOptions"
        label="校区"
        placeholder="选择校区"
        @update:model-value="changeCampus"
      />
    </SettingRow>
    <SettingRow title="楼层">
      <MenuSelect
        :model-value="floor"
        :options="floorOptions"
        label="楼层"
        placeholder="选择楼层"
        @update:model-value="changeFloor"
      />
    </SettingRow>
  </SettingGroup>

  <SettingGroup title="区域">
    <EmptyState v-if="loading && !areas.length" compact title="正在读取区域…" />
    <EmptyState
      v-else-if="!areas.length"
      compact
      :icon="BookMarked"
      title="这一层没有可查询的区域"
    />
    <SettingRow
      v-for="item in areas"
      :key="item.code"
      clickable
      :disabled="loading"
      :title="item.name"
      @select="openArea(item.code)"
    >
      <span v-if="item.code === area" class="badge accent">查看中</span>
    </SettingRow>
  </SettingGroup>

  <SettingGroup v-if="area" :title="areaName ? `座位 · ${areaName}` : '座位'">
    <div class="library-summary">
      <span
        >共 {{ seats?.seats.length ?? 0 }} 个座位 · 空余 <strong>{{ freeCount }}</strong> 个</span
      >
      <UiButton size="sm" :disabled="loading" @click="loadSeats">刷新</UiButton>
    </div>
    <div v-if="loading && !seats" class="library-grid">
      <span class="setting-value">正在读取座位…</span>
    </div>
    <div v-else-if="seats" class="library-grid">
      <span
        v-for="seat in seats.seats"
        :key="seat.id"
        class="library-seat"
        :class="{ free: seat.free }"
        :title="seat.free ? `${seat.id} 空余` : `${seat.id} 已占用`"
        >{{ seat.id }}</span
      >
    </div>
  </SettingGroup>
</template>

<style scoped>
.library-summary {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 10px 16px;
  color: var(--text-secondary);
  font-size: 12.5px;
}
.library-summary strong {
  color: var(--success);
  font-variant-numeric: tabular-nums;
}
.library-grid {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding: 12px 16px;
}
.library-seat {
  padding: 4px 8px;
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.library-seat.free {
  border-color: var(--success);
  background: var(--success-soft);
  color: var(--success);
}
</style>
