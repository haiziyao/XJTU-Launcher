<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import {
  AlertCircle,
  BookMarked,
  BookOpen,
  Building2,
  CalendarDays,
  CalendarCheck,
  Compass,
  GraduationCap,
  LoaderCircle,
  MapPin,
  RefreshCw,
  ScrollText,
} from 'lucide-vue-next';
import { campusLinks } from '../domain';
import type { AppConfig, CampusNotice, CampusNoticeBoard, Course, Plugin } from '../types';
import UiCard from './ui/UiCard.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile, { type Tone } from './ui/IconTile.vue';
import UiSwitch from './ui/UiSwitch.vue';
import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
const props = defineProps<{
  config: AppConfig;
  plugins: Plugin[];
  connected: boolean;
  displayName: string;
  now: Date;
  hasTimetable: boolean;
  timetableEnabled: boolean;
  courses: Course[];
  notices: CampusNoticeBoard | null;
  noticesLoading: boolean;
  noticesError: string;
  loading: boolean;
  error: string;
  saving: boolean;
}>();
const emit = defineEmits<{
  link: [url: string];
  refresh: [];
  refreshNotices: [];
  toggle: [id: string];
  account: [];
  open: [id: string];
}>();
const date = computed(() =>
  new Intl.DateTimeFormat('zh-CN', { month: 'long', day: 'numeric', weekday: 'long' }).format(
    props.now,
  ),
);
const time = computed(() =>
  new Intl.DateTimeFormat('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
    second: props.config.showSeconds ? '2-digit' : undefined,
    hour12: false,
  }).format(props.now),
);
const greeting = computed(() => {
  const hour = props.now.getHours();
  const part =
    hour < 5
      ? '夜深了'
      : hour < 11
        ? '早上好'
        : hour < 13
          ? '中午好'
          : hour < 18
            ? '下午好'
            : '晚上好';
  return props.displayName ? `${part}，${props.displayName}` : part;
});
const shortcutStyle: { icon: typeof Building2; tone: Tone }[] = [
  { icon: Building2, tone: 'blue' },
  { icon: GraduationCap, tone: 'violet' },
  { icon: BookOpen, tone: 'teal' },
  { icon: BookMarked, tone: 'green' },
];
const sections = computed(() => new Set(props.config.homeSections));
const showGreeting = computed(() => sections.value.has('greeting'));
const showClock = computed(() => sections.value.has('clock'));
const showHero = computed(() => showGreeting.value || showClock.value);
const showCourses = computed(() => sections.value.has('timetable') && props.timetableEnabled);
const showLinks = computed(() => sections.value.has('links'));
const showNotices = computed(() => sections.value.has('notices'));
const showPlugins = computed(() => sections.value.has('plugins'));
const noticeFilter = ref('all');
const noticeCategories = computed(() => {
  const counts = new Map<string, number>();
  for (const item of props.notices?.items ?? []) {
    const name = item.category?.trim() || '其他';
    counts.set(name, (counts.get(name) ?? 0) + 1);
  }
  return [
    { id: 'all', name: '全部', count: props.notices?.items.length ?? 0 },
    ...[...counts].map(([name, count]) => ({ id: name, name, count })),
  ];
});
const filteredNotices = computed<CampusNotice[]>(() => {
  const items = props.notices?.items ?? [];
  if (noticeFilter.value === 'all') return items;
  return items.filter((item) => (item.category?.trim() || '其他') === noticeFilter.value);
});
function noticeDate(value: string) {
  return /^\d{4}-\d{2}-\d{2}$/.test(value) ? value.slice(5) : value;
}
watch(
  () => props.notices?.items,
  () => {
    if (!noticeCategories.value.some((entry) => entry.id === noticeFilter.value)) {
      noticeFilter.value = 'all';
    }
  },
);
// Each widget keeps its configured order; course and links share a row when both are shown.
const order = (widget: string) => props.config.widgets.indexOf(widget);
const paired = computed(() => showCourses.value && showLinks.value);
const splitNotices = computed(() => showNotices.value && (showCourses.value || showLinks.value));
</script>

<template>
  <div class="home">
    <header v-if="showHero" class="home-hero">
      <div v-if="showGreeting">
        <p class="home-date">{{ date }}</p>
        <h1>{{ greeting }}</h1>
      </div>
      <time
        v-if="showClock"
        class="home-clock"
        :class="config.homeClock"
        :datetime="now.toISOString()"
        >{{ time }}</time
      >
    </header>

    <div class="home-grid" :class="{ paired, 'split-notices': splitNotices }">
      <div class="home-left">
        <UiCard
          v-if="showCourses"
          title="今日课程"
          :subtitle="hasTimetable && courses.length ? `${courses.length} 节` : undefined"
          class="home-courses"
          :style="{ order: order('timetable') }"
        >
          <template v-if="connected && hasTimetable" #actions>
            <UiIconButton
              label="刷新课表"
              size="sm"
              title="刷新课表"
              :disabled="loading"
              @click="emit('refresh')"
            >
              <RefreshCw :size="14" :class="{ spinning: loading }" />
            </UiIconButton>
          </template>
          <Transition name="fade" mode="out-in">
            <EmptyState
              v-if="!hasTimetable"
              key="empty"
              compact
              :icon="loading ? LoaderCircle : CalendarDays"
              :busy="loading"
              :tone="error ? 'danger' : 'default'"
              :title="loading ? '正在同步课表…' : connected ? '尚未同步课表' : '未登录校园账号'"
              :description="
                error || (connected ? '从校园系统获取最新课表' : '登录后即可在这里查看今日课程')
              "
            >
              <UiButton
                variant="primary"
                size="sm"
                v-if="connected"
                :disabled="loading"
                @click="emit('refresh')"
              >
                同步课表
              </UiButton>
              <UiButton size="sm" v-else @click="emit('account')">前往校园账号</UiButton>
            </EmptyState>
            <EmptyState
              v-else-if="!courses.length"
              key="free"
              compact
              tone="success"
              :icon="CalendarCheck"
              title="今天没有课程"
              :description="error || '好好安排属于自己的一天'"
            />
            <ol v-else key="list" class="course-list">
              <li v-for="(course, i) in courses" :key="`${course.name}-${i}`" class="course-item">
                <span class="course-period">
                  <strong>{{ course.start }}</strong
                  ><span v-if="course.end !== course.start">–{{ course.end }}</span>
                  <small>节</small>
                </span>
                <span class="course-bar" aria-hidden="true"></span>
                <div class="course-copy">
                  <h3>{{ course.name }}</h3>
                  <p>
                    <MapPin :size="12" />{{ course.location || '地点待定'
                    }}<span v-if="course.teacher" class="course-teacher">{{ course.teacher }}</span>
                  </p>
                </div>
              </li>
              <li v-if="error" class="course-error" role="alert">{{ error }}</li>
            </ol>
          </Transition>
        </UiCard>

        <UiCard
          v-if="showLinks"
          title="校园快捷入口"
          class="home-links"
          :style="{ order: order('links') }"
        >
          <div class="shortcut-grid">
            <button
              v-for="(link, i) in campusLinks"
              :key="link.url"
              class="shortcut"
              :title="`${link.title} · 打开校园网页`"
              @click="emit('link', link.url)"
            >
              <IconTile :tone="shortcutStyle[i % shortcutStyle.length].tone" :size="38">
                <component :is="shortcutStyle[i % shortcutStyle.length].icon" :size="19" />
              </IconTile>
              <span class="shortcut-copy">
                <strong>{{ link.title }}</strong>
                <small>{{ link.description }}</small>
              </span>
            </button>
          </div>
        </UiCard>
      </div>

      <UiCard
        v-if="showNotices"
        title="教务处通知"
        :subtitle="notices?.name || undefined"
        class="home-notices"
        :style="{ order: order('notices') }"
      >
        <template #actions>
          <UiIconButton
            label="刷新通知"
            size="sm"
            title="刷新通知"
            :disabled="noticesLoading"
            @click="emit('refreshNotices')"
          >
            <RefreshCw :size="14" :class="{ spinning: noticesLoading }" />
          </UiIconButton>
        </template>
        <Transition name="fade" mode="out-in">
          <div v-if="notices?.items.length" key="list" class="notice-panel">
            <div class="notice-filter" role="tablist" aria-label="通知分类">
              <button
                v-for="category in noticeCategories"
                :key="category.id"
                type="button"
                role="tab"
                :aria-selected="noticeFilter === category.id"
                :class="{ active: noticeFilter === category.id }"
                @click="noticeFilter = category.id"
              >
                {{ category.name }}<span>{{ category.count }}</span>
              </button>
            </div>
            <div class="notice-scroll" :class="{ scrollable: filteredNotices.length > 6 }">
              <button
                v-for="item in filteredNotices"
                :key="item.url"
                class="notice-item"
                :title="`${item.title}${item.date ? `（${item.date}）` : ''}`"
                @click="emit('link', item.url)"
              >
                <span class="notice-date">{{ noticeDate(item.date) }}</span>
                <span class="notice-body">
                  <span class="notice-title">{{ item.title }}</span>
                  <span v-if="noticeFilter === 'all' && item.category" class="notice-tag">{{
                    item.category
                  }}</span>
                </span>
              </button>
            </div>
          </div>
          <EmptyState
            v-else
            key="empty"
            compact
            :icon="noticesError ? AlertCircle : ScrollText"
            :busy="noticesLoading"
            :tone="noticesError ? 'danger' : 'default'"
            :title="
              noticesLoading ? '正在读取通知…' : noticesError ? '通知暂时无法读取' : '暂无通知'
            "
            :description="noticesError || '来自教务处教学通知栏目'"
          >
            <UiButton size="sm" v-if="!noticesLoading" @click="emit('refreshNotices')"
              >重试</UiButton
            >
          </EmptyState>
        </Transition>
      </UiCard>

      <UiCard
        v-if="showPlugins"
        title="我的插件"
        :subtitle="plugins.length ? `${plugins.length} 个` : undefined"
        class="home-plugins"
        :style="{ order: order('plugins') }"
      >
        <div v-if="plugins.length" class="plugin-grid">
          <div
            v-for="plugin in plugins"
            :key="plugin.id"
            class="plugin-tile"
            :class="{ disabled: config.disabledPlugins.includes(plugin.id) }"
          >
            <button
              class="plugin-tile-main"
              :disabled="config.disabledPlugins.includes(plugin.id)"
              :title="`打开${plugin.name}`"
              @click="emit('open', plugin.id)"
            >
              <IconTile :tone="plugin.page.type === 'timetable' ? 'blue' : 'green'" :size="34">
                <CalendarDays v-if="plugin.page.type === 'timetable'" :size="17" /><Compass
                  v-else
                  :size="17"
                />
              </IconTile>
              <span class="plugin-tile-copy">
                <strong>{{ plugin.name }}</strong>
                <small>{{ plugin.description }}</small>
              </span>
            </button>
            <UiSwitch
              :checked="!config.disabledPlugins.includes(plugin.id)"
              :label="`${plugin.name}启用`"
              :disabled="saving"
              @toggle="emit('toggle', plugin.id)"
            />
          </div>
        </div>
        <EmptyState v-else compact :icon="Compass" title="暂无已安装插件" />
      </UiCard>
    </div>

    <EmptyState
      v-if="!showCourses && !showLinks && !showNotices && !showPlugins"
      :icon="Compass"
      title="主页暂无组件"
      description="在“主页主题”插件中打开需要的元素。"
    />
  </div>
</template>

<style scoped>
.home-hero {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 24px;
  margin: 4px 0 24px;
}
.home-date {
  color: var(--text-tertiary);
  font-size: 13px;
  font-weight: 500;
}
.home-hero h1 {
  margin-top: 4px;
  font-size: 24px;
  font-weight: 600;
  letter-spacing: -0.015em;
  line-height: 1.3;
}
.home-clock {
  margin-left: auto;
  color: var(--text-secondary);
  font-size: 22px;
  font-weight: 300;
  letter-spacing: 0.01em;
  font-variant-numeric: tabular-nums;
  line-height: 1.3;
}
.home-clock.bold {
  font-size: 34px;
  font-weight: 650;
  letter-spacing: -0.02em;
}
.home-clock.mono {
  font-family: var(--font-mono);
  font-size: 23px;
  font-weight: 400;
  letter-spacing: 0;
}
.home-clock.card {
  padding: 6px 14px;
  border: 1px solid var(--panel-border, var(--border));
  border-radius: 12px;
  background: var(--panel, var(--surface-subtle));
  font-family: var(--font-mono);
  font-size: 20px;
  font-weight: 500;
}

.home-grid {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  gap: 16px;
}
.home-grid.paired {
  grid-template-columns: minmax(0, 1.15fr) minmax(0, 1fr);
}
/* On wide screens the notices become a right-hand feed beside the courses and
   shortcuts; the left cards stay stacked in their own column. */
.home-left {
  display: contents;
}
.home-grid.split-notices {
  grid-template-columns: minmax(0, 1.3fr) minmax(0, 1fr);
  align-items: start;
}
.home-grid.split-notices .home-left {
  display: flex;
  flex-direction: column;
  gap: 16px;
  min-width: 0;
}
.home-grid.split-notices .home-notices {
  grid-column: 2;
  display: flex;
  flex-direction: column;
  align-self: stretch;
  /* Match the stacked cards on the left instead of running down the page. */
  max-height: 620px;
}
.home-grid.split-notices .notice-panel {
  display: flex;
  flex: 1;
  flex-direction: column;
  min-height: 0;
}
.home-grid.split-notices .notice-scroll {
  flex: 1;
  min-height: 0;
}
.home-plugins {
  grid-column: 1 / -1;
}

/* Courses */
.home-courses :deep(.empty-state) {
  min-height: 228px;
}
.course-list {
  list-style: none;
  margin: 0;
  padding: 4px 8px 10px;
  max-height: 340px;
  overflow-y: auto;
}
.course-item {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px;
  border-radius: var(--radius);
  transition: background var(--duration-fast) ease;
}
.course-item:hover {
  background: var(--accent-wash);
}
.course-period {
  display: flex;
  align-items: baseline;
  justify-content: flex-end;
  width: 46px;
  flex-shrink: 0;
  color: var(--text);
  font-variant-numeric: tabular-nums;
}
.course-period strong {
  font-size: 16px;
  font-weight: 600;
}
.course-period span {
  font-size: 13px;
  color: var(--text-secondary);
}
.course-period small {
  margin-left: 2px;
  color: var(--text-tertiary);
  font-size: 11px;
}
.course-bar {
  align-self: stretch;
  width: 3px;
  border-radius: 2px;
  background: var(--accent);
  opacity: 0.75;
}
.course-copy {
  min-width: 0;
}
.course-copy h3 {
  font-size: 13.5px;
  font-weight: 550;
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}
.course-copy p {
  display: flex;
  align-items: center;
  gap: 4px;
  margin-top: 3px;
  color: var(--text-tertiary);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
}
.course-teacher::before {
  content: '·';
  margin: 0 5px 0 3px;
}
.course-error {
  padding: 8px 10px 0;
  color: var(--danger);
  font-size: 12px;
  line-height: 1.6;
  overflow-wrap: anywhere;
}

/* Shortcuts */
.shortcut-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 4px;
  padding: 6px 8px 10px;
}
.home-grid:not(.paired) .shortcut-grid {
  grid-template-columns: repeat(4, minmax(0, 1fr));
}
.shortcut {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 0;
  padding: 12px 10px;
  border: 0;
  border-radius: var(--radius);
  background: transparent;
  text-align: left;
  transition:
    background var(--duration-fast) ease,
    transform var(--duration-fast) var(--ease-out);
}
.shortcut:hover {
  background: var(--accent-wash);
}
.shortcut:active {
  background: var(--surface-sunken);
  transform: scale(0.98);
}
.shortcut .icon-tile {
  transition: transform var(--duration) var(--ease-out);
}
.shortcut:hover .icon-tile {
  transform: translateY(-1px);
}
.shortcut-copy {
  min-width: 0;
}
.shortcut-copy strong,
.plugin-tile-copy strong {
  display: block;
  font-size: 13px;
  font-weight: 550;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.shortcut-copy small,
.plugin-tile-copy small {
  display: block;
  margin-top: 2px;
  color: var(--text-tertiary);
  font-size: 12px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

/* Notices */
.home-notices {
  grid-column: 1 / -1;
}
.notice-panel {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 0 10px;
}
.notice-filter {
  display: flex;
  gap: 6px;
  overflow-x: auto;
  padding: 0 4px 2px;
  scrollbar-width: none;
  -webkit-mask-image: linear-gradient(to right, #000 calc(100% - 14px), transparent);
  mask-image: linear-gradient(to right, #000 calc(100% - 14px), transparent);
}
.notice-filter::-webkit-scrollbar {
  display: none;
}
.notice-filter button {
  flex: none;
  border: 0;
  border-radius: 999px;
  background: var(--surface-sunken);
  color: var(--text-secondary);
  font: inherit;
  font-size: 11.5px;
  padding: 4px 10px;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    color var(--duration-fast) ease;
}
.notice-filter button span {
  margin-left: 6px;
  opacity: 0.65;
  font-size: 10.5px;
}
.notice-filter button:hover:not(.active) {
  background: var(--surface-hover);
  color: var(--text);
}
.notice-filter button.active {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.notice-scroll {
  display: grid;
  grid-template-columns: minmax(0, 1fr);
  max-height: 340px;
  overflow-y: auto;
  margin: 0;
  padding: 2px 4px 10px;
  scrollbar-width: thin;
}
.notice-scroll.scrollable {
  -webkit-mask-image: linear-gradient(to bottom, #000 calc(100% - 16px), transparent);
  mask-image: linear-gradient(to bottom, #000 calc(100% - 16px), transparent);
}
.notice-item {
  display: flex;
  align-items: flex-start;
  gap: 14px;
  width: 100%;
  padding: 11px 10px;
  border: 0;
  border-bottom: 1px solid var(--divider);
  border-radius: var(--radius-sm);
  background: transparent;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.notice-item:hover {
  background: var(--surface-hover);
}
.notice-date {
  flex: none;
  width: 38px;
  padding-top: 2px;
  color: var(--text-tertiary);
  font-size: 11px;
  font-variant-numeric: tabular-nums;
}
.notice-body {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
}
.notice-title {
  color: var(--text);
  font-size: 12.5px;
  font-weight: 500;
  line-height: 1.6;
  overflow-wrap: anywhere;
}
.notice-tag {
  color: var(--text-tertiary);
  font-size: 10.5px;
}

/* Plugins */
.plugin-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
  gap: 4px;
  padding: 6px 8px 10px;
}
.plugin-tile {
  display: flex;
  align-items: center;
  gap: 10px;
  padding-right: 10px;
  border-radius: var(--radius);
  transition: background var(--duration-fast) ease;
}
.plugin-tile:hover {
  background: var(--surface-subtle);
}
.plugin-tile-main {
  display: flex;
  align-items: center;
  gap: 12px;
  flex: 1;
  min-width: 0;
  padding: 10px;
  border: 0;
  background: transparent;
  text-align: left;
}
.plugin-tile-main:disabled {
  opacity: 1;
}
.plugin-tile-copy {
  min-width: 0;
}
.plugin-tile .icon-tile,
.plugin-tile-copy {
  transition:
    opacity var(--duration) ease,
    filter var(--duration) ease;
}
.plugin-tile.disabled .icon-tile {
  filter: grayscale(1);
  opacity: 0.55;
}
.plugin-tile.disabled .plugin-tile-copy {
  opacity: 0.55;
}

@media (max-width: 1020px) {
  .home-grid.paired,
  .home-grid.split-notices {
    grid-template-columns: minmax(0, 1fr);
  }
  .home-grid.split-notices .home-left {
    display: contents;
  }
  .home-grid.split-notices .home-notices {
    grid-column: 1 / -1;
  }
  .home-grid.paired .shortcut-grid {
    grid-template-columns: repeat(4, minmax(0, 1fr));
  }
}
@media (max-width: 760px) {
  .home-grid .shortcut-grid,
  .home-grid.paired .shortcut-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}
</style>
