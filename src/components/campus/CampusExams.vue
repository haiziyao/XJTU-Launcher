<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { ExternalLink, MapPin } from 'lucide-vue-next';
import SectionHeader from '../ui/SectionHeader.vue';
import StatRow from '../ui/StatRow.vue';
import StatTile from '../ui/StatTile.vue';
import StatusBadge from '../ui/StatusBadge.vue';
import DueLabel from '../ui/DueLabel.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import MoreMenu, { type MoreItem } from '../ui/MoreMenu.vue';
import { bridge } from '../../bridge';
import { applyCached, cacheName } from '../../cache';
import { currentAcademicStart, examEntries, type ExamEntry } from '../../domain';
import type { Exams, Terms } from '../../types';

/**
 * 考试 (LAYOUT §9.5): the next exam, upcoming exams grouped by campus day, and
 * a collapsed list of finished ones. Times come from the school string and are
 * read as campus time.
 */
const terms = ref<Terms | null>(null);
const exams = ref<Exams | null>(null);
const term = ref('');
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | undefined;

// Do not offer academic years that have not started yet: they always open on
// an empty exam list.
const academicStart = currentAcademicStart();
const termOptions = computed(() => [
  { value: '', label: '当前学期' },
  ...(terms.value?.rows ?? [])
    .filter((item) => {
      const year = Number(item.id.slice(0, 4));
      return !Number.isFinite(year) || year <= academicStart;
    })
    .map((item) => ({ value: item.id, label: item.name })),
]);
const entries = computed(() => examEntries(exams.value?.rows ?? [], now.value));
const upcoming = computed(() => entries.value.filter((entry) => !entry.ended));
const past = computed(() => entries.value.filter((entry) => entry.ended).reverse());
const next = computed(
  () => upcoming.value.find((entry) => entry.start != null && entry.start > now.value) ?? null,
);
const groups = computed(() => {
  const result: { label: string; items: ExamEntry[] }[] = [];
  for (const entry of upcoming.value) {
    const label = entry.dateLabel || '日期待定';
    let group = result.find((group) => group.label === label);
    if (!group) {
      group = { label, items: [] };
      result.push(group);
    }
    group.items.push(entry);
  }
  return result;
});
const subtitle = computed(() => exams.value?.term ?? '');
const moreItems: MoreItem[] = [{ id: 'jwxt', label: '在教务系统中打开', icon: ExternalLink }];

async function loadTerms(force = false) {
  try {
    await applyCached<Terms>('terms', undefined, (value) => (terms.value = value));
    terms.value = (await bridge.schoolQuery({ type: 'terms' }, 'terms', force)) as Terms;
  } catch (e) {
    console.error('[campus] exam terms', e);
  }
}

async function load(force = false) {
  loading.value = true;
  error.value = '';
  const params = { term: term.value || null };
  try {
    await applyCached<Exams>('exams', params, (value) => (exams.value = value));
    exams.value = (await bridge.schoolQuery(
      { type: 'exams', term: params.term },
      cacheName('exams', params),
      force,
    )) as Exams;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] exams', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function changeTerm(value: string) {
  term.value = value;
  void load();
}

function onMore(id: string) {
  if (id === 'jwxt') void bridge.openUrl('https://jwxt.xjtu.edu.cn');
}

onMounted(async () => {
  timer = setInterval(() => (now.value = Date.now()), 60_000);
  await loadTerms();
  await load();
});
onBeforeUnmount(() => clearInterval(timer));
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="考试"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    >
      <template #filters>
        <MenuSelect
          :model-value="term"
          :options="termOptions"
          label="学期"
          placeholder="当前学期"
          @update:model-value="changeTerm"
        />
      </template>
      <template #more><MoreMenu :items="moreItems" @select="onMore" /></template>
    </SectionHeader>

    <StatRow label="考试概况">
      <StatTile
        label="待考"
        :value="upcoming.length"
        :loading="loading && !exams"
        note="含日期待定"
      />
      <StatTile label="已结束" :value="past.length" :loading="loading && !exams" />
      <StatTile label="全部场次" :value="entries.length" :loading="loading && !exams" />
    </StatRow>

    <InlineAlert v-if="error" tone="error" :title="error" />

    <div v-if="loading && !exams" class="exam-loading">
      <span class="setting-value">正在读取考试安排…</span>
    </div>
    <template v-else-if="entries.length">
      <article v-if="next" class="exam-next">
        <div class="exam-next-head">
          <span class="exam-next-label">下一场</span>
          <DueLabel :at="next.start" :now="now" />
        </div>
        <h3>{{ next.course }}</h3>
        <p>
          <span>{{ next.dateLabel }} · {{ next.range }}</span>
          <span class="exam-place">
            <MapPin :size="13" aria-hidden="true" />
            {{ [next.campus, next.location].filter(Boolean).join(' · ') || '考场待定' }}
            <template v-if="next.seat"> · {{ next.seat }} 号</template>
          </span>
        </p>
      </article>

      <section v-for="group in groups" :key="group.label" class="exam-group">
        <h3 class="exam-group-title">
          <span>{{ group.label }}</span>
          <span class="exam-group-count">{{ group.items.length }} 场</span>
        </h3>
        <div v-for="(entry, index) in group.items" :key="index" class="exam-row">
          <span class="exam-range">{{ entry.range }}</span>
          <span class="exam-course">
            <strong>{{ entry.course }}</strong>
            <small>{{
              [entry.campus, entry.location].filter(Boolean).join(' · ') || '考场待定'
            }}</small>
          </span>
          <span class="exam-seat">{{ entry.seat ? `${entry.seat} 号` : '座号待定' }}</span>
          <StatusBadge v-if="entry.start == null" status="neutral" label="时间待定" />
        </div>
      </section>

      <details v-if="past.length" class="exam-past">
        <summary>已结束 {{ past.length }} 场</summary>
        <div v-for="(entry, index) in past" :key="index" class="exam-row past">
          <span class="exam-range">{{ entry.range }}</span>
          <span class="exam-course">
            <strong>{{ entry.course }}</strong>
            <small>{{ entry.dateLabel }}</small>
          </span>
          <span class="exam-seat">{{
            [entry.location, entry.seat && `${entry.seat} 号`].filter(Boolean).join(' · ') || '—'
          }}</span>
        </div>
      </details>
    </template>
    <p v-else-if="!error" class="exam-empty">还没有考试安排</p>
  </section>
</template>

<style scoped>
.exam-loading,
.exam-empty {
  padding: 32px 0;
  color: var(--text-tertiary);
  font-size: 12.5px;
  text-align: center;
}
.exam-next {
  padding: 14px 16px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.exam-next-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}
.exam-next-label {
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 600;
}
.exam-next h3 {
  margin-top: 8px;
  font-size: 16px;
  font-weight: 600;
}
.exam-next p {
  display: flex;
  flex-wrap: wrap;
  gap: 4px 14px;
  margin-top: 5px;
  color: var(--text-secondary);
  font-size: 12.5px;
  font-variant-numeric: tabular-nums;
}
.exam-place {
  display: inline-flex;
  align-items: center;
  gap: 4px;
}
.exam-group {
  margin-top: 20px;
}
.exam-group-title {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding-bottom: 6px;
  border-bottom: 1px solid var(--divider);
  font-size: 12.5px;
  font-weight: 600;
}
.exam-group-count {
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 500;
}
.exam-row {
  display: grid;
  grid-template-columns: 96px minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 14px;
  min-height: 46px;
  padding: 6px 4px;
  border-bottom: 1px solid var(--divider);
}
.exam-row.past {
  grid-template-columns: 96px minmax(0, 1fr) auto;
  color: var(--text-tertiary);
}
.exam-range {
  color: var(--text-secondary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.exam-course {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}
.exam-course strong {
  overflow: hidden;
  font-size: 13px;
  font-weight: 500;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.exam-course small {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.exam-seat {
  color: var(--text-secondary);
  font-size: 12px;
  white-space: nowrap;
}
.exam-past {
  margin-top: 18px;
}
.exam-past summary {
  padding: 6px 4px;
  border-bottom: 1px solid var(--divider);
  color: var(--text-secondary);
  font-size: 12.5px;
  cursor: pointer;
}
@media (max-width: 720px) {
  .exam-row {
    grid-template-columns: 1fr auto;
  }
  .exam-range {
    grid-column: 1 / -1;
  }
}
</style>
