<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { CalendarClock, ExternalLink } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import {
  currentAcademicStart,
  dueInfo,
  examEntries,
  updatedInfo,
  type ExamEntry,
} from '../../../shared/domain';
import type { Exams, Terms } from '../../../types';

/**
 * 考试 (layout v2): terms on the left, upcoming exams grouped by campus day in
 * the centre and the selected exam's details on the right. Times come from the
 * school string and are read as campus time. Overlapping exams are flagged from
 * the parsed start and end times.
 */
const terms = ref<Terms | null>(null);
const exams = ref<Exams | null>(null);
const term = ref('');
const selectedKey = ref('');
const loading = ref(false);
const loadedAt = ref<number | null>(null);
const error = ref('');
const now = ref(Date.now());
let timer: ReturnType<typeof setInterval> | undefined;

// Offer only semesters that actually hold exams. The exam API answers one
// term at a time, so each candidate is checked once and added to this set as
// soon as a non-empty answer arrives; an empty term is never listed.
const academicStart = currentAcademicStart();
const candidates = computed(() =>
  (terms.value?.rows ?? []).filter((item) => {
    const year = Number(item.id.slice(0, 4));
    return !Number.isFinite(year) || year <= academicStart;
  }),
);
const availableTermIds = ref<Set<string>>(new Set());
const termOptions = computed(() => [
  { value: '', label: '当前学期' },
  ...candidates.value
    .filter((item) => availableTermIds.value.has(item.id))
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
    let group = result.find((item) => item.label === label);
    if (!group) {
      group = { label, items: [] };
      result.push(group);
    }
    group.items.push(entry);
  }
  return result;
});
const keyOf = (entry: ExamEntry) =>
  `${entry.course}|${entry.dateLabel}|${entry.range}|${entry.location}|${entry.campus}|${entry.seat}`;
const selected = computed(
  () => entries.value.find((entry) => keyOf(entry) === selectedKey.value) ?? null,
);
const conflictKeys = computed(() => {
  const keys = new Set<string>();
  const list = upcoming.value.filter((entry) => entry.start != null && entry.end != null);
  for (let i = 0; i < list.length; i++) {
    for (let j = i + 1; j < list.length; j++) {
      const a = list[i];
      const b = list[j];
      if (a.start == null || a.end == null || b.start == null || b.end == null) continue;
      if (a.start < b.end && b.start < a.end) {
        keys.add(keyOf(a));
        keys.add(keyOf(b));
      }
    }
  }
  return keys;
});
const conflictCount = computed(() => conflictKeys.value.size);
const nextLabel = computed(() => {
  const entry = next.value;
  if (!entry?.start) return '';
  return dueInfo(entry.start, now.value).countdown || '即将开始';
});
const sameDay = computed(() => {
  const entry = selected.value;
  if (!entry) return [];
  return entries.value.filter(
    (item) => item !== entry && keyOf(item) !== keyOf(entry) && item.dateLabel === entry.dateLabel,
  );
});
const selectedBadge = computed<{ text: string; tone: string } | null>(() => {
  const entry = selected.value;
  if (!entry) return null;
  if (entry.ended) return { text: '已结束', tone: 'muted' };
  if (conflictKeys.value.has(keyOf(entry))) return { text: '时间冲突', tone: 'danger' };
  const info = dueInfo(entry.start, now.value);
  if (!info.countdown) return null;
  const tone = info.tone === 'danger' ? 'danger' : info.tone === 'warning' ? 'warn' : 'accent';
  return { text: info.countdown.replace(/^还剩\s*/, ''), tone };
});
const subtitle = computed(() => exams.value?.term ?? '');

function pickTerm(event: Event) {
  changeTerm((event.target as HTMLSelectElement).value);
}

function monthDay(label: string) {
  const found = label.match(/(\d{1,2})月(\d{1,2})日/);
  if (found) return `${found[1].padStart(2, '0')}-${found[2].padStart(2, '0')}`;
  const iso = label.match(/\d{4}-(\d{2})-(\d{2})/);
  return iso ? `${iso[1]}-${iso[2]}` : label;
}

function selectExam(entry: ExamEntry) {
  selectedKey.value = keyOf(entry);
}

async function loadTerms(force = false) {
  try {
    await applyCached<Terms>('terms', undefined, (value) => (terms.value = value));
    terms.value = (await bridge.schoolQuery({ type: 'terms' }, 'terms', force)) as Terms;
  } catch (e) {
    console.error('[campus] exam terms', e);
  }
}

/**
 * Checks every candidate term once, newest first. The backend answers from its
 * own cache while fresh, so the first page open costs one request per term and
 * later opens are mostly local. A term is revealed the moment its list is
 * known to have rows; failures stay hidden rather than showing an empty term.
 */
async function loadAvailability(force = false) {
  const queue = candidates.value.map((item) => item.id);
  const found = new Set<string>();
  const worker = async () => {
    for (let id = queue.shift(); id; id = queue.shift()) {
      try {
        const value = (await bridge.schoolQuery(
          { type: 'exams', term: id },
          cacheName('exams', { term: id }),
          force,
        )) as Exams;
        if (value.rows.length) {
          found.add(id);
          availableTermIds.value = new Set(found);
        }
      } catch (e) {
        console.error('[campus] exam term', id, e);
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(4, queue.length) }, worker));
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

function refresh() {
  void load(true);
  void loadAvailability(true);
}

function onMore(id: string) {
  if (id === 'jwxt') void bridge.openUrl('https://jwxt.xjtu.edu.cn');
}

watch(
  entries,
  (list) => {
    if (list.some((entry) => keyOf(entry) === selectedKey.value)) return;
    const first = list.find((entry) => !entry.ended) ?? list[0];
    selectedKey.value = first ? keyOf(first) : '';
  },
  { immediate: true },
);

onMounted(async () => {
  timer = setInterval(() => (now.value = Date.now()), 60_000);
  await loadTerms();
  void load();
  void loadAvailability();
});
onBeforeUnmount(() => clearInterval(timer));
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="loadedAt">更新于 {{ updatedInfo(loadedAt, now) }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <select class="cv-select" :value="term" aria-label="学期" @change="pickTerm">
      <option v-for="option in termOptions" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
    <button type="button" class="cv-btn" :disabled="loading" @click="refresh">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols lcr">
    <div class="cv-panel cv-list">
      <div class="cv-lh">学期</div>
      <button
        v-for="option in termOptions"
        :key="option.value"
        type="button"
        class="cv-item plain"
        :class="{ on: term === option.value }"
        @click="changeTerm(option.value)"
      >
        <b>{{ option.label }}</b>
      </button>

      <div class="cv-lh">冲突</div>
      <div class="cv-item plain">
        <b>{{ conflictCount ? `${conflictCount} 场时间重叠` : '无冲突' }}</b>
        <span class="cv-n">{{ conflictCount ? '!' : '✓' }}</span>
      </div>
    </div>

    <div class="cv-center">
      <InlineAlert v-if="error" tone="error" :title="error" />
      <div class="cv-ch">
        <b>{{ subtitle || '考试安排' }}</b>
        <span class="meta">
          {{
            [nextLabel, conflictCount ? `${conflictCount} 场冲突` : entries.length ? '无冲突' : '']
              .filter(Boolean)
              .join(' · ')
          }}
        </span>
      </div>

      <div v-if="loading && !exams" class="cv-empty">
        <CalendarClock :size="22" /><b>正在读取考试安排…</b>
      </div>
      <div v-else-if="!entries.length" class="cv-empty">
        <CalendarClock :size="22" /><b>还没有考试安排</b>
      </div>
      <div v-else class="cv-panel cv-tbl">
        <div class="cv-thead" style="--cols: minmax(140px, 1fr) 72px 96px minmax(80px, 1fr) 48px">
          <span>课程</span><span>日期</span><span>时间</span><span>地点</span>
          <span class="cv-r">座位</span>
        </div>
        <template v-for="group in groups" :key="group.label">
          <div class="cv-grp">
            {{ group.label }}<span class="n">{{ group.items.length }} 场</span>
          </div>
          <button
            v-for="(entry, index) in group.items"
            :key="`${group.label}-${index}`"
            type="button"
            class="cv-tr"
            style="--cols: minmax(140px, 1fr) 72px 96px minmax(80px, 1fr) 48px"
            :class="{ sel: selectedKey === keyOf(entry) }"
            @click="selectExam(entry)"
          >
            <span class="cv-ellip">{{ entry.course }}</span>
            <span class="cv-num">{{ monthDay(entry.dateLabel) }}</span>
            <span class="cv-num cv-sec">{{ entry.range || '时间待定' }}</span>
            <span class="cv-sec cv-ellip">{{ entry.location || '考场待定' }}</span>
            <span class="cv-r">
              <span v-if="entry.start == null" class="cv-badge muted">待定</span>
              <span v-else class="cv-num">{{ entry.seat || '—' }}</span>
            </span>
          </button>
        </template>
        <template v-if="past.length">
          <div class="cv-grp">
            已结束<span class="n">{{ past.length }} 场</span>
          </div>
          <button
            v-for="(entry, index) in past"
            :key="`past-${index}`"
            type="button"
            class="cv-tr g-past"
            style="--cols: minmax(140px, 1fr) 72px 96px minmax(80px, 1fr) 48px"
            :class="{ sel: selectedKey === keyOf(entry) }"
            @click="selectExam(entry)"
          >
            <span class="cv-ellip">{{ entry.course }}</span>
            <span class="cv-num">{{ monthDay(entry.dateLabel) }}</span>
            <span class="cv-num cv-sec">{{ entry.range || '时间待定' }}</span>
            <span class="cv-sec cv-ellip">{{ entry.location || '考场待定' }}</span>
            <span class="cv-r cv-num">{{ entry.seat || '—' }}</span>
          </button>
        </template>
      </div>
    </div>

    <div class="cv-panel cv-detail">
      <template v-if="selected">
        <div class="cv-type">
          考试
          <span v-if="selectedBadge" class="cv-badge" :class="selectedBadge.tone">
            {{ selectedBadge.text }}
          </span>
        </div>
        <h3>{{ selected.course }}</h3>
        <div class="cv-fig">
          <span class="k2">
            {{ [selected.dateLabel, selected.range].filter(Boolean).join(' ') || '时间待定' }}
          </span>
          <span class="v g-place">{{ selected.location || '考场待定' }}</span>
        </div>
        <dl class="cv-kv">
          <dt>座位</dt>
          <dd>{{ selected.seat ? `${selected.seat} 号` : '待定' }}</dd>
          <dt>校区</dt>
          <dd>{{ selected.campus || '—' }}</dd>
          <dt>日期</dt>
          <dd>{{ selected.dateLabel || '待定' }}</dd>
          <dt>时间</dt>
          <dd class="cv-num">{{ selected.range || '待定' }}</dd>
        </dl>
        <div v-if="sameDay.length" class="cv-blk">
          <h4>同一天</h4>
          <div v-for="(entry, index) in sameDay" :key="index" class="cv-file">
            <span class="cv-ellip">{{ entry.course }}</span>
            <span class="sz cv-num">
              {{ [entry.range, entry.location].filter(Boolean).join(' ') }}
            </span>
          </div>
        </div>
        <div class="cv-foot">
          <button type="button" class="cv-btn" @click="onMore('jwxt')">
            <ExternalLink :size="13" />在教务系统中打开
          </button>
        </div>
      </template>
      <div v-else class="cv-placeholder">选择一场考试查看时间、地点和座位。</div>
    </div>
  </div>
</template>

<style scoped>
.g-place {
  font-size: 18px;
  word-break: break-word;
}
.g-past {
  color: var(--text-tertiary);
}
</style>
