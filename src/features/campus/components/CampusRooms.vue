<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { DoorOpen } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import type { Option, RoomOptions, RoomRow, Rooms } from '../../../types';

/**
 * 空闲教室 (layout v2): the date / period / campus / building filters sit in the
 * left column and the free rooms are grouped by building in the centre.
 */
const options = ref<RoomOptions | null>(null);
const rooms = ref<Rooms | null>(null);
const campus = ref('');
const building = ref('');
const date = ref(today());
const start = ref('1');
const end = ref('2');
const loading = ref(false);
const error = ref('');

function today() {
  const now = new Date();
  return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(
    now.getDate(),
  ).padStart(2, '0')}`;
}
const toOptions = (list?: Option[]) =>
  (list ?? []).map((item) => ({ value: item.id, label: item.name }));
const campusOptions = computed(() => toOptions(options.value?.campuses));
const buildingOptions = computed(() => toOptions(options.value?.buildings));
const sectionOptions = Array.from({ length: 14 }, (_, index) => ({
  value: String(index + 1),
  label: `第 ${index + 1} 节`,
}));
const dateText = computed(() => {
  const value = date.value;
  if (!value) return '所选日期';
  const offset = Math.round((Date.parse(value) - Date.parse(today())) / 86_400_000);
  if (offset === 0) return '今天';
  if (offset === 1) return '明天';
  if (offset === 2) return '后天';
  const found = value.match(/\d{4}-(\d{2})-(\d{2})/);
  return found ? `${Number(found[1])}月${Number(found[2])}日` : value;
});
const periodText = computed(() =>
  start.value === end.value ? `第 ${start.value} 节` : `${start.value}–${end.value} 节`,
);
const heading = computed(() => `${dateText.value} ${periodText.value}空闲`);
const campusName = computed(
  () => options.value?.campuses.find((item) => item.id === campus.value)?.name ?? '',
);
const resultMeta = computed(
  () => `${campusName.value || '全部校区'} · ${rooms.value?.rows.length ?? 0} 间 · 按楼宇`,
);
const groups = computed(() => {
  const result: { title: string; items: RoomRow[] }[] = [];
  for (const row of rooms.value?.rows ?? []) {
    const title = row.building || '楼宇待定';
    let group = result.find((item) => item.title === title);
    if (!group) {
      group = { title, items: [] };
      result.push(group);
    }
    group.items.push(row);
  }
  return result;
});

function pickStart(event: Event) {
  changeStart((event.target as HTMLSelectElement).value);
}

function pickEnd(event: Event) {
  changeEnd((event.target as HTMLSelectElement).value);
}

async function loadOptions() {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<RoomOptions>('roomOptions', undefined, (value) => {
      options.value = value;
      campus.value ||= value.campuses[0]?.id ?? '';
      building.value ||= value.buildings[0]?.id ?? '';
    });
    options.value = (await bridge.schoolQuery(
      { type: 'roomOptions' },
      'roomOptions',
    )) as RoomOptions;
    campus.value ||= options.value.campuses[0]?.id ?? '';
    building.value ||= options.value.buildings[0]?.id ?? '';
  } catch (e) {
    console.error('[campus] room options', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function search(force = true) {
  if (!campus.value || !building.value) return;
  const params = {
    campus: campus.value,
    building: building.value,
    date: date.value,
    start: Number(start.value),
    end: Number(end.value),
  };
  await applyCached<Rooms>('rooms', params, (value) => (rooms.value = value));
  loading.value = true;
  error.value = '';
  try {
    rooms.value = (await bridge.schoolQuery(
      { type: 'rooms', ...params },
      cacheName('rooms', params),
      force,
    )) as Rooms;
  } catch (e) {
    console.error('[campus] rooms', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeCampus(value: string) {
  campus.value = value;
  void search(false);
}

function changeBuilding(value: string) {
  building.value = value;
  void search(false);
}

function changeStart(value: string) {
  start.value = value;
  void search(false);
}

function changeEnd(value: string) {
  end.value = value;
  void search(false);
}

onMounted(async () => {
  await loadOptions();
  await search(false);
});
</script>

<template>
  <CampusHeaderSlot slot="tools">
    <button type="button" class="cv-btn" :disabled="loading" @click="search(true)">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols lc">
    <div class="cv-panel cv-list">
      <div class="cv-lh">日期</div>
      <input
        v-model="date"
        class="room-input"
        type="date"
        aria-label="查询日期"
        @change="search(false)"
      />

      <div class="cv-lh">节次</div>
      <div class="room-pair">
        <select class="cv-select" :value="start" aria-label="开始节次" @change="pickStart">
          <option v-for="item in sectionOptions" :key="item.value" :value="item.value">
            {{ item.label }}
          </option>
        </select>
        <span class="cv-muted">至</span>
        <select class="cv-select" :value="end" aria-label="结束节次" @change="pickEnd">
          <option v-for="item in sectionOptions" :key="item.value" :value="item.value">
            {{ item.label }}
          </option>
        </select>
      </div>

      <div class="cv-lh">校区</div>
      <button
        v-for="item in campusOptions"
        :key="item.value"
        type="button"
        class="cv-item plain"
        :class="{ on: campus === item.value }"
        @click="changeCampus(item.value)"
      >
        <b>{{ item.label }}</b>
      </button>

      <div class="cv-lh">楼宇</div>
      <button
        v-for="item in buildingOptions"
        :key="item.value"
        type="button"
        class="cv-item plain"
        :class="{ on: building === item.value }"
        @click="changeBuilding(item.value)"
      >
        <b>{{ item.label }}</b>
      </button>

      <div class="cv-foot room-foot">
        <button
          type="button"
          class="cv-btn primary room-query"
          :disabled="loading || !campus || !building"
          @click="search(true)"
        >
          {{ loading ? '查询中…' : '查询空闲教室' }}
        </button>
      </div>
    </div>

    <div class="cv-center">
      <InlineAlert v-if="error" tone="error" :title="error" />
      <div class="cv-ch">
        <b>{{ heading }}</b>
        <span class="meta">{{ resultMeta }}</span>
      </div>

      <div v-if="loading && !rooms" class="cv-empty">
        <DoorOpen :size="22" /><b>正在读取空闲教室…</b>
      </div>
      <div v-else-if="!rooms" class="cv-placeholder">选择校区和楼宇后查询空闲教室。</div>
      <div v-else-if="!rooms.rows.length" class="cv-empty">
        <DoorOpen :size="22" /><b>所选时间没有空闲教室</b>
      </div>
      <div v-else class="cv-panel cv-tbl">
        <div class="cv-thead" style="--cols: minmax(90px, 1fr) 90px 80px 70px 70px">
          <span>教室</span><span>楼宇</span><span>类型</span> <span class="cv-r">座位</span
          ><span>校区</span>
        </div>
        <template v-for="group in groups" :key="group.title">
          <div class="cv-grp">
            {{ group.title }}<span class="n">{{ group.items.length }}</span>
          </div>
          <div
            v-for="(row, index) in group.items"
            :key="`${group.title}-${index}`"
            class="cv-tr"
            style="--cols: minmax(90px, 1fr) 90px 80px 70px 70px"
          >
            <span class="cv-ellip">{{ row.name }}</span>
            <span class="cv-sec cv-ellip">{{ row.building || '—' }}</span>
            <span class="cv-sec cv-ellip">{{ row.category || '—' }}</span>
            <span class="cv-r cv-num">{{ row.capacity ?? '—' }}</span>
            <span class="cv-muted cv-ellip">{{ row.campus || '—' }}</span>
          </div>
        </template>
      </div>
    </div>
  </div>
</template>

<style scoped>
.room-input {
  width: 100%;
  height: 28px;
  padding: 0 8px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-size: 12px;
}
.room-pair {
  display: flex;
  align-items: center;
  gap: 6px;
  padding: 0 7px;
}
.room-pair .cv-select {
  flex: 1;
  min-width: 0;
}
.room-foot {
  padding: 8px 2px 2px;
}
.room-query {
  width: 100%;
  justify-content: center;
}
</style>
