<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { CalendarDays, ExternalLink } from 'lucide-vue-next';
import SectionHeader from '../../../components/ui/SectionHeader.vue';
import SettingGroup from '../../../components/ui/SettingGroup.vue';
import SettingRow from '../../../components/ui/SettingRow.vue';
import EmptyState from '../../../components/ui/EmptyState.vue';
import UiButton from '../../../components/ui/UiButton.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached } from '../../../shared/cache';
import type { CalendarList } from '../../../types';

/**
 * 校历 (LAYOUT §9.7): the school publishes calendar images for each academic
 * year; they open in the campus window because the backend has no image proxy.
 */
const calendar = ref<CalendarList | null>(null);
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');

async function load(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<CalendarList>('calendar', undefined, (value) => (calendar.value = value));
    calendar.value = (await bridge.schoolQuery(
      { type: 'calendar' },
      'calendar',
      force,
    )) as CalendarList;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] calendar', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function open(url: string) {
  void bridge.openUrl(url);
}

onMounted(() => void load());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="校历"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    />

    <InlineAlert v-if="error" tone="error" :title="error" />

    <SettingGroup title="校历图片">
      <EmptyState v-if="loading && !calendar" compact :icon="CalendarDays" title="正在读取校历…" />
      <EmptyState
        v-else-if="!calendar?.rows.length"
        compact
        :icon="CalendarDays"
        title="暂时没有校历图片"
      />
      <SettingRow
        v-for="(row, index) in calendar?.rows ?? []"
        :key="index"
        class="actionable"
        :title="row.year || '未标注学年'"
        description="在校园窗口中打开校历图片"
        @click="open(row.url)"
      >
        <UiButton variant="tinted" size="sm" @click.stop="open(row.url)">
          <ExternalLink :size="14" />打开
        </UiButton>
      </SettingRow>
    </SettingGroup>
  </section>
</template>
