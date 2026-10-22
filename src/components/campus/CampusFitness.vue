<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { Activity } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import { bridge } from '../../bridge';
import type { FitnessScore, FitnessYears } from '../../types';

const years = ref<FitnessYears | null>(null);
const score = ref<FitnessScore | null>(null);
const year = ref('');
const loading = ref(false);
const error = ref('');

const yearOptions = computed(() =>
  (years.value?.rows ?? []).map((item) => ({ value: item.year, label: item.name || item.year })),
);

async function loadYears() {
  loading.value = true;
  error.value = '';
  try {
    years.value = (await bridge.schoolQuery({ type: 'fitness' })) as FitnessYears;
    const preferred = years.value.rows.find((item) => item.checked) ?? years.value.rows[0];
    if (preferred) {
      year.value = preferred.year;
      await loadScore();
    }
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadScore() {
  if (!year.value) return;
  loading.value = true;
  error.value = '';
  try {
    score.value = (await bridge.schoolQuery({
      type: 'fitnessScore',
      year: year.value,
    })) as FitnessScore;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeYear(value: string) {
  year.value = value;
  void loadScore();
}

onMounted(loadYears);
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>
  <SettingGroup title="体测成绩">
    <SettingRow title="学年">
      <MenuSelect
        :model-value="year"
        :options="yearOptions"
        label="体测学年"
        placeholder="选择学年"
        @update:model-value="changeYear"
      />
    </SettingRow>
  </SettingGroup>
  <EmptyState v-if="loading && !score" compact :icon="Activity" title="正在读取体测成绩…" />
  <SettingGroup v-if="score" title="成绩明细">
    <SettingRow
      :title="`总分 ${score.totalScore || '—'}`"
      :description="
        [score.studentName, score.studentNumber, score.sex, score.grade].filter(Boolean).join(' · ')
      "
    >
      <span class="badge accent">{{ score.totalGrade || '—' }}</span>
    </SettingRow>
    <div v-for="item in score.items" :key="item.key" class="campus-record three">
      <span class="campus-record-main">
        <strong>{{ item.name }}</strong>
        <small>{{ item.grade || '—' }}</small>
      </span>
      <span class="campus-record-amount">{{ item.score || '—' }}</span>
    </div>
    <p v-if="score.reportStatus || score.reportType" class="fitness-note">
      {{ [score.reportType, score.reportStatus].filter(Boolean).join(' · ') }}
    </p>
  </SettingGroup>
</template>

<style scoped>
.fitness-note {
  padding: 10px 16px;
  border-top: 1px solid var(--divider);
  color: var(--text-tertiary);
  font-size: 12px;
}
</style>
