<script setup lang="ts">
import { computed, onMounted, ref, type Component } from 'vue';
import {
  Activity,
  ArrowDown,
  ArrowLeft,
  ArrowUp,
  BarChart3,
  BookMarked,
  BookOpen,
  BookOpenText,
  CalendarCheck,
  CalendarDays,
  ChevronRight,
  ClipboardCheck,
  ClipboardList,
  CreditCard,
  DoorOpen,
  IdCard,
  MapPin,
} from 'lucide-vue-next';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import IconTile from './ui/IconTile.vue';
import EmptyState from './ui/EmptyState.vue';
import CampusRooms from './campus/CampusRooms.vue';
import CampusCourses from './campus/CampusCourses.vue';
import CampusEvaluations from './campus/CampusEvaluations.vue';
import CampusFitness from './campus/CampusFitness.vue';
import CampusProfile from './campus/CampusProfile.vue';
import CampusLms from './campus/CampusLms.vue';
import CampusLibrary from './campus/CampusLibrary.vue';
import CampusVenue from './campus/CampusVenue.vue';
import { bridge } from '../bridge';
import { directQuery, ensureDirectSession } from '../campusSession';
import type {
  AttendanceStreams,
  CalendarList,
  CardInfo,
  CardTransactions,
  Exams,
  Grades,
  SchoolFeature,
} from '../types';

const props = defineProps<{
  name: string;
  pinned: string[];
  accessMode: 'direct' | 'webvpn' | null;
  directSession?: boolean;
}>();
const emit = defineEmits<{ 'update-pinned': [ids: string[]]; 'direct-session': [] }>();

type Tone = 'blue' | 'green' | 'orange' | 'violet' | 'teal' | 'gray';
interface Tool {
  id: string;
  name: string;
  description: string;
  icon: Component;
  tone: Tone;
}

const SECTIONS: { title: string; tools: Tool[] }[] = [
  {
    title: '学习',
    tools: [
      {
        id: 'grades',
        name: '成绩查询',
        description: '学期成绩与绩点',
        icon: BarChart3,
        tone: 'blue',
      },
      {
        id: 'exams',
        name: '考试安排',
        description: '考试时间与考场',
        icon: ClipboardList,
        tone: 'violet',
      },
      {
        id: 'rooms',
        name: '空闲教室',
        description: '按日期与节次查询',
        icon: DoorOpen,
        tone: 'teal',
      },
      {
        id: 'courses',
        name: '全校课程',
        description: '按学期与课程名检索',
        icon: BookOpenText,
        tone: 'green',
      },
      {
        id: 'calendar',
        name: '校历',
        description: '学年与假期安排',
        icon: CalendarDays,
        tone: 'orange',
      },
      {
        id: 'evaluations',
        name: '一键评教',
        description: '预览后由你确认提交',
        icon: ClipboardCheck,
        tone: 'gray',
      },
    ],
  },
  {
    title: '校园生活',
    tools: [
      {
        id: 'attendance',
        name: '考勤查询',
        description: '刷卡流水与课程考勤',
        icon: CalendarCheck,
        tone: 'blue',
      },
      {
        id: 'card',
        name: '校园卡',
        description: '余额与最近流水',
        icon: CreditCard,
        tone: 'green',
      },
      {
        id: 'library',
        name: '图书馆',
        description: '楼层与实时座位',
        icon: BookMarked,
        tone: 'teal',
      },
      {
        id: 'fitness',
        name: '体测成绩',
        description: '体测项目与总分',
        icon: Activity,
        tone: 'orange',
      },
      {
        id: 'venues',
        name: '场馆预约',
        description: '场馆时段与我的预订',
        icon: MapPin,
        tone: 'violet',
      },
      { id: 'profile', name: '学籍档案', description: '个人学籍信息', icon: IdCard, tone: 'gray' },
      { id: 'lms', name: '思源学堂', description: '课程与活动', icon: BookOpen, tone: 'blue' },
    ],
  },
];

const READY = [
  'card',
  'grades',
  'exams',
  'attendance',
  'calendar',
  'rooms',
  'courses',
  'evaluations',
  'fitness',
  'profile',
  'lms',
  'library',
  'venues',
];
const TOOLS = SECTIONS.flatMap((section) => section.tools);
const pinnedTools = computed(() =>
  props.pinned
    .map((id) => TOOLS.find((item) => item.id === id))
    .filter((item): item is Tool => !!item),
);
const visibleSections = computed(() =>
  SECTIONS.map((section) => ({
    title: section.title,
    tools: section.tools.filter((item) => !props.pinned.includes(item.id)),
  })),
);
const currentTool = computed(() => TOOLS.find((item) => item.id === tool.value));
const title = computed(() => currentTool.value?.name ?? props.name);
function pin(id: string) {
  if (!props.pinned.includes(id)) emit('update-pinned', [id, ...props.pinned]);
}
function unpin(id: string) {
  emit(
    'update-pinned',
    props.pinned.filter((item) => item !== id),
  );
}
const features = ref<SchoolFeature[]>([]);
const tool = ref('');
const error = ref('');
const loading = ref(false);
const info = ref<CardInfo | null>(null);
const transactions = ref<CardTransactions | null>(null);
const grades = ref<Grades | null>(null);
const exams = ref<Exams | null>(null);
const streams = ref<AttendanceStreams | null>(null);
const calendar = ref<CalendarList | null>(null);
const page = ref(1);

const backend = (id: string) => features.value.find((feature) => feature.id === id)?.backend;
const directReady = ref(false);
const hasDirect = computed(() => props.directSession || directReady.value);
// These services have no WebVPN route; a direct login unlocks them.
const DIRECT_ONLY = ['card', 'fitness'];
// Plain-HTTP services are reachable only from the campus network.
const CAMPUS_ONLY = ['library', 'venues'];
const vpnBlocked = (id: string) =>
  props.accessMode === 'webvpn' && !hasDirect.value && DIRECT_ONLY.includes(id);
const campusBlocked = (id: string) => props.accessMode === 'webvpn' && CAMPUS_ONLY.includes(id);
const available = (id: string) => READY.includes(id);
const badge = (id: string) => {
  if (campusBlocked(id)) return '需校园网';
  if (vpnBlocked(id)) return 'VPN 下不可用';
  if (available(id)) return '可用';
  const state = backend(id);
  if (!state) return '…';
  return state === 'pending' ? '待接入' : '界面待实现';
};

/** Direct-only tools log in on demand; a failed login explains itself. */
async function openDirect(id: string): Promise<boolean> {
  if (props.accessMode !== 'webvpn' || hasDirect.value || !DIRECT_ONLY.includes(id)) return true;
  try {
    await ensureDirectSession();
    directReady.value = true;
    emit('direct-session');
    return true;
  } catch (e) {
    console.error('[campus] direct login', id, e);
    error.value = e instanceof Error ? e.message : String(e);
    return false;
  }
}

function range(days: number): [string, string] {
  const end = new Date();
  const start = new Date(end);
  start.setDate(start.getDate() - days);
  const format = (date: Date) =>
    `${date.getFullYear()}-${String(date.getMonth() + 1).padStart(2, '0')}-${String(
      date.getDate(),
    ).padStart(2, '0')}`;
  return [format(start), format(end)];
}

async function open(id: string) {
  if (!available(id)) return;
  if (campusBlocked(id)) {
    error.value = '图书馆和场馆系统只支持校园网直连，请连接校园网后重试';
    return;
  }
  error.value = '';
  if (!(await openDirect(id))) return;
  tool.value = id;
  error.value = '';
  page.value = 1;
  loading.value = true;
  try {
    if (id === 'card') await loadCard();
    else if (id === 'grades')
      grades.value = (await bridge.schoolQuery({ type: 'grades' })) as Grades;
    else if (id === 'exams') exams.value = (await bridge.schoolQuery({ type: 'exams' })) as Exams;
    else if (id === 'attendance') await loadStreams();
    else if (id === 'calendar')
      calendar.value = (await bridge.schoolQuery({ type: 'calendar' })) as CalendarList;
  } catch (e) {
    console.error('[campus] open', tool.value, e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

/** Return to the tool list from a sub-view. */
function close() {
  tool.value = '';
  error.value = '';
  page.value = 1;
}

/** Calendar rows open their image in the campus window. */
function openCalendar(url: string) {
  void bridge.openUrl(url);
}

/** Double-clicking a row opens it, unless the click landed on a control. */
function rowDblClick(event: MouseEvent, action: () => void) {
  const target = event.target as HTMLElement | null;
  if (target?.closest('button, a, input, [role="switch"]')) return;
  action();
}

async function loadCard() {
  const [start, end] = range(30);
  const [card, list] = await Promise.all([
    directQuery<CardInfo>({ type: 'card' }),
    directQuery<CardTransactions>({ type: 'cardTransactions', start, end, page: page.value }),
  ]);
  info.value = card;
  transactions.value = list;
}

async function loadStreams() {
  streams.value = (await bridge.schoolQuery({
    type: 'attendanceStreams',
    page: page.value,
  })) as AttendanceStreams;
}

async function turn(delta: number) {
  page.value += delta;
  loading.value = true;
  error.value = '';
  try {
    if (tool.value === 'card') await loadCard();
    else if (tool.value === 'attendance') await loadStreams();
  } catch (e) {
    console.error('[campus] page', tool.value, e);
    error.value = e instanceof Error ? e.message : String(e);
    page.value -= delta;
  } finally {
    loading.value = false;
  }
}

const yuan = (cents: number) => (cents / 100).toFixed(2);
const signed = (cents: number) => `${cents < 0 ? '-' : '+'}${yuan(Math.abs(cents))}`;
const morePages = computed(() => {
  if (tool.value === 'card' && transactions.value)
    return transactions.value.page * 30 < transactions.value.total;
  if (tool.value === 'attendance' && streams.value)
    return streams.value.page * 50 < streams.value.total;
  return false;
});

onMounted(async () => {
  try {
    features.value = await bridge.schoolCatalog();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
});
</script>

<template>
  <section class="desktop-page">
    <PageHeader :title="title">
      <template v-if="tool" #title>
        <nav class="campus-breadcrumb" aria-label="当前位置">
          <button
            type="button"
            class="campus-breadcrumb-parent"
            :title="`返回${name}`"
            @click="close"
          >
            {{ name }}
          </button>
          <ChevronRight :size="14" :stroke-width="2" aria-hidden="true" />
          <span aria-current="page">{{ title }}</span>
        </nav>
      </template>
      <template v-if="tool" #actions>
        <button class="btn ghost small" @click="close">
          <ArrowLeft :size="14" aria-hidden="true" />返回
        </button>
      </template>
    </PageHeader>
    <template v-if="!tool">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
      <SettingGroup v-if="pinnedTools.length" title="常用">
        <SettingRow
          v-for="item in pinnedTools"
          :key="item.id"
          :title="item.name"
          :description="item.description"
          :class="{ actionable: available(item.id) }"
          @dblclick="rowDblClick($event, () => open(item.id))"
        >
          <template #leading
            ><IconTile :size="32" :tone="item.tone"
              ><component :is="item.icon" :size="16" /></IconTile
          ></template>
          <span class="badge" :class="{ accent: available(item.id) }">{{ badge(item.id) }}</span>
          <button
            class="btn ghost icon small"
            :title="`将${item.name}移回原位置`"
            :aria-label="`将${item.name}移回原位置`"
            @click="unpin(item.id)"
          >
            <ArrowDown :size="14" />
          </button>
          <button class="btn small tinted" :disabled="!available(item.id)" @click="open(item.id)">
            {{ available(item.id) ? '打开' : '未开放' }}
          </button>
        </SettingRow>
      </SettingGroup>
      <SettingGroup v-for="section in visibleSections" :key="section.title" :title="section.title">
        <SettingRow
          v-for="item in section.tools"
          :key="item.id"
          :title="item.name"
          :description="item.description"
          :class="{ actionable: available(item.id) }"
          @dblclick="rowDblClick($event, () => open(item.id))"
        >
          <template #leading
            ><IconTile :size="32" :tone="item.tone"
              ><component :is="item.icon" :size="16" /></IconTile
          ></template>
          <span class="badge" :class="{ accent: available(item.id) }">{{ badge(item.id) }}</span>
          <button
            class="btn ghost icon small"
            :title="`将${item.name}置顶到常用`"
            :aria-label="`将${item.name}置顶到常用`"
            @click="pin(item.id)"
          >
            <ArrowUp :size="14" />
          </button>
          <button class="btn small tinted" :disabled="!available(item.id)" @click="open(item.id)">
            {{ available(item.id) ? '打开' : '未开放' }}
          </button>
        </SettingRow>
      </SettingGroup>
    </template>

    <template v-else>
      <p v-if="error" class="notice" role="alert">{{ error }}</p>

      <template v-if="tool === 'card'">
        <SettingGroup title="校园卡">
          <SettingRow
            title="余额"
            :description="info ? `${info.name} · ${info.account}` : '正在读取…'"
          >
            <strong class="card-balance">{{ info ? `¥${yuan(info.balance)}` : '—' }}</strong>
          </SettingRow>
          <SettingRow
            v-if="info"
            title="状态"
            :description="`${info.cardType || '校园卡'} · 有效期 ${info.expireDate || '未知'}`"
          >
            <span class="badge">{{ info.lost ? '已挂失' : info.frozen ? '已冻结' : '正常' }}</span>
          </SettingRow>
        </SettingGroup>
        <SettingGroup title="最近 30 天流水">
          <EmptyState v-if="loading && !transactions" compact title="正在读取流水…" />
          <EmptyState
            v-else-if="!transactions?.rows.length"
            compact
            :icon="CreditCard"
            title="这段时间没有流水"
          />
          <template v-else>
            <div
              v-for="(row, index) in transactions.rows"
              :key="`${row.time}-${index}`"
              class="campus-record three"
            >
              <span class="campus-record-main">
                <strong>{{ row.merchant || row.type || '校园卡交易' }}</strong>
                <small
                  >{{ row.time
                  }}<template v-if="row.description"> · {{ row.description }}</template></small
                >
              </span>
              <span class="campus-record-amount" :class="{ income: row.amount > 0 }">
                {{ signed(row.amount) }}
              </span>
              <span class="campus-record-meta">¥{{ yuan(row.balance) }}</span>
            </div>
            <div class="campus-record-pages">
              <button class="btn small" :disabled="page <= 1 || loading" @click="turn(-1)">
                上一页
              </button>
              <span class="setting-value"
                >第 {{ transactions.page }} 页 · 共 {{ transactions.total }} 条</span
              >
              <button class="btn small" :disabled="loading || !morePages" @click="turn(1)">
                下一页
              </button>
            </div>
          </template>
        </SettingGroup>
      </template>

      <template v-else-if="tool === 'grades'">
        <SettingGroup title="成绩">
          <EmptyState v-if="loading && !grades" compact title="正在读取成绩…" />
          <EmptyState
            v-else-if="!grades?.rows.length"
            compact
            :icon="BarChart3"
            title="还没有成绩记录"
          />
          <div v-for="row in grades?.rows ?? []" :key="row.id" class="campus-record three">
            <span class="campus-record-main">
              <strong>{{ row.name }}</strong>
              <small
                >{{ row.term }}<template v-if="row.category"> · {{ row.category }}</template></small
              >
            </span>
            <span class="campus-record-amount" :class="{ failed: row.passed === false }">
              {{ row.score || '—' }}
            </span>
            <span class="campus-record-meta"
              >{{ row.credits ?? '—' }} 学分<template v-if="row.gpa != null">
                · 绩点 {{ row.gpa }}</template
              ></span
            >
          </div>
        </SettingGroup>
      </template>

      <template v-else-if="tool === 'exams'">
        <SettingGroup :title="exams?.term ? `考试安排 · ${exams.term}` : '考试安排'">
          <EmptyState v-if="loading && !exams" compact title="正在读取考试安排…" />
          <EmptyState
            v-else-if="!exams?.rows.length"
            compact
            :icon="ClipboardList"
            title="还没有考试安排"
          />
          <div v-for="(row, index) in exams?.rows ?? []" :key="index" class="campus-record three">
            <span class="campus-record-main">
              <strong>{{ row.course }}</strong>
              <small>{{ [row.date, row.time, row.campus].filter(Boolean).join(' · ') }}</small>
            </span>
            <span class="campus-record-meta"
              >{{ row.location || '考场未知'
              }}<template v-if="row.seat"> · {{ row.seat }} 号</template></span
            >
          </div>
        </SettingGroup>
      </template>

      <template v-else-if="tool === 'attendance'">
        <SettingGroup title="刷卡流水">
          <EmptyState v-if="loading && !streams" compact title="正在读取考勤流水…" />
          <EmptyState
            v-else-if="!streams?.rows.length"
            compact
            :icon="CalendarCheck"
            title="还没有考勤流水"
          />
          <template v-else>
            <div v-for="row in streams.rows" :key="row.id" class="campus-record two">
              <span class="campus-record-main">
                <strong>{{ row.place || '未知地点' }}</strong>
                <small>{{ row.time }}</small>
              </span>
              <span class="badge" :class="{ accent: row.valid }">{{
                row.valid ? '有效' : '无效'
              }}</span>
            </div>
            <div class="campus-record-pages">
              <button class="btn small" :disabled="page <= 1 || loading" @click="turn(-1)">
                上一页
              </button>
              <span class="setting-value"
                >第 {{ streams.page }} 页 · 共 {{ streams.total }} 条</span
              >
              <button class="btn small" :disabled="loading || !morePages" @click="turn(1)">
                下一页
              </button>
            </div>
          </template>
        </SettingGroup>
      </template>

      <CampusRooms v-else-if="tool === 'rooms'" />
      <CampusCourses v-else-if="tool === 'courses'" />
      <CampusEvaluations v-else-if="tool === 'evaluations'" />
      <CampusFitness v-else-if="tool === 'fitness'" />
      <CampusProfile v-else-if="tool === 'profile'" />
      <CampusLms v-else-if="tool === 'lms'" />
      <CampusLibrary v-else-if="tool === 'library'" />
      <CampusVenue v-else-if="tool === 'venues'" />

      <template v-else-if="tool === 'calendar'">
        <SettingGroup title="校历">
          <EmptyState v-if="loading && !calendar" compact title="正在读取校历…" />
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
            @dblclick="rowDblClick($event, () => openCalendar(row.url))"
          >
            <button class="btn small tinted" @click="openCalendar(row.url)">打开</button>
          </SettingRow>
        </SettingGroup>
      </template>
    </template>
  </section>
</template>

<style scoped>
.campus-breadcrumb {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  min-width: 0;
}
.campus-breadcrumb-parent {
  padding: 0;
  border: 0;
  background: none;
  color: var(--text-secondary);
  font: inherit;
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: color var(--duration-fast) ease;
}
.campus-breadcrumb-parent:hover {
  color: var(--accent);
}
.campus-breadcrumb svg {
  flex-shrink: 0;
  color: var(--text-tertiary);
}
.card-balance {
  font-size: 18px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
</style>
