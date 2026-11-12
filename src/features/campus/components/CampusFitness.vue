<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { Activity } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import { siteQuery } from '../campusSession';
import { bridge } from '../../../shared/bridge';
import { applyCached, cacheName } from '../../../shared/cache';
import { currentAcademicStart } from '../../../shared/domain';
import { bmiOf, measureValue } from '../profileOverview';
import type {
  FitnessItem,
  FitnessItemDetail,
  FitnessScore,
  FitnessYear,
  FitnessYears,
  SchoolQuery,
} from '../../../types';

/**
 * 体测 (layout v2): the years and their total scores on the left, the item grid
 * and the total-score history in the centre, one item's raw measurement in the
 * detail column. Raw values come from `Report/getItemDetails`; the history list
 * in that reply is not wired yet, so the page shows the current measurement
 * only and never invents a trend.
 */
const ITEM_NUM: Record<string, number> = {
  bmi: 1,
  vc: 2,
  jump: 3,
  sit_and_reach: 4,
  pull_and_sit: 5,
  '50m': 6,
  run: 7,
};
const ITEM_UNIT: Record<string, string> = {
  bmi: '',
  vc: 'ml',
  jump: 'cm',
  sit_and_reach: 'cm',
  pull_and_sit: '次',
  '50m': '秒',
  run: '',
};
const ITEM_MEASURE: Record<string, string> = {
  bmi: '身高 / 体重',
  vc: '肺活量',
  jump: '立定跳远',
  sit_and_reach: '坐位体前屈',
  pull_and_sit: '引体向上 / 仰卧起坐',
  '50m': '50 米',
  run: '800 / 1000 米',
};

const years = ref<FitnessYear[]>([]);
const scores = ref<Record<string, FitnessScore | null>>({});
const year = ref('');
const detailKey = ref('');
const detail = ref<FitnessItemDetail | null>(null);
const detailLoading = ref(false);
const detailError = ref('');
const loading = ref(false);
const error = ref('');

// Future academic years cannot have a score yet; keeping them in the picker
// only produces empty pages.
const academicStart = currentAcademicStart();
const availableYears = computed(() =>
  years.value
    .filter((item) => /^\d{4}$/.test(item.year) && Number(item.year) <= academicStart)
    .sort((left, right) => right.year.localeCompare(left.year)),
);
const activeYear = computed(() => availableYears.value.find((item) => item.year === year.value));
const score = computed(() => (year.value ? (scores.value[year.value] ?? null) : null));
const items = computed(() => score.value?.items ?? []);
const chosen = computed(() => items.value.find((item) => item.key === detailKey.value) ?? null);
const meta = computed(() => {
  const value = score.value;
  return [value?.studentName, value?.studentNumber, value?.sex, value?.grade]
    .filter(Boolean)
    .join(' · ');
});

const gradeTone = (grade: string): string => {
  if (/优/.test(grade)) return 'ok';
  if (/良/.test(grade)) return 'accent';
  if (/不及格|不合格/.test(grade)) return 'danger';
  return 'muted';
};
function numeric(text: string): number | null {
  const trimmed = (text ?? '').trim();
  if (!/^\d+(\.\d+)?$/.test(trimmed)) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) ? value : null;
}
/** The score to show on a card; `converted` is the fallback when `score` is blank. */
function itemScore(item: FitnessItem): string {
  return item.score || item.converted || '';
}
function itemValue(item: FitnessItem): number | null {
  return numeric(itemScore(item));
}

/* ---- total history ---- */
const history = computed(() => {
  const points = availableYears.value
    .map((item) => ({
      year: item.year,
      label: item.name || item.year,
      value: numeric(scores.value[item.year]?.totalScore ?? ''),
    }))
    .filter((point): point is { year: string; label: string; value: number } => point.value != null)
    .sort((left, right) => left.year.localeCompare(right.year));
  if (!points.length) return null;
  const values = points.map((point) => point.value);
  const lo = Math.max(0, Math.min(60, Math.floor(Math.min(...values) - 5)));
  const hi = 100;
  // Leave room for a centred year label at both ends; otherwise a label such
  // as “2025-2026学年” runs past the right edge of the viewBox.
  const xAt = (index: number) =>
    points.length === 1 ? 230 : 60 + (index * (400 - 60)) / (points.length - 1);
  const yAt = (value: number) => 110 - ((value - lo) / (hi - lo)) * 100;
  return {
    points: points.map((point, index) => ({
      ...point,
      x: xAt(index),
      y: yAt(point.value),
      // Keep the direct value label inside the viewBox when the point sits on
      // the top grid line (for example a full score).
      labelY: Math.max(12, yAt(point.value) - 10),
    })),
    show80: 80 >= lo,
    y80: yAt(80),
    grid: [90, 80, 70, 60]
      .filter((value) => value >= lo)
      .map((value) => ({ value, y: yAt(value) })),
    base: yAt(lo),
  };
});

/* ---- raw measurement ---- */
const rawValues = computed(() => {
  const info = detail.value?.info;
  if (!info) return [] as { label: string; text: string }[];
  const key = detailKey.value;
  const first = info.value_1;
  const second = info.value_2;
  if (key === 'bmi') {
    return [
      { label: '身高', text: first == null ? '—' : `${first} cm` },
      { label: '体重', text: second == null ? '—' : `${second} kg` },
    ].filter((row) => row.text !== '—');
  }
  const unit = ITEM_UNIT[key] ?? '';
  const text = first == null || String(first).trim() === '' ? '—' : `${first} ${unit}`.trim();
  return text === '—' ? [] : [{ label: ITEM_MEASURE[key] ?? '原始值', text }];
});
const bmi = computed(() =>
  detailKey.value === 'bmi'
    ? bmiOf(measureValue(detail.value?.info?.value_1), measureValue(detail.value?.info?.value_2))
    : null,
);

/* ---- loading ---- */
// The fitness platform keeps its own H5 login; a shared direct session is not
// enough. `siteQuery` signs in with the saved password once when the stored
// fitness session is missing or expired.
const fitnessQuery = <T,>(query: SchoolQuery, cache?: string, force = false) =>
  siteQuery<T>(query, 'fitness', () => bridge.refreshFitnessSession(), cache, force);

async function loadYearScore(target: string, force = false) {
  const params = { year: target };
  try {
    await applyCached<FitnessScore>('fitnessScore', params, (value) => {
      if (!scores.value[target]) scores.value = { ...scores.value, [target]: value };
    });
    const value = await fitnessQuery<FitnessScore>(
      { type: 'fitnessScore', ...params },
      cacheName('fitnessScore', params),
      force,
    );
    scores.value = { ...scores.value, [target]: value };
  } catch (e) {
    console.error('[campus] fitness score', target, e);
    scores.value = { ...scores.value, [target]: null };
  }
}

async function loadYears(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<FitnessYears>('fitness', undefined, (value) => (years.value = value.rows));
    const value = await fitnessQuery<FitnessYears>({ type: 'fitness' }, 'fitness', force);
    years.value = value.rows ?? [];
    const preferred = availableYears.value.find((item) => item.checked) ?? availableYears.value[0];
    if (!preferred) {
      year.value = '';
      return;
    }
    year.value = preferred.year;
    await loadYearScore(preferred.year, force);
    // The list shows every year's total; the other years load in the
    // background so opening the page does not wait for four report calls.
    for (const item of availableYears.value) {
      if (item.year === preferred.year) continue;
      void loadYearScore(item.year, force);
    }
    chooseItem();
  } catch (e) {
    console.error('[campus] fitness years', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadDetail() {
  const target = year.value;
  const key = detailKey.value;
  const item = ITEM_NUM[key];
  if (!target || !item) {
    detail.value = null;
    return;
  }
  detailLoading.value = true;
  detailError.value = '';
  try {
    const params = { item, year: target };
    await applyCached<FitnessItemDetail>('fitnessItem', params, (value) => {
      if (detailKey.value === key) detail.value = value;
    });
    const value = await fitnessQuery<FitnessItemDetail>(
      { type: 'fitnessItem', ...params },
      cacheName('fitnessItem', params),
    );
    if (detailKey.value === key && year.value === target) detail.value = value;
  } catch (e) {
    console.error('[campus] fitness item', key, e);
    if (detailKey.value === key) {
      detail.value = null;
      detailError.value = e instanceof Error ? e.message : String(e);
    }
  } finally {
    detailLoading.value = false;
  }
}

function chooseItem() {
  // While the new year's score is still loading, keep the current selection
  // instead of blanking the detail column for a moment.
  if (!score.value) return;
  if (!items.value.length) {
    detailKey.value = '';
    return;
  }
  if (items.value.some((item) => item.key === detailKey.value)) return;
  detailKey.value = items.value.find((item) => item.key === 'vc')?.key ?? items.value[0].key;
}

function changeYear(value: string) {
  if (year.value === value) return;
  year.value = value;
  if (!scores.value[value]) {
    void loadYearScore(value).then(() => {
      if (year.value === value) chooseItem();
    });
  }
  chooseItem();
}

function pickItem(key: string) {
  if (detailKey.value === key) return;
  detailKey.value = key;
}

watch([year, detailKey], () => {
  detail.value = null;
  void loadDetail();
});

onMounted(loadYears);
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <button type="button" class="cv-btn" :disabled="loading" @click="loadYears(true)">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols fz">
    <div class="cv-panel cv-list">
      <div class="cv-lh">学年<span>总分</span></div>
      <button
        v-for="item in availableYears"
        :key="item.year"
        type="button"
        class="cv-item plain"
        :class="{ on: year === item.year }"
        @click="changeYear(item.year)"
      >
        <b>{{ item.name || item.year }}</b>
        <span class="cv-n">{{ scores[item.year]?.totalScore || '—' }}</span>
      </button>
      <div v-if="!availableYears.length && !loading" class="cv-empty" style="min-height: 120px">
        <Activity :size="20" /><b>还没有可查询的体测学年</b>
      </div>
    </div>

    <div class="cv-center">
      <div class="cv-scroll">
        <div class="cv-ch">
          <b>{{ activeYear?.name || year || '体测' }}</b>
          <span class="meta">
            <template v-if="score">
              总分 {{ score.totalScore || '—' }} · {{ score.totalGrade || '暂无等级' }}
            </template>
          </span>
        </div>

        <div v-if="loading && !score" class="cv-empty">
          <Activity :size="20" /><b>正在读取体测成绩…</b>
        </div>
        <div v-else-if="error && !score" class="cv-empty">
          <Activity :size="20" /><b>体测成绩没有读取成功</b><span>{{ error }}</span>
        </div>
        <div v-else-if="!score" class="cv-empty">
          <Activity :size="20" /><b>还没有这个学年的体测成绩</b>
        </div>
        <template v-else>
          <div class="fz-items">
            <button
              v-for="item in items"
              :key="item.key"
              type="button"
              class="fz-item"
              :class="{ on: item.key === detailKey }"
              @click="pickItem(item.key)"
            >
              <span class="t">
                <span class="nm">{{ item.name }}</span>
                <span class="cv-badge" :class="gradeTone(item.grade)">{{ item.grade || '—' }}</span>
              </span>
              <span class="v">{{ itemScore(item) || '—' }}</span>
              <span class="s">
                <template v-if="item.converted && item.converted !== item.score">
                  换算 {{ item.converted }} 分
                </template>
                <template v-else>{{ ITEM_MEASURE[item.key] || '换算分' }}</template>
              </span>
              <span class="bar">
                <i
                  v-if="itemValue(item) != null"
                  :style="{ width: `${Math.min(100, Math.max(0, itemValue(item) as number))}%` }"
                ></i>
              </span>
            </button>
          </div>

          <div v-if="history" class="cv-panel fz-chart">
            <div class="ph2"><b>总分 · 历年</b><span class="meta">每学年一次</span></div>
            <svg viewBox="0 0 460 156">
              <line
                v-for="line in history.grid"
                :key="line.value"
                class="gl"
                x1="34"
                x2="452"
                :y1="line.y"
                :y2="line.y"
              />
              <line class="base" x1="34" x2="452" :y1="history.base" :y2="history.base" />
              <text
                v-for="line in history.grid"
                :key="`t-${line.value}`"
                x="28"
                :y="line.y + 3"
                text-anchor="end"
              >
                {{ line.value }}
              </text>
              <template v-if="history.show80">
                <line class="ref" x1="34" x2="452" :y1="history.y80" :y2="history.y80" />
                <text x="40" :y="history.y80 - 5">良好线 80</text>
              </template>
              <polyline
                v-if="history.points.length > 1"
                class="ln"
                :points="history.points.map((point) => `${point.x},${point.y}`).join(' ')"
              />
              <circle
                v-for="point in history.points"
                :key="point.year"
                class="dot"
                :cx="point.x"
                :cy="point.y"
                r="4"
              >
                <title>{{ point.label }} · {{ point.value }}</title>
              </circle>
              <text
                class="val"
                :x="history.points[history.points.length - 1].x"
                :y="history.points[history.points.length - 1].labelY"
                text-anchor="middle"
              >
                {{ history.points[history.points.length - 1].value }}
              </text>
              <text
                v-for="point in history.points"
                :key="`l-${point.year}`"
                :x="point.x"
                y="150"
                text-anchor="middle"
              >
                {{ point.label }}
              </text>
            </svg>
          </div>
          <div class="fz-note">成绩来自体质健康测试系统，应用内只读。</div>
        </template>
      </div>
    </div>

    <div class="cv-panel cv-detail fz-detail">
      <div class="cv-type">
        单项详情<span v-if="chosen?.grade" class="cv-badge" :class="gradeTone(chosen.grade)">
          {{ chosen.grade }}
        </span>
      </div>
      <h3>{{ chosen?.name || '选择一项' }}</h3>
      <div v-if="!chosen" class="cv-placeholder">点左侧单项查看原始测量值。</div>
      <template v-else>
        <div class="cv-fig">
          <span class="v">{{ itemScore(chosen) || '—' }}</span>
          <span class="k2">
            {{
              chosen.converted && chosen.converted !== chosen.score
                ? `换算 ${chosen.converted} 分`
                : '换算分'
            }}
            · {{ chosen.grade || '暂无等级' }}
          </span>
        </div>
        <div class="cv-blk">
          <h4>
            本次测量<span>{{ year }}</span>
          </h4>
          <div v-if="detailLoading && !detail" class="cv-note">正在读取原始值…</div>
          <div v-else-if="detailError" class="cv-note err">{{ detailError }}</div>
          <dl v-else-if="rawValues.length" class="cv-kv">
            <template v-for="row in rawValues" :key="row.label">
              <dt>{{ row.label }}</dt>
              <dd class="cv-num">{{ row.text }}</dd>
            </template>
            <template v-if="bmi != null">
              <dt>BMI</dt>
              <dd class="cv-num">{{ bmi }}</dd>
            </template>
          </dl>
          <div v-else class="cv-note">这个项目没有返回原始测量值。</div>
        </div>
        <div class="cv-blk">
          <h4>说明</h4>
          <p>
            原始值来自体测系统的单项详情；历史记录等实测字段确认后再展示。等级与总分为学校口径。
          </p>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.cv-cols.fz {
  grid-template-columns: minmax(150px, 178px) minmax(0, 1fr) minmax(210px, 260px);
}
.fz-items {
  display: grid;
  /* Four per row as designed; the centre column becomes a container so a
     narrow window drops to three or two instead of squeezing the cards. */
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 8px;
}
.cv-center {
  container-type: inline-size;
}
@container (max-width: 520px) {
  .fz-items {
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }
}
@container (max-width: 380px) {
  .fz-items {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
.fz-item {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  padding: 9px 11px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface);
  color: inherit;
  text-align: left;
}
.fz-item:hover:not(.on) {
  background: var(--surface-hover);
}
.fz-item.on {
  border-color: var(--accent);
  box-shadow: inset 0 0 0 1px var(--accent);
}
.fz-item .t {
  display: flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
  color: var(--text-tertiary);
  font-size: 11px;
  white-space: nowrap;
}
.fz-item .t .nm {
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
}
.fz-item .t .cv-badge {
  height: 17px;
  margin-left: auto;
  padding: 0 5px;
  font-size: 10px;
  flex-shrink: 0;
}
.fz-item .v {
  max-width: 100%;
  overflow: hidden;
  color: var(--text);
  font-size: 17px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  line-height: 1.25;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fz-item .s {
  overflow: hidden;
  color: var(--text-tertiary);
  font-size: 10.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.fz-item .bar {
  display: block;
  height: 4px;
  margin-top: 2px;
  overflow: hidden;
  border-radius: 2px;
  background: var(--surface-sunken);
}
.fz-item .bar i {
  display: block;
  height: 100%;
  border-radius: 2px;
  background: var(--v1, #4a76c9);
}
.fz-chart {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 10px 12px;
  margin-top: 2px;
}
.fz-chart .ph2 {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.fz-chart .ph2 b {
  color: var(--text);
  font-size: 12.5px;
  font-weight: 600;
}
.fz-chart .ph2 .meta {
  color: var(--text-tertiary);
  font-size: 11px;
}
.fz-chart svg {
  display: block;
  width: 100%;
}
.fz-chart text {
  fill: var(--text-tertiary);
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
}
.fz-chart text.val {
  fill: var(--text);
  font-size: 11px;
  font-weight: 600;
}
.fz-chart .gl {
  stroke: #eceef1;
  stroke-width: 1;
}
.fz-chart .base {
  stroke: var(--border-strong);
  stroke-width: 1;
}
.fz-chart .ref {
  stroke: var(--text-tertiary);
  stroke-width: 1;
  stroke-dasharray: 3 3;
}
.fz-chart .ln {
  fill: none;
  stroke: #4a76c9;
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 2;
}
.fz-chart .dot {
  fill: #4a76c9;
  stroke: #fff;
  stroke-width: 2;
}
.fz-note {
  color: var(--text-tertiary);
  font-size: 11px;
}
.cv-cols.fz .cv-item b {
  min-width: 0;
}
.cv-cols.fz .cv-item .n {
  flex-shrink: 0;
  text-align: right;
  white-space: nowrap;
}
.fz-detail {
  gap: 10px;
  overflow-x: hidden;
}
.fz-detail .cv-fig {
  gap: 3px;
}
.fz-detail .cv-fig .v {
  overflow-wrap: anywhere;
  font-size: 24px;
  line-height: 1.15;
}
.fz-detail .cv-fig .k2 {
  line-height: 1.5;
  overflow-wrap: anywhere;
}
.fz-detail .cv-kv dd {
  overflow-wrap: anywhere;
  word-break: break-word;
}
.fz-detail .cv-kv dt {
  white-space: nowrap;
}
.cv-note {
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.6;
}
.cv-note.err {
  color: var(--danger);
}
</style>
