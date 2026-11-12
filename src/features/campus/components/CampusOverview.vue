<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import UiAvatar from '../../../components/ui/UiAvatar.vue';
import { bridge } from '../../../shared/bridge';
import { cacheName } from '../../../shared/cache';
import {
  campusDateKey,
  campusParts,
  campusTimestamp,
  currentAcademicStart,
  gradeStats,
  money,
  percent,
  updatedInfo,
} from '../../../shared/domain';
import { directQuery, ensureDirectSession, siteQuery } from '../campusSession';
import {
  bmiOf,
  clockText,
  measureValue,
  mealStats,
  spendTotals,
  weekdayAverages,
  type SpendCategory,
} from '../profileOverview';
import type {
  CardInfo,
  CardTransaction,
  CardTransactions,
  FitnessItem,
  FitnessItemDetail,
  FitnessScore,
  FitnessYear,
  FitnessYears,
  GradeRow,
  Grades,
  ProfileInfo,
  SchoolQuery,
} from '../../../types';

/**
 * 个人画像 → 概览 (layout v2): an identity strip with four headline figures,
 * then 学业 / 消费 / 健康. Every block loads on its own so one unavailable
 * service (card and fitness reject WebVPN) leaves the other two readable.
 * Figures are calculated from the rows on screen and say so; nothing here is
 * a school-official statistic.
 */
const props = withDefaults(
  defineProps<{
    accessMode: 'direct' | 'webvpn' | null;
    directSession?: boolean;
    /** The avatar the user configured, used when the school picture fails. */
    avatar?: string | null;
  }>(),
  { directSession: false, avatar: null },
);
const emit = defineEmits<{ 'open-section': [id: string] }>();

const profile = ref<ProfileInfo | null>(null);
const grades = ref<GradeRow[]>([]);
const card = ref<CardInfo | null>(null);
const transactions = ref<CardTransaction[]>([]);
const fitness = ref<FitnessScore | null>(null);
const fitnessYear = ref<FitnessYear | null>(null);
const body = ref<FitnessItemDetail | null>(null);
const directReady = ref(false);
const directBusy = ref(false);
const loadedAt = ref<number | null>(null);
const busy = ref<Record<'profile' | 'grades' | 'card' | 'fitness', boolean>>({
  profile: false,
  grades: false,
  card: false,
  fitness: false,
});
const errors = ref<Record<'profile' | 'grades' | 'card' | 'fitness', string>>({
  profile: '',
  grades: '',
  card: '',
  fitness: '',
});

const message = (error: unknown) => (error instanceof Error ? error.message : String(error));
const hasDirect = computed(() => props.directSession || directReady.value);
// Card and fitness reject WebVPN itself; asking on mount would open a login
// the user did not request. The panel offers the login instead.
const directBlocked = computed(() => props.accessMode === 'webvpn' && !hasDirect.value);

/* ---- identity ---- */
const whoLines = computed(() => {
  const value = profile.value;
  const first = [value?.major, value?.className].filter(Boolean).join(' · ');
  const second = [
    value?.grade ? `${value.grade} 级` : '',
    value?.sex,
    value?.studentNo ? `学号 ${value.studentNo}` : '',
  ]
    .filter(Boolean)
    .join(' · ');
  return [first, second].filter(Boolean);
});

/* ---- 学业 ---- */
const stats = computed(() => gradeStats(grades.value));
function numeric(text: string): number | null {
  const trimmed = (text ?? '').trim();
  if (!/^\d+(\.\d+)?$/.test(trimmed)) return null;
  const value = Number(trimmed);
  return Number.isFinite(value) ? value : null;
}
function termLabel(term: string): string {
  const parts = term.split('-');
  if (parts.length !== 3 || parts[0].length !== 4) return term;
  const year = parts[0].slice(2);
  const half = parts[2] === '1' ? '秋' : parts[2] === '2' ? '春' : '夏';
  return `${year} ${half}`;
}
const termStats = computed(() => {
  const map = new Map<string, GradeRow[]>();
  for (const row of grades.value) {
    if (!row.term) continue;
    const list = map.get(row.term);
    if (list) list.push(row);
    else map.set(row.term, [row]);
  }
  return [...map.entries()]
    .map(([term, rows]) => ({ term, label: termLabel(term), stats: gradeStats(rows) }))
    .filter((item) => item.stats.gpa != null)
    .sort((left, right) => left.term.localeCompare(right.term))
    .slice(-8);
});
const gpaDelta = computed(() => {
  const list = termStats.value;
  if (list.length < 2) return null;
  const last = list[list.length - 1].stats.gpa;
  const previous = list[list.length - 2].stats.gpa;
  if (last == null || previous == null) return null;
  return last - previous;
});
const gpaChart = computed(() => {
  const items = termStats.value;
  if (!items.length) return null;
  const values = items.map((item) => item.stats.gpa as number);
  let lo = Math.floor((Math.min(...values) - 0.2) * 10) / 10;
  let hi = Math.ceil((Math.max(...values) + 0.1) * 10) / 10;
  if (hi - lo < 0.4) {
    lo = Math.max(0, Math.round((lo - 0.2) * 10) / 10);
    hi = Math.round((hi + 0.2) * 10) / 10;
  }
  const left = 44;
  const right = 279;
  const xAt = (index: number) =>
    items.length === 1 ? (left + right) / 2 : left + (index * (right - left)) / (items.length - 1);
  const yAt = (value: number) => 110 - ((value - lo) / (hi - lo)) * 100;
  return {
    lo,
    mid: Math.round(((lo + hi) / 2) * 100) / 100,
    hi,
    items: items.length,
    points: items.map((item, index) => ({
      x: xAt(index),
      y: yAt(item.stats.gpa as number),
      value: item.stats.gpa as number,
      label: item.label,
    })),
    showLabel: (index: number) =>
      items.length <= 6 ||
      index === 0 ||
      index === items.length - 1 ||
      index === Math.floor(items.length / 2),
  };
});
const creditCategories = computed(() => {
  const map = new Map<string, number>();
  for (const row of grades.value) {
    if (row.passed === false) continue;
    const credit = row.credits ?? 0;
    if (credit <= 0) continue;
    const key = row.category || '其他';
    map.set(key, (map.get(key) ?? 0) + credit);
  }
  const list = [...map.entries()]
    .map(([name, credits]) => ({ name, credits }))
    .sort((left, right) => right.credits - left.credits);
  const max = list[0]?.credits ?? 0;
  return list.slice(0, 4).map((item) => ({ ...item, share: max ? item.credits / max : 0 }));
});
const BANDS = [
  { label: '优秀 ≥90', min: 90 },
  { label: '良好 80–89', min: 80 },
  { label: '中等 70–79', min: 70 },
  { label: '及格 60–69', min: 60 },
  { label: '不及格 <60', min: 0 },
];
const distribution = computed(() => {
  const counts = new Array(BANDS.length).fill(0) as number[];
  for (const row of grades.value) {
    const value = numeric(row.score);
    if (value == null || value < 0 || value > 100) continue;
    const index = BANDS.findIndex((band) => value >= band.min);
    if (index >= 0) counts[index] += 1;
  }
  const max = Math.max(...counts, 1);
  return BANDS.map((band, index) => ({
    label: band.label,
    count: counts[index],
    share: counts[index] / max,
  })).filter((band) => band.count > 0);
});

/* ---- 消费 ---- */
const SPEND_COLORS: Record<SpendCategory, string> = {
  meal: 'var(--v1)',
  market: 'var(--v2)',
  water: 'var(--v3)',
  other: 'var(--v-other)',
};
const spend = computed(() => spendTotals(transactions.value));
const donut = computed(() => {
  const radius = 52;
  const circumference = 2 * Math.PI * radius;
  let offset = 0;
  return {
    total: spend.value.total,
    items: spend.value.categories.map((item) => {
      const length = item.share * circumference;
      const segment = {
        ...item,
        color: SPEND_COLORS[item.key],
        dash: `${length} ${circumference - length}`,
        offset: -offset,
      };
      offset += length;
      return segment;
    }),
  };
});
const dailySpend = computed(() => Math.round(spend.value.total / 30));
const meals = computed(() => {
  const value = mealStats(transactions.value);
  return { ...value, meals: value.meals.filter((meal) => meal.days > 0) };
});
const weekdays = computed(() => weekdayAverages(transactions.value, 30));
const weekdayMax = computed(() => Math.max(...weekdays.value.values, 1));
const todayWeekday = computed(() => {
  const parts = campusParts(Date.now());
  return parts ? (parts.weekday + 6) % 7 : -1;
});

/* ---- 健康 ---- */
const heightCm = computed(() => measureValue(body.value?.info?.value_1));
const weightKg = computed(() => measureValue(body.value?.info?.value_2));
const bmi = computed(() => bmiOf(heightCm.value, weightKg.value));
const bmiLeft = computed(() =>
  bmi.value == null ? null : Math.min(98, Math.max(2, (bmi.value / 45) * 100)),
);
const bmiGrade = computed(
  () => fitness.value?.items.find((item) => item.key === 'bmi')?.grade ?? '',
);
const HEALTH_KEYS = ['vc', '50m', 'jump', 'run'];
const fitnessMeters = computed(() => {
  const items = fitness.value?.items ?? [];
  const map = new Map(items.map((item) => [item.key, item]));
  const picked: FitnessItem[] = HEALTH_KEYS.map((key) => map.get(key)).filter(
    (item): item is FitnessItem => !!item,
  );
  return picked
    .map((item) => ({
      key: item.key,
      name: item.name,
      grade: item.grade,
      value: numeric(item.converted) ?? numeric(item.score),
    }))
    .filter((item) => item.value != null);
});

/* ---- loading ---- */
async function loadProfile() {
  busy.value.profile = true;
  errors.value.profile = '';
  try {
    profile.value = await siteQuery<ProfileInfo>(
      { type: 'profile' },
      'profile',
      () => bridge.refreshProfileSession(),
      'profile',
    );
    loadedAt.value = Date.now();
  } catch (error) {
    console.error('[campus] overview profile', error);
    errors.value.profile = message(error);
  } finally {
    busy.value.profile = false;
  }
}

async function loadGrades() {
  busy.value.grades = true;
  errors.value.grades = '';
  try {
    const value = (await bridge.schoolQuery({ type: 'grades', term: null }, 'grades')) as Grades;
    grades.value = value.rows ?? [];
    loadedAt.value = Date.now();
  } catch (error) {
    console.error('[campus] overview grades', error);
    errors.value.grades = message(error);
  } finally {
    busy.value.grades = false;
  }
}

async function readTransactions(): Promise<CardTransaction[]> {
  const end = campusDateKey(Date.now());
  const start = campusDateKey(campusTimestamp(`${end} 00:00`) - 29 * 86400000);
  const rows: CardTransaction[] = [];
  // The service pages by 30; a busy month can span a few pages. Ten pages is a
  // hard stop so the overview never turns into an unbounded crawl.
  for (let page = 1; page <= 10; page++) {
    const params = { start, end, page };
    const value = await directQuery<CardTransactions>(
      { type: 'cardTransactions', ...params },
      cacheName('cardTransactions', params),
    );
    const batch = value.rows ?? [];
    rows.push(...batch);
    if (!batch.length || rows.length >= value.total) break;
  }
  return rows;
}

async function loadCard() {
  if (directBlocked.value) return;
  busy.value.card = true;
  errors.value.card = '';
  try {
    const [info, rows] = await Promise.all([
      directQuery<CardInfo>({ type: 'card' }, 'card'),
      readTransactions(),
    ]);
    card.value = info;
    transactions.value = rows;
    loadedAt.value = Date.now();
  } catch (error) {
    console.error('[campus] overview card', error);
    errors.value.card = message(error);
  } finally {
    busy.value.card = false;
  }
}

// The fitness platform has its own H5 login; the shared direct session only
// gets through CAS, not the PHP session behind year/score/measurement data.
const fitnessQuery = <T,>(query: SchoolQuery, cache?: string) =>
  siteQuery<T>(query, 'fitness', () => bridge.refreshFitnessSession(), cache);

async function loadFitness() {
  if (directBlocked.value) return;
  busy.value.fitness = true;
  errors.value.fitness = '';
  try {
    const years = await fitnessQuery<FitnessYears>({ type: 'fitness' }, 'fitness');
    const academicStart = currentAcademicStart();
    const available = (years.rows ?? [])
      .filter((item) => /^\d{4}$/.test(item.year) && Number(item.year) <= academicStart)
      .sort((left, right) => right.year.localeCompare(left.year));
    const year = available.find((item) => item.checked) ?? available[0];
    fitnessYear.value = year ?? null;
    if (!year) {
      fitness.value = null;
      body.value = null;
      return;
    }
    const scoreParams = { year: year.year };
    const score = await fitnessQuery<FitnessScore>(
      { type: 'fitnessScore', ...scoreParams },
      cacheName('fitnessScore', scoreParams),
    );
    fitness.value = score;
    loadedAt.value = Date.now();
    // Raw height/weight is a separate request. Its failure must not blank the
    // score that just loaded; the health block simply omits the BMI.
    const itemParams = { item: 1, year: year.year };
    try {
      body.value = await fitnessQuery<FitnessItemDetail>(
        { type: 'fitnessItem', ...itemParams },
        cacheName('fitnessItem', itemParams),
      );
    } catch (error) {
      console.error('[campus] overview fitness item', error);
      body.value = null;
    }
  } catch (error) {
    console.error('[campus] overview fitness', error);
    errors.value.fitness = message(error);
  } finally {
    busy.value.fitness = false;
  }
}

async function enableDirect() {
  directBusy.value = true;
  errors.value.card = '';
  errors.value.fitness = '';
  try {
    await ensureDirectSession();
    directReady.value = true;
    await Promise.all([loadCard(), loadFitness()]);
  } catch (error) {
    console.error('[campus] overview direct login', error);
    errors.value.card = message(error);
    errors.value.fitness = message(error);
  } finally {
    directBusy.value = false;
  }
}

function refresh() {
  void loadProfile();
  void loadGrades();
  if (!directBlocked.value) {
    void loadCard();
    void loadFitness();
  }
}

// The workspace may have opened a direct session from another section.
watch(
  () => props.directSession,
  (ready) => {
    if (ready && !directReady.value) {
      directReady.value = true;
      void loadCard();
      void loadFitness();
    }
  },
);

onMounted(() => {
  void loadProfile();
  void loadGrades();
  if (!directBlocked.value) {
    void loadCard();
    void loadFitness();
  }
});
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="loadedAt">更新于 {{ updatedInfo(loadedAt) }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <button
      type="button"
      class="cv-btn"
      :disabled="Object.values(busy).some(Boolean)"
      @click="refresh"
    >
      刷新
    </button>
  </CampusHeaderSlot>

  <div class="cv-cols ov">
    <div class="cv-panel ov-id">
      <UiAvatar
        :src="profile?.pictureUrl || null"
        :fallback="avatar"
        :size="48"
        :alt="profile?.name || '学生头像'"
      />
      <div class="who">
        <b>{{ profile?.name || (busy.profile ? '正在读取…' : '姓名未提供') }}</b>
        <span v-for="(line, index) in whoLines" :key="index">{{ line }}</span>
        <span v-if="errors.profile" class="err">{{ errors.profile }}</span>
      </div>
      <div class="kpis">
        <div class="kpi">
          <span class="k2">总 GPA</span>
          <span class="v">{{ stats.gpa == null ? '—' : stats.gpa.toFixed(2) }}</span>
          <span class="d">
            <template v-if="gpaDelta != null">
              <span :class="gpaDelta >= 0 ? 'up' : 'down'">
                {{ gpaDelta >= 0 ? '↑' : '↓' }} {{ Math.abs(gpaDelta).toFixed(2) }}
              </span>
              较上学期
            </template>
            <template v-else>按课程学分加权</template>
          </span>
        </div>
        <div class="kpi">
          <span class="k2">已修学分</span>
          <span class="v">{{ stats.credits ? stats.credits.toFixed(1) : '—' }}</span>
          <span class="d">
            {{ termStats.length ? `${termStats.length} 个学期` : '—' }} · {{ stats.courses }} 门课
          </span>
        </div>
        <div class="kpi">
          <span class="k2">校园卡余额</span>
          <span class="v">{{ card ? money(card.balance) : '—' }}</span>
          <span class="d">
            <template v-if="transactions.length">近 30 天支出 {{ money(spend.total) }}</template>
            <template v-else>近 30 天无支出记录</template>
          </span>
        </div>
        <div class="kpi">
          <span class="k2">体测总分</span>
          <span class="v">{{ fitness?.totalScore || '—' }}</span>
          <span class="d">
            <span v-if="fitness?.totalGrade" class="badge">{{ fitness.totalGrade }}</span>
            {{ fitnessYear?.name || '暂无体测' }}
          </span>
        </div>
      </div>
    </div>

    <div class="cv-panel ov-panel chart">
      <div class="ph2"><b>学业</b><span class="meta">学期绩点</span></div>
      <div v-if="busy.grades && !grades.length" class="ov-note">正在读取成绩…</div>
      <div v-else-if="errors.grades && !grades.length" class="ov-note err">{{ errors.grades }}</div>
      <div v-else-if="!gpaChart" class="ov-note">还没有成绩数据</div>
      <svg v-else viewBox="0 0 300 136">
        <line class="gl" x1="30" x2="296" :y1="10" y2="10" />
        <line class="gl" x1="30" x2="296" :y1="60" y2="60" />
        <line class="base" x1="30" x2="296" :y1="110" y2="110" />
        <text x="24" y="13" text-anchor="end">{{ gpaChart.hi.toFixed(1) }}</text>
        <text x="24" y="63" text-anchor="end">{{ gpaChart.mid.toFixed(1) }}</text>
        <text x="24" y="113" text-anchor="end">{{ gpaChart.lo.toFixed(1) }}</text>
        <polyline
          v-if="gpaChart.points.length > 1"
          class="ln"
          :points="gpaChart.points.map((point) => `${point.x},${point.y}`).join(' ')"
        />
        <circle
          v-for="(point, index) in gpaChart.points"
          :key="index"
          class="dot"
          :cx="point.x"
          :cy="point.y"
          r="4"
        >
          <title>{{ point.label }} · 绩点 {{ point.value.toFixed(2) }}</title>
        </circle>
        <text
          class="val"
          :x="gpaChart.points[gpaChart.points.length - 1].x"
          :y="gpaChart.points[gpaChart.points.length - 1].y - 10"
          text-anchor="middle"
        >
          {{ gpaChart.points[gpaChart.points.length - 1].value.toFixed(2) }}
        </text>
        <text
          v-for="(point, index) in gpaChart.points"
          v-show="gpaChart.showLabel(index)"
          :key="`label-${index}`"
          :x="point.x"
          y="128"
          text-anchor="middle"
        >
          {{ point.label }}
        </text>
      </svg>

      <template v-if="creditCategories.length">
        <div class="blkh">已修学分 · 按课程性质</div>
        <div class="meters">
          <div v-for="item in creditCategories" :key="item.name" class="meter">
            <span class="ellip">{{ item.name }}</span>
            <span class="bar"><i :style="{ width: `${item.share * 100}%` }"></i></span>
            <span class="n">{{ item.credits.toFixed(1) }}</span>
          </div>
        </div>
      </template>
      <template v-if="distribution.length">
        <div class="blkh">成绩分布 · {{ stats.courses }} 门</div>
        <div class="meters">
          <div v-for="band in distribution" :key="band.label" class="meter">
            <span class="ellip">{{ band.label }}</span>
            <span class="bar"><i :style="{ width: `${band.share * 100}%` }"></i></span>
            <span class="n">{{ band.count }}</span>
          </div>
        </div>
      </template>
      <div class="ov-note">按成绩行计算，以学校系统为准</div>
    </div>

    <div class="cv-panel ov-panel chart">
      <div class="ph2">
        <b>消费</b><span class="meta">近 30 天 · 按商户名归类</span>
        <button type="button" class="more" @click="emit('open-section', 'card')">校园卡 →</button>
      </div>
      <div v-if="directBlocked" class="ov-note">
        校园卡需要直连登录，当前是 WebVPN 会话。
        <button type="button" class="cv-link" :disabled="directBusy" @click="enableDirect">
          {{ directBusy ? '正在登录…' : '登录后读取' }}
        </button>
      </div>
      <div v-else-if="busy.card && !transactions.length" class="ov-note">正在读取校园卡…</div>
      <div v-else-if="errors.card && !transactions.length" class="ov-note err">
        {{ errors.card }}
      </div>
      <div v-else-if="!donut.total" class="ov-note">近 30 天没有校园卡支出</div>
      <template v-else>
        <div class="donut">
          <svg viewBox="0 0 132 132">
            <g transform="rotate(-90 66 66)" fill="none" stroke-width="16">
              <circle
                v-for="segment in donut.items"
                :key="segment.key"
                cx="66"
                cy="66"
                r="52"
                :stroke="segment.color"
                :stroke-dasharray="segment.dash"
                :stroke-dashoffset="segment.offset"
              >
                <title>
                  {{ segment.label }} {{ money(segment.amount) }} ·
                  {{ percent(segment.share) }}
                </title>
              </circle>
            </g>
            <text class="c" x="66" y="66" text-anchor="middle">{{ money(donut.total) }}</text>
            <text class="cs2" x="66" y="82" text-anchor="middle">日均 {{ money(dailySpend) }}</text>
          </svg>
          <div class="lg">
            <div v-for="segment in donut.items" :key="segment.key">
              <i :style="{ background: segment.color }"></i>
              <span>{{ segment.label }}</span>
              <span class="n">{{ money(segment.amount) }}</span>
              <span class="p">{{ percent(segment.share) }}</span>
            </div>
          </div>
        </div>

        <template v-if="meals.meals.length">
          <div class="blkh">用餐习惯 · 刷卡时间中位数</div>
          <div class="stats">
            <div v-for="meal in meals.meals" :key="meal.key" class="stat">
              <div class="k2">{{ meal.label }}</div>
              <div class="v">
                {{ clockText(meal.minutes) }}<small>{{ meal.days }} 天</small>
              </div>
            </div>
            <div v-if="meals.topMerchant" class="stat">
              <div class="k2">最常去</div>
              <div class="v small">
                {{ meals.topMerchant.name }}<small>{{ meals.topMerchant.count }} 次</small>
              </div>
            </div>
          </div>
        </template>

        <template v-if="spend.total">
          <div class="blkh">按星期 · 日均支出</div>
          <div class="wkv">
            <span v-for="(value, index) in weekdays.values" :key="`v-${index}`">
              {{ (value / 100).toFixed(0) }}
            </span>
          </div>
          <div class="wk">
            <i
              v-for="(value, index) in weekdays.values"
              :key="`b-${index}`"
              :class="{ on: index === todayWeekday }"
              :style="{ height: `${Math.max(4, (value / weekdayMax) * 100)}%` }"
            ></i>
          </div>
          <div class="wkl">
            <span v-for="(label, index) in weekdays.labels" :key="`l-${index}`">{{ label }}</span>
          </div>
        </template>
        <div class="ov-note">按商户名归类，金额为近 30 天合计</div>
      </template>
    </div>

    <div class="cv-panel ov-panel">
      <div class="ph2">
        <b>健康</b>
        <span class="meta">{{ fitnessYear ? `${fitnessYear.name} 体测` : '体测' }}</span>
        <button type="button" class="more" @click="emit('open-section', 'fitness')">体测 →</button>
      </div>
      <div v-if="directBlocked" class="ov-note">
        体测需要直连登录，当前是 WebVPN 会话。
        <button type="button" class="cv-link" :disabled="directBusy" @click="enableDirect">
          {{ directBusy ? '正在登录…' : '登录后读取' }}
        </button>
      </div>
      <div v-else-if="busy.fitness && !fitness" class="ov-note">正在读取体测成绩…</div>
      <div v-else-if="errors.fitness && !fitness" class="ov-note err">{{ errors.fitness }}</div>
      <div v-else-if="!fitness" class="ov-note">还没有体测成绩</div>
      <template v-else>
        <div class="body">
          <svg viewBox="0 0 120 200" aria-hidden="true">
            <line v-if="heightCm" class="dim" x1="10" x2="10" y1="12" y2="186" />
            <line v-if="heightCm" class="dim" x1="6" x2="14" y1="12" y2="12" />
            <line v-if="heightCm" class="dim" x1="6" x2="14" y1="186" y2="186" />
            <text
              v-if="heightCm"
              x="20"
              y="104"
              transform="rotate(-90 20 100)"
              text-anchor="middle"
            >
              {{ heightCm }} cm
            </text>
            <circle class="sil" cx="72" cy="26" r="13" />
            <rect class="sil" x="52" y="44" width="40" height="64" rx="12" />
            <rect class="sil" x="39" y="48" width="11" height="56" rx="5.5" />
            <rect class="sil" x="94" y="48" width="11" height="56" rx="5.5" />
            <rect class="sil" x="54" y="106" width="16" height="80" rx="7" />
            <rect class="sil" x="74" y="106" width="16" height="80" rx="7" />
          </svg>
          <div class="stats one">
            <div class="stat">
              <div class="k2">身高 / 体重</div>
              <div class="v">
                {{ heightCm ?? '—' }}<small>cm</small> · {{ weightKg ?? '—' }}<small>kg</small>
              </div>
            </div>
            <div class="stat">
              <div class="k2">BMI</div>
              <div class="v">
                {{ bmi ?? '—' }}<small v-if="bmiGrade">{{ bmiGrade }}</small>
              </div>
            </div>
            <div v-if="bmiLeft != null" class="bmi">
              <div class="range">
                <i class="b1"></i><i class="b2"></i><i class="b3"></i><i class="b4"></i>
                <span class="pin" :style="{ left: `${bmiLeft}%` }"></span>
              </div>
              <div class="rl">
                <span>偏瘦</span><span>正常</span><span>超重</span><span>肥胖</span>
              </div>
            </div>
          </div>
        </div>
        <template v-if="fitnessMeters.length">
          <div class="blkh">体测单项 · 换算分</div>
          <div class="meters">
            <div v-for="item in fitnessMeters" :key="item.key" class="meter">
              <span class="ellip">{{ item.name }}</span>
              <span class="bar"><i :style="{ width: `${item.value}%` }"></i></span>
              <span class="n">{{ item.value }}</span>
            </div>
          </div>
        </template>
        <div class="stats two">
          <div class="stat">
            <div class="k2">总分</div>
            <div class="v">
              {{ fitness.totalScore || '—' }}<small>{{ fitness.totalGrade }}</small>
            </div>
          </div>
          <div class="stat">
            <div class="k2">报告</div>
            <div class="v small">{{ fitness.reportType || '—' }}</div>
          </div>
        </div>
        <div class="ov-note">原始值与历史记录见体测页</div>
      </template>
    </div>
  </div>
</template>

<style scoped>
/* Chart colours follow the validated categorical order; the third colour sits
   just under 3:1 on the light surface, so every slice carries a text label. */
.cv-cols.ov {
  --v1: #4a76c9;
  --v2: #16a085;
  --v3: #c08a22;
  --v-other: #c3c8d0;
  --grid: #eceef1;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  grid-template-rows: auto minmax(0, 1fr);
}

.ov-id {
  grid-column: 1 / -1;
  display: grid;
  grid-template-columns: 48px minmax(0, 1.1fr) minmax(0, 2.4fr);
  align-items: center;
  gap: 14px 16px;
  padding: 14px 18px;
}
.ov-id .who {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.ov-id .who b {
  color: var(--text);
  font-size: 16px;
  font-weight: 600;
}
.ov-id .who span {
  overflow: hidden;
  color: var(--text-secondary);
  font-size: 12px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ov-id .who .err {
  color: var(--danger);
}
.kpis {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 12px;
}
.kpi {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
  border-left: 1px solid var(--divider);
  padding-left: 14px;
}
.kpi .k2 {
  color: var(--text-tertiary);
  font-size: 11.5px;
  white-space: nowrap;
}
.kpi .v {
  overflow: hidden;
  color: var(--text);
  font-size: 20px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
  line-height: 1.3;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.kpi .d {
  overflow: hidden;
  color: var(--text-tertiary);
  font-size: 11px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.kpi .d .badge {
  padding: 1px 5px;
  border-radius: var(--radius-full);
  background: var(--success-soft);
  color: var(--success-text);
  font-size: 10.5px;
}
.up {
  color: var(--success-text);
}
.down {
  color: var(--danger);
}

.ov-panel {
  display: flex;
  flex-direction: column;
  gap: 10px;
  min-height: 0;
  padding: 14px 16px;
  overflow: auto;
}
.ph2 {
  display: flex;
  align-items: baseline;
  gap: 8px;
}
.ph2 b {
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
}
.ph2 .meta {
  overflow: hidden;
  color: var(--text-tertiary);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ph2 .more {
  margin-left: auto;
  padding: 0;
  border: 0;
  background: none;
  color: var(--accent-text);
  font-size: 11.5px;
  white-space: nowrap;
}
.ph2 .more:hover {
  text-decoration: underline;
}
.ov-note {
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.6;
}
.ov-note.err {
  color: var(--danger);
}
.blkh {
  border-top: 1px solid var(--divider);
  padding-top: 10px;
  color: var(--text-secondary);
  font-size: 11.5px;
  font-weight: 600;
}

.chart svg {
  display: block;
  width: 100%;
}
.chart text {
  fill: var(--text-tertiary);
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
}
.chart text.val {
  fill: var(--text);
  font-size: 11px;
  font-weight: 600;
}
.chart .gl {
  stroke: var(--grid);
  stroke-width: 1;
}
.chart .base {
  stroke: var(--border-strong);
  stroke-width: 1;
}
.chart .ln {
  fill: none;
  stroke: var(--v1);
  stroke-linecap: round;
  stroke-linejoin: round;
  stroke-width: 2;
}
.chart .dot {
  fill: var(--v1);
  stroke: #fff;
  stroke-width: 2;
}

/* The panel is about 300px wide in the app (the design sheet was wider), so
   the legend sits under the ring instead of beside it. */
.donut {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
}
.donut svg {
  width: 112px;
  height: 112px;
  flex-shrink: 0;
}
.donut .lg {
  width: 100%;
}
.donut .c {
  fill: var(--text);
  font-size: 17px;
  font-weight: 600;
}
.donut .cs2 {
  fill: var(--text-tertiary);
  font-size: 10.5px;
}
.lg {
  display: flex;
  flex-direction: column;
  gap: 7px;
  font-size: 12px;
}
.lg div {
  display: grid;
  grid-template-columns: 10px minmax(0, 1fr) auto 40px;
  align-items: center;
  gap: 7px;
  white-space: nowrap;
}
.lg i {
  display: block;
  width: 10px;
  height: 10px;
  border-radius: 3px;
}
.lg .n {
  color: var(--text);
  font-variant-numeric: tabular-nums;
  text-align: right;
}
.lg .p {
  color: var(--text-tertiary);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.meters {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.meter {
  display: grid;
  grid-template-columns: 84px minmax(0, 1fr) 40px;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.meter .ellip {
  overflow: hidden;
  color: var(--text-secondary);
  text-overflow: ellipsis;
  white-space: nowrap;
}
.meter .bar {
  height: 8px;
  overflow: hidden;
  border-radius: 4px;
  background: var(--surface-sunken);
}
.meter .bar i {
  display: block;
  height: 100%;
  border-radius: 4px;
  background: var(--v1);
}
.meter .n {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
  text-align: right;
}

.stats {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 12px;
}
.stats.one {
  grid-template-columns: 1fr;
}
.stats.two {
  border-top: 1px solid var(--divider);
  padding-top: 10px;
}
.stat .k2 {
  color: var(--text-tertiary);
  font-size: 11px;
}
.stat .v {
  color: var(--text);
  font-size: 16px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.stat .v.small {
  font-size: 13px;
}
.stat .v small {
  margin-left: 3px;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 400;
}

.body {
  display: grid;
  grid-template-columns: 118px minmax(0, 1fr);
  align-items: center;
  gap: 12px;
}
.body svg {
  width: 118px;
  height: 196px;
}
.body .sil {
  fill: #e6eaf1;
  stroke: #b9c4d6;
  stroke-width: 1.2;
}
.body .dim {
  stroke: var(--text-tertiary);
  stroke-width: 1;
}
.body text {
  fill: var(--text-secondary);
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
}
.bmi {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.range {
  position: relative;
  display: grid;
  grid-template-columns: 18.5fr 5.5fr 4fr 17fr;
  gap: 2px;
  height: 8px;
  border-radius: 4px;
}
.range i {
  display: block;
  border-radius: 2px;
}
.range .b1 {
  background: var(--surface-sunken);
}
.range .b2 {
  background: var(--success-soft);
}
.range .b3 {
  background: var(--warning-soft);
}
.range .b4 {
  background: var(--danger-soft);
}
.range .pin {
  position: absolute;
  top: -4px;
  width: 2px;
  height: 16px;
  border-radius: 1px;
  background: var(--text);
}
.rl {
  display: flex;
  justify-content: space-between;
  color: var(--text-tertiary);
  font-size: 10.5px;
}

.wk {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  align-items: end;
  gap: 6px;
  height: 62px;
}
.wk i {
  display: block;
  border-radius: 4px 4px 0 0;
  background: var(--seq-2, #b3c8ec);
}
.wk i.on {
  background: var(--v1);
}
.wkl,
.wkv {
  display: grid;
  grid-template-columns: repeat(7, 1fr);
  gap: 6px;
  text-align: center;
}
.wkl {
  color: var(--text-tertiary);
  font-size: 10.5px;
}
.wkv {
  color: var(--text-secondary);
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
}

@media (max-width: 1100px) {
  .cv-cols.ov {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-template-rows: auto minmax(0, 1fr) minmax(0, 1fr);
  }
  .ov-id {
    grid-template-columns: 48px minmax(0, 1fr);
  }
  .kpis {
    grid-column: 1 / -1;
  }
}
</style>
