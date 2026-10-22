<script setup lang="ts">
import { computed, onMounted, ref, type Component } from 'vue';
import {
  Activity,
  ArrowLeft,
  BarChart3,
  BookMarked,
  BookOpen,
  BookOpenText,
  CalendarCheck,
  CalendarDays,
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
import { bridge } from '../bridge';
import type { CardInfo, CardTransactions, SchoolFeature } from '../types';

defineProps<{ name: string }>();

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
      { id: 'library', name: '图书馆', description: '借阅与座位', icon: BookMarked, tone: 'teal' },
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
        description: '场馆查询与预约',
        icon: MapPin,
        tone: 'violet',
      },
      { id: 'profile', name: '学籍档案', description: '个人学籍信息', icon: IdCard, tone: 'gray' },
      { id: 'lms', name: '思源学堂', description: '课程资料与作业', icon: BookOpen, tone: 'blue' },
    ],
  },
];

const features = ref<SchoolFeature[]>([]);
const tool = ref('');
const error = ref('');
const loading = ref(false);
const info = ref<CardInfo | null>(null);
const transactions = ref<CardTransactions | null>(null);
const page = ref(1);

const backend = (id: string) => features.value.find((feature) => feature.id === id)?.backend;
const available = (id: string) => id === 'card';
const badge = (id: string) => {
  if (id === 'card') return '可用';
  const state = backend(id);
  if (!state) return '…';
  return state === 'pending' ? '待接入' : '界面待实现';
};

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
  tool.value = id;
  error.value = '';
  if (id === 'card') {
    page.value = 1;
    await loadCard();
  }
}

async function loadCard() {
  loading.value = true;
  error.value = '';
  try {
    const [start, end] = range(30);
    const [card, list] = await Promise.all([
      bridge.schoolQuery({ type: 'card' }),
      bridge.schoolQuery({ type: 'cardTransactions', start, end, page: page.value }),
    ]);
    info.value = card as CardInfo;
    transactions.value = list as CardTransactions;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function turn(delta: number) {
  page.value += delta;
  loading.value = true;
  error.value = '';
  try {
    const [start, end] = range(30);
    transactions.value = (await bridge.schoolQuery({
      type: 'cardTransactions',
      start,
      end,
      page: page.value,
    })) as CardTransactions;
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
    page.value -= delta;
  } finally {
    loading.value = false;
  }
}

const yuan = (cents: number) => (cents / 100).toFixed(2);
const signed = (cents: number) => `${cents < 0 ? '-' : '+'}${yuan(Math.abs(cents))}`;
const morePages = computed(
  () => (transactions.value?.page ?? 1) * 30 < (transactions.value?.total ?? 0),
);

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
    <PageHeader :title="name" />
    <template v-if="!tool">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
      <SettingGroup v-for="section in SECTIONS" :key="section.title" :title="section.title">
        <SettingRow
          v-for="item in section.tools"
          :key="item.id"
          :title="item.name"
          :description="item.description"
        >
          <template #leading
            ><IconTile :size="32" :tone="item.tone"
              ><component :is="item.icon" :size="16" /></IconTile
          ></template>
          <span class="badge" :class="{ accent: available(item.id) }">{{ badge(item.id) }}</span>
          <button class="btn small" :disabled="!available(item.id)" @click="open(item.id)">
            {{ available(item.id) ? '打开' : '未开放' }}
          </button>
        </SettingRow>
      </SettingGroup>
    </template>

    <template v-else>
      <button class="back-button" @click="tool = ''">
        <ArrowLeft :size="14" /><span>{{ name }}</span>
      </button>
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
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
            class="tx-row"
          >
            <span class="tx-main">
              <strong>{{ row.merchant || row.type || '校园卡交易' }}</strong>
              <small
                >{{ row.time
                }}<template v-if="row.description"> · {{ row.description }}</template></small
              >
            </span>
            <span class="tx-amount" :class="{ income: row.amount > 0 }">
              {{ signed(row.amount) }}
            </span>
            <span class="tx-balance">¥{{ yuan(row.balance) }}</span>
          </div>
          <div class="tx-pages">
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
  </section>
</template>

<style scoped>
.card-balance {
  font-size: 18px;
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.tx-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) auto auto;
  align-items: center;
  gap: 14px;
  padding: 10px 16px;
  border-top: 1px solid var(--divider);
}
.tx-main {
  display: flex;
  flex-direction: column;
  min-width: 0;
}
.tx-main strong {
  font-size: 13px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tx-main small {
  margin-top: 2px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tx-amount {
  color: var(--text);
  font-size: 13px;
  font-variant-numeric: tabular-nums;
}
.tx-amount.income {
  color: var(--success);
}
.tx-balance {
  color: var(--text-tertiary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.tx-pages {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 10px;
  padding: 10px 16px;
  border-top: 1px solid var(--divider);
}
</style>
