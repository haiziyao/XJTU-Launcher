<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { DoorOpen } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import { bridge } from '../../bridge';
import type { Option, RoomOptions, Rooms } from '../../types';

import UiButton from '../ui/UiButton.vue';
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

async function loadOptions() {
  loading.value = true;
  error.value = '';
  try {
    options.value = (await bridge.schoolQuery({ type: 'roomOptions' })) as RoomOptions;
    campus.value = options.value.campuses[0]?.id ?? '';
    building.value = options.value.buildings[0]?.id ?? '';
  } catch (e) {
    console.error('[campus] room options', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function search() {
  if (!campus.value || !building.value) return;
  loading.value = true;
  error.value = '';
  try {
    rooms.value = (await bridge.schoolQuery({
      type: 'rooms',
      campus: campus.value,
      building: building.value,
      date: date.value,
      start: Number(start.value),
      end: Number(end.value),
    })) as Rooms;
  } catch (e) {
    console.error('[campus] rooms', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

onMounted(loadOptions);
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>
  <SettingGroup title="空闲教室">
    <SettingRow title="校区">
      <MenuSelect
        :model-value="campus"
        :options="campusOptions"
        label="校区"
        placeholder="选择校区"
        @update:model-value="campus = $event"
      />
    </SettingRow>
    <SettingRow title="楼宇">
      <MenuSelect
        :model-value="building"
        :options="buildingOptions"
        label="楼宇"
        placeholder="选择楼宇"
        @update:model-value="building = $event"
      />
    </SettingRow>
    <SettingRow title="日期">
      <input v-model="date" class="text-field" type="date" aria-label="查询日期" />
    </SettingRow>
    <SettingRow title="节次">
      <div class="campus-field-row">
        <MenuSelect
          :model-value="start"
          :options="sectionOptions"
          label="开始节次"
          @update:model-value="start = $event"
        />
        <span class="campus-record-meta">至</span>
        <MenuSelect
          :model-value="end"
          :options="sectionOptions"
          label="结束节次"
          @update:model-value="end = $event"
        />
      </div>
    </SettingRow>
    <div class="campus-form-actions">
      <UiButton variant="primary" :disabled="loading || !campus || !building" @click="search">
        {{ loading ? '查询中…' : '查询空闲教室' }}
      </UiButton>
    </div>
  </SettingGroup>
  <SettingGroup v-if="rooms" title="查询结果">
    <EmptyState v-if="!rooms.rows.length" compact :icon="DoorOpen" title="所选时间没有空闲教室" />
    <div v-for="(row, index) in rooms.rows" :key="index" class="campus-record three">
      <span class="campus-record-main">
        <strong>{{ row.name }}</strong>
        <small>{{ [row.building, row.campus].filter(Boolean).join(' · ') }}</small>
      </span>
      <span class="campus-record-meta">{{ row.capacity ?? '—' }} 座</span>
      <span class="campus-record-meta">{{ row.category }}</span>
    </div>
  </SettingGroup>
</template>
