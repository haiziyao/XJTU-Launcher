<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookOpen, ExternalLink, PlayCircle, Presentation, Star } from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import SectionHeader from '../ui/SectionHeader.vue';
import SplitView from '../ui/SplitView.vue';
import ListItem from '../ui/ListItem.vue';
import SegmentedTabs from '../ui/SegmentedTabs.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import StatusBadge, { type StatusTone } from '../ui/StatusBadge.vue';
import DueLabel from '../ui/DueLabel.vue';
import Skeleton from '../ui/Skeleton.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import MoreMenu, { type MoreItem } from '../ui/MoreMenu.vue';
import FileRow from './FileRow.vue';
import UiButton from '../ui/UiButton.vue';
import UiIconButton from '../ui/UiIconButton.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import { applyCached, cacheName } from '../../cache';
import { campusDateKey, campusTimestamp, dueInfo, fileSize, lmsTime } from '../../domain';
import type {
  LmsActivities,
  LmsActivity,
  LmsActivityDetail,
  LmsCourse,
  LmsCourses,
  LmsSubmission,
  LmsSubmissions,
  LmsUpload,
} from '../../types';

/**
 * 思源学堂 (LAYOUT §9.3): course → categorised activities → activity detail.
 * The list/detail split keeps the course picker in the header so the list can
 * stay on activities. Submitting homework is not implemented in the app; the
 * page says so and points at the campus website.
 */
const emit = defineEmits<{
  capture: [target: { activity: number; course: number | null; title: string }];
}>();

const TYPE_LABEL: Record<string, string> = {
  homework: '作业',
  material: '资料',
  lesson: '课程回放',
  online_video: '在线视频',
  lecture_live: '直播课堂',
  forum: '讨论',
  questionnaire: '问卷',
  web_link: '网页链接',
  page: '页面',
};
const CATEGORIES = [
  { id: 'homework', title: '作业', types: ['homework'] },
  { id: 'material', title: '学习资料', types: ['material'] },
  { id: 'lesson', title: '课程回放', types: ['lesson'] },
  { id: 'online_video', title: '在线视频', types: ['online_video'] },
  { id: 'lecture_live', title: '直播课堂', types: ['lecture_live'] },
  { id: 'forum', title: '讨论', types: ['forum'] },
  { id: 'questionnaire', title: '问卷', types: ['questionnaire'] },
];
const TERM_RANK: Record<string, number> = { 第一学期: 1, 第二学期: 2, 第三学期: 3 };
const courses = ref<LmsCourse[]>([]);
const course = ref<LmsCourse | null>(null);
const activeSemester = ref('');
const favoriteIds = ref<number[]>([]);
const lastCourseId = ref<number | null>(null);
let favoriteKey = 'campus.lms.favorites:local';
let lastCourseKey = 'campus.lms.course:local';
const activities = ref<LmsActivity[]>([]);
const activity = ref<LmsActivity | null>(null);
const detail = ref<LmsActivityDetail | null>(null);
const submissions = ref<LmsSubmissions | null>(null);
const active = ref('');
const listWidth = ref(320);
const loadingCourses = ref(false);
const loadingActivities = ref(false);
const loadingDetail = ref(false);
const loadingSubmissions = ref(false);
const downloading = ref(0);
const loadedAt = ref<number | null>(null);
const error = ref('');
const notice = ref('');
const moreItems: MoreItem[] = [{ id: 'lms', label: '在思源学堂中打开', icon: ExternalLink }];

function semesterKey(item: LmsCourse) {
  const label = item.termLabel.trim();
  if (label) return label;
  return [item.year.trim(), item.term.trim()].filter(Boolean).join(' · ') || '未标注学期';
}

/** 秋季 = 1、春季 = 2、夏季 = 3，跟着学年一起倒序。 */
function semesterRank(item: LmsCourse) {
  const label = semesterKey(item);
  const start = Number(label.match(/(\d{4})-\d{4}/)?.[1]);
  const ordinal = label.includes('秋')
    ? 1
    : label.includes('春')
      ? 2
      : label.includes('夏')
        ? 3
        : TERM_RANK[item.term.trim()] || 0;
  return Number.isFinite(start) ? start * 10 + ordinal : -Infinity;
}

const semesters = computed(() => {
  const groups = new Map<string, { id: string; title: string; rank: number; items: LmsCourse[] }>();
  for (const item of courses.value) {
    const title = semesterKey(item);
    let group = groups.get(title);
    if (!group) {
      group = { id: title, title, rank: semesterRank(item), items: [] };
      groups.set(title, group);
    }
    group.items.push(item);
  }
  return [...groups.values()].sort((a, b) => b.rank - a.rank);
});
// The semester list only contains semesters that actually have courses, so a
// term picker never leads to an empty page.
const semesterOptions = computed(() =>
  semesters.value.map((semester) => ({ value: semester.id, label: semester.title })),
);
const activeSemesterGroup = computed(
  () =>
    semesters.value.find((semester) => semester.id === activeSemester.value) ??
    semesters.value[0] ??
    null,
);
const courseOptions = computed(() =>
  (activeSemesterGroup.value?.items ?? []).map((item) => ({
    value: String(item.id),
    label: item.name,
  })),
);
const favoriteCourses = computed(() =>
  courses.value.filter((item) => favoriteIds.value.includes(item.id)),
);
const isFavorite = computed(() =>
  Boolean(course.value && favoriteIds.value.includes(course.value.id)),
);
const categories = computed(() => {
  const known = new Set(CATEGORIES.flatMap((category) => category.types));
  const result = CATEGORIES.map((category) => ({
    id: category.id,
    title: category.title,
    items: activities.value.filter((item) => category.types.includes(item.type)),
  })).filter((group) => group.items.length);
  const other = activities.value.filter((item) => !known.has(item.type));
  if (other.length) result.push({ id: 'other', title: '其他活动', items: other });
  return result;
});
const tabs = computed(() =>
  categories.value.map((group) => ({
    id: group.id,
    title: `${group.title} ${group.items.length}`,
  })),
);
const activeGroup = computed(
  () => categories.value.find((group) => group.id === active.value) || categories.value[0],
);
const subtitle = computed(
  () =>
    [course.value?.name, activeSemesterGroup.value?.title].filter(Boolean).join(' · ') ||
    '课程与活动',
);
const detailType = computed(() => (detail.value ? typeName(detail.value.type) : ''));
const dueAt = computed(() => (detail.value?.end ? campusTimestamp(detail.value.end) : null));

const short = lmsTime;
const span = (start: string, end: string) =>
  [short(start), short(end)].filter(Boolean).join(' – ') || '时间未公布';
const typeName = (type: string) => TYPE_LABEL[type] || '活动';
const meta = (item: LmsActivity) => {
  if (item.type === 'homework') return item.end ? `截止 ${short(item.end)}` : '无截止时间';
  if (item.type === 'lesson' || item.type === 'lecture_live') return span(item.start, item.end);
  return item.start ? short(item.start) : '';
};
const homeworkTone = (item: LmsActivity): { tone: StatusTone; label: string } | null => {
  if (item.type !== 'homework' || !item.end) return null;
  const info = dueInfo(campusTimestamp(item.end));
  if (info.overdue) return { tone: 'urgent', label: '已截止' };
  if (info.tone === 'warning') return { tone: 'near', label: '临近截止' };
  return null;
};
const statusTone = computed<StatusTone>(() => {
  const value = detail.value;
  if (!value) return 'neutral';
  if (value.type === 'homework') {
    if ((value.submitCount ?? 0) > 0) return 'done';
    if (value.closed) return 'neutral';
    const info = dueInfo(dueAt.value);
    if (info.overdue) return 'urgent';
    if (info.tone === 'warning') return 'near';
    return 'active';
  }
  return value.published ? 'active' : 'neutral';
});
const statusLabel = computed(() => {
  const value = detail.value;
  if (!value) return '';
  if (value.type === 'homework') {
    if ((value.submitCount ?? 0) > 0) return '已提交';
    if (value.closed) return '已截止';
    if (dueInfo(dueAt.value).overdue) return '已截止';
    return value.resubmitOpen ? '可重交' : '进行中';
  }
  return value.published ? typeName(value.type) : '未发布';
});
const timeRow = computed(() => {
  const value = detail.value;
  if (!value) return '';
  if (value.type === 'homework') return value.end ? short(value.end) : '无截止时间';
  if (value.type === 'lesson') {
    return span(value.lessonStart || value.start, value.lessonEnd || value.end);
  }
  if (value.type === 'lecture_live') {
    return [value.liveName, span(value.liveStart || value.start, value.liveEnd || value.end)]
      .filter(Boolean)
      .join(' · ');
  }
  return span(value.start, value.end);
});
const submissionLine = computed(() => {
  const value = detail.value;
  if (!value || value.type !== 'homework') return '';
  const count = value.submitCount == null ? '—' : value.submitCount;
  const allowed = value.submitTimes == null ? '' : ` / 可提交 ${value.submitTimes} 次`;
  return `已提交 ${count} 次${allowed}${value.submitByGroup ? ' · 小组提交' : ''}`;
});
const scoreLine = computed(() => {
  const value = detail.value;
  if (!value || value.type !== 'homework') return '';
  const parts = [
    value.averageScore == null ? '' : `平均 ${value.averageScore}`,
    value.highestScore == null ? '' : `最高 ${value.highestScore}`,
    value.lowestScore == null ? '' : `最低 ${value.lowestScore}`,
  ].filter(Boolean);
  if (!parts.length) return '';
  const scored = value.scoredCount == null ? '' : `（${value.scoredCount} 人已评阅）`;
  return `${parts.join(' · ')}${scored}`;
});
const submissionRows = computed(() => submissions.value?.submissions ?? []);
const newestFirst = computed(() =>
  [...submissionRows.value].sort((a, b) =>
    (b.submittedAt || b.updatedAt || b.createdAt).localeCompare(
      a.submittedAt || a.updatedAt || a.createdAt,
    ),
  ),
);

function issue(text: string) {
  notice.value = '';
  error.value = text;
}

function inTerm(item: LmsCourse, today: string) {
  return Boolean(item.start && item.end && item.start <= today && today <= item.end);
}

/** Prefer the semester whose course dates contain today, then the newest. */
function defaultSemester() {
  const today = campusDateKey(Date.now());
  return (
    semesters.value.find((semester) => semester.items.some((item) => inTerm(item, today))) ??
    semesters.value[0] ??
    null
  );
}

/** Reopen the last course, else the first course of the current semester. */
async function chooseDefaultCourse() {
  if (course.value || !courses.value.length) return;
  const saved = courses.value.find((item) => item.id === lastCourseId.value);
  if (saved) {
    activeSemester.value = semesterKey(saved);
    await openCourse(saved);
    return;
  }
  const semester = defaultSemester();
  if (!semester) return;
  activeSemester.value = semester.id;
  await openCourse(semester.items[0]);
}

async function loadPrefs() {
  let owner = 'local';
  try {
    const account = await bridge.account();
    owner = account.studentId ?? account.name ?? 'local';
  } catch {
    // The default key is still account-neutral; preferences just stay local.
  }
  favoriteKey = `campus.lms.favorites:${owner}`;
  lastCourseKey = `campus.lms.course:${owner}`;
  try {
    const value = JSON.parse(localStorage.getItem(favoriteKey) ?? '[]');
    if (Array.isArray(value)) {
      favoriteIds.value = value.filter((id): id is number => typeof id === 'number');
    }
  } catch {
    favoriteIds.value = [];
  }
  const last = Number(localStorage.getItem(lastCourseKey));
  lastCourseId.value = Number.isInteger(last) && last > 0 ? last : null;
}

function rememberCourse(id: number) {
  lastCourseId.value = id;
  try {
    localStorage.setItem(lastCourseKey, String(id));
  } catch {
    // Losing the last-course hint only changes the next default.
  }
}

function persistFavorites() {
  try {
    localStorage.setItem(favoriteKey, JSON.stringify(favoriteIds.value));
  } catch {
    // Favorites are a convenience; losing them must not break the page.
  }
}

function toggleFavorite() {
  if (!course.value) return;
  const id = course.value.id;
  favoriteIds.value = favoriteIds.value.includes(id)
    ? favoriteIds.value.filter((value) => value !== id)
    : [...favoriteIds.value, id];
  persistFavorites();
}

function openFavorite(item: LmsCourse) {
  activeSemester.value = semesterKey(item);
  void openCourse(item);
}

function changeSemester(id: string) {
  activeSemester.value = id;
  const first = semesters.value.find((semester) => semester.id === id)?.items[0];
  if (first && first.id !== course.value?.id) void openCourse(first);
}

async function loadCourses(force = false) {
  loadingCourses.value = true;
  error.value = '';
  try {
    await applyCached<LmsCourses>('lmsCourses', undefined, (value) => {
      courses.value = value.courses;
      if (!course.value) void chooseDefaultCourse();
    });
    const result = await siteQuery<LmsCourses>(
      { type: 'lmsCourses' },
      'lms',
      () => bridge.refreshLmsSession(),
      'lmsCourses',
      force,
    );
    courses.value = result.courses;
    if (course.value && !courses.value.some((item) => item.id === course.value?.id)) {
      course.value = null;
      activity.value = null;
      detail.value = null;
      activities.value = [];
    }
    if (course.value) activeSemester.value ||= semesterKey(course.value);
    else void chooseDefaultCourse();
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] lms courses', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    loadingCourses.value = false;
  }
}

async function openCourse(item: LmsCourse, force = false) {
  course.value = item;
  activeSemester.value = semesterKey(item);
  rememberCourse(item.id);
  activity.value = null;
  detail.value = null;
  submissions.value = null;
  activities.value = [];
  active.value = '';
  const params = { course: item.id };
  await applyCached<LmsActivities>('lmsActivities', params, (value) => {
    activities.value = value.activities;
    active.value = categories.value[0]?.id || '';
  });
  loadingActivities.value = true;
  error.value = '';
  notice.value = '';
  try {
    const result = await siteQuery<LmsActivities>(
      { type: 'lmsActivities', course: item.id },
      'lms',
      () => bridge.refreshLmsSession(),
      cacheName('lmsActivities', params),
      force,
    );
    activities.value = result.activities;
    if (!categories.value.some((group) => group.id === active.value))
      active.value = categories.value[0]?.id || '';
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] lms activities', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    loadingActivities.value = false;
  }
}

async function openActivity(item: LmsActivity, force = false) {
  activity.value = item;
  detail.value = null;
  submissions.value = null;
  loadingDetail.value = true;
  error.value = '';
  notice.value = '';
  try {
    detail.value = await siteQuery<LmsActivityDetail>(
      { type: 'lmsActivity', activity: item.id },
      'lms',
      () => bridge.refreshLmsSession(),
      cacheName('lmsActivity', { activity: item.id }),
      force,
    );
    if (detail.value.type === 'homework') void loadSubmissions(item.id, force);
  } catch (e) {
    console.error('[campus] lms activity', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    loadingDetail.value = false;
  }
}

async function loadSubmissions(id: number, force = false) {
  loadingSubmissions.value = true;
  try {
    submissions.value = await siteQuery<LmsSubmissions>(
      { type: 'lmsSubmissions', activity: id },
      'lms',
      () => bridge.refreshLmsSession(),
      cacheName('lmsSubmissions', { activity: id }),
      force,
    );
  } catch (e) {
    // A missing submission list must not hide the activity itself.
    console.error('[campus] lms submissions', e);
    submissions.value = null;
  } finally {
    loadingSubmissions.value = false;
  }
}

async function refresh() {
  if (activity.value) await openActivity(activity.value, true);
  else if (course.value) await openCourse(course.value, true);
  else await loadCourses(true);
}

function changeCourse(id: string) {
  const next = courses.value.find((item) => String(item.id) === id);
  if (next) void openCourse(next);
}

async function openFile(file: LmsUpload) {
  error.value = '';
  notice.value = '';
  try {
    await bridge.lmsOpenFile(file.id);
  } catch (e) {
    console.error('[campus] lms open file', e);
    issue(e instanceof Error ? e.message : String(e));
  }
}

async function downloadFile(file: LmsUpload) {
  error.value = '';
  notice.value = '';
  const target = await save({ defaultPath: await bridge.saveTarget(file.name || '附件') });
  if (!target) return;
  downloading.value = file.id;
  try {
    const bytes = await bridge.lmsSaveFile(file.id, target);
    notice.value = `已保存到 ${await bridge.displayPath(target)}${bytes ? `（${fileSize(bytes)}）` : ''}`;
  } catch (e) {
    console.error('[campus] lms save file', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    downloading.value = 0;
  }
}

function startCapture(item: LmsActivityDetail) {
  emit('capture', { activity: item.id, course: item.courseId, title: item.title });
}

async function openReplay(player: string) {
  error.value = '';
  try {
    await bridge.openUrl(player);
  } catch (e) {
    console.error('[campus] lms replay', e);
    issue(e instanceof Error ? e.message : String(e));
  }
}

function submissionLabel(item: LmsSubmission) {
  if (item.score != null || item.correction?.score != null) return '已评阅';
  return item.status || '已提交';
}
function submissionTone(item: LmsSubmission): StatusTone {
  if (item.score != null || item.correction?.score != null) return 'done';
  return item.latest ? 'done' : 'neutral';
}
function submissionMeta(item: LmsSubmission) {
  const score = item.score ?? item.correction?.score;
  return [
    score == null ? '' : `${score} 分`,
    item.latest ? '最新版本' : '',
    item.submittedAt || item.createdAt || '',
  ]
    .filter(Boolean)
    .join(' · ');
}
function onMore(id: string) {
  if (id === 'lms') void bridge.openUrl('https://lms.xjtu.edu.cn');
}

onMounted(async () => {
  await loadPrefs();
  await loadCourses();
});
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="思源学堂"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loadingCourses || loadingActivities || loadingDetail"
      @refresh="refresh"
    >
      <template #filters>
        <MenuSelect
          :model-value="activeSemester"
          :options="semesterOptions"
          label="学期"
          placeholder="选择学期"
          @update:model-value="changeSemester"
        />
        <MenuSelect
          :model-value="course ? String(course.id) : ''"
          :options="courseOptions"
          label="课程"
          placeholder="选择课程"
          @update:model-value="changeCourse"
        />
        <UiIconButton
          v-if="course"
          variant="accent"
          :active="isFavorite"
          :label="isFavorite ? '取消关注课程' : '关注课程'"
          :title="isFavorite ? '取消关注课程' : '关注课程'"
          @click="toggleFavorite"
        >
          <Star :size="15" :fill="isFavorite ? 'currentColor' : 'none'" />
        </UiIconButton>
      </template>
      <template #more><MoreMenu :items="moreItems" @select="onMore" /></template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="notice" tone="info" :title="notice" />

    <div v-if="favoriteCourses.length" class="lms-favorites" aria-label="关注的课程">
      <button
        v-for="item in favoriteCourses"
        :key="item.id"
        type="button"
        class="lms-favorite"
        :class="{ active: course?.id === item.id }"
        :aria-pressed="course?.id === item.id"
        @click="openFavorite(item)"
      >
        <Star :size="12" fill="currentColor" aria-hidden="true" />
        {{ item.name }}
      </button>
    </div>

    <SplitView
      v-model:width="listWidth"
      :detail-open="Boolean(activity)"
      list-label="活动列表"
      detail-label="活动详情"
      @back="activity = null"
    >
      <template #list>
        <template v-if="!course">
          <Skeleton
            v-if="loadingCourses && !courses.length"
            :lines="8"
            height="18px"
            label="正在读取课程"
          />
          <p v-else-if="!courses.length" class="lms-empty">
            <BookOpen :size="20" aria-hidden="true" />
            没有找到课程
          </p>
          <div v-else class="lms-courses">
            <section v-if="activeSemesterGroup" :key="activeSemesterGroup.id">
              <h3 class="lms-group-title">{{ activeSemesterGroup.title }}</h3>
              <ListItem
                v-for="item in activeSemesterGroup.items"
                :key="item.id"
                :title="item.name"
                :subtitle="[item.code, item.instructors.join('、')].filter(Boolean).join(' · ')"
                @select="openCourse(item)"
              >
                <template #meta>{{ item.credit == null ? '—' : `${item.credit} 学分` }}</template>
              </ListItem>
            </section>
          </div>
        </template>

        <template v-else>
          <Skeleton
            v-if="loadingActivities && !activities.length"
            :lines="8"
            height="18px"
            label="正在读取课程活动"
          />
          <p v-else-if="!activities.length" class="lms-empty">
            {{ error ? '暂时无法读取课程活动' : '这门课暂时没有活动' }}
          </p>
          <template v-else>
            <SegmentedTabs
              v-model="active"
              class="lms-tabs"
              compact
              :tabs="tabs"
              label="活动分类"
            />
            <div class="lms-activities" role="listbox" aria-label="课程活动">
              <ListItem
                v-for="item in activeGroup?.items ?? []"
                :key="item.id"
                :title="item.title || '未命名活动'"
                :subtitle="meta(item)"
                :selected="activity?.id === item.id"
                @select="openActivity(item)"
              >
                <template #status>
                  <StatusBadge
                    v-if="homeworkTone(item)"
                    :status="homeworkTone(item)!.tone"
                    :label="homeworkTone(item)!.label"
                  />
                  <span v-else-if="!item.published" class="badge">未发布</span>
                </template>
              </ListItem>
            </div>
          </template>
        </template>
      </template>

      <template #detail>
        <Skeleton
          v-if="loadingDetail && !detail"
          :lines="8"
          height="18px"
          label="正在读取活动详情"
        />
        <article v-else-if="detail" class="lms-detail">
          <header class="lms-detail-head">
            <p class="lms-detail-meta">
              <span>{{ detailType }}</span>
              <StatusBadge :status="statusTone" :label="statusLabel" />
            </p>
            <h3>{{ detail.title || '活动详情' }}</h3>
            <DueLabel
              v-if="detail.type === 'homework'"
              :at="dueAt"
              :done="(detail.submitCount ?? 0) > 0 || detail.closed"
            />
            <p v-else class="lms-detail-time">{{ timeRow }}</p>
          </header>

          <InlineAlert v-if="detail.type === 'homework'" tone="info" title="应用内提交尚未开放">
            作业只能在这里查看要求与提交记录；提交和撤回请到思源学堂网页办理。
          </InlineAlert>

          <SettingGroup title="活动信息">
            <SettingRow title="时间">
              <span class="setting-value">{{ timeRow }}</span>
            </SettingRow>
            <SettingRow v-if="detail.type === 'lesson' && detail.duration" title="时长">
              <span class="setting-value">{{
                detail.duration == null
                  ? '—'
                  : `${Math.max(1, Math.round(detail.duration / 60))} 分钟`
              }}</span>
            </SettingRow>
            <SettingRow v-if="submissionLine" title="提交情况">
              <span class="setting-value">{{ submissionLine }}</span>
            </SettingRow>
            <SettingRow v-if="scoreLine" title="成绩">
              <span class="setting-value">{{ scoreLine }}</span>
            </SettingRow>
          </SettingGroup>

          <SettingGroup v-if="detail.description" title="活动说明">
            <p class="lms-description">{{ detail.description }}</p>
          </SettingGroup>

          <SettingGroup v-if="detail.uploads.length" title="附件">
            <FileRow
              v-for="file in detail.uploads"
              :key="file.id"
              :name="file.name || '未命名附件'"
              :size="file.size"
              :kind="file.type"
              :downloaded="false"
              :previewable="false"
              :downloadable="file.allowDownload"
              :busy="downloading === file.id"
              @download="downloadFile(file)"
            >
              <template #actions>
                <UiButton size="sm" variant="ghost" @click="openFile(file)">打开</UiButton>
              </template>
            </FileRow>
          </SettingGroup>

          <SettingGroup v-if="detail.type === 'lesson'" title="回放">
            <SettingRow
              v-if="detail.player"
              title="观看课程回放"
              description="在校园窗口中播放，会话随当前账号"
            >
              <UiButton size="sm" @click="openReplay(detail.player)">
                <PlayCircle :size="14" />打开回放
              </UiButton>
            </SettingRow>
            <SettingRow
              v-if="detail.player"
              title="截取讲义"
              description="把回放里的幻灯片截成 PDF 或图片包，只保存在本机"
            >
              <UiButton size="sm" variant="ghost" @click="startCapture(detail)">
                <Presentation :size="14" />截取讲义
              </UiButton>
            </SettingRow>
            <SettingRow
              v-else-if="detail.replayCode"
              title="回放暂不可用"
              description="已登记回放，但当前没有可用的播放地址"
            />
            <SettingRow v-else title="暂无回放" description="这门录播课还没有生成回放" />
          </SettingGroup>

          <SettingGroup v-if="detail.type === 'lecture_live'" title="直播">
            <SettingRow title="直播时间" :description="detail.liveName || ''">
              <span class="setting-value">{{ timeRow }}</span>
            </SettingRow>
            <SettingRow
              v-if="detail.replayCode"
              title="直播回放"
              description="直播回放暂未开放，可在思源学堂网页中查看"
            />
          </SettingGroup>

          <SettingGroup v-if="detail.type === 'homework'" title="提交记录">
            <Skeleton
              v-if="loadingSubmissions && !submissions"
              :lines="3"
              height="16px"
              label="正在读取提交记录"
            />
            <p v-else-if="!newestFirst.length" class="lms-empty">还没有提交记录</p>
            <div v-else class="lms-submissions">
              <article v-for="item in newestFirst" :key="item.id" class="lms-submission">
                <header>
                  <strong>{{ item.latest ? '最新版本' : '历史版本' }}</strong>
                  <StatusBadge :status="submissionTone(item)" :label="submissionLabel(item)" />
                </header>
                <p class="lms-submission-meta">{{ submissionMeta(item) }}</p>
                <p v-if="item.comment" class="lms-submission-text">{{ item.comment }}</p>
                <p v-if="item.instructorComment" class="lms-submission-comment">
                  教师评语：{{ item.instructorComment }}
                </p>
                <FileRow
                  v-for="file in item.files"
                  :key="file.id"
                  :name="file.name || '附件'"
                  :size="file.size"
                  :kind="file.type"
                  :previewable="false"
                  :downloadable="file.allowDownload"
                  :busy="downloading === file.id"
                  @download="downloadFile(file)"
                >
                  <template #actions>
                    <UiButton size="sm" variant="ghost" @click="openFile(file)">打开</UiButton>
                  </template>
                </FileRow>
                <FileRow
                  v-for="file in item.correction?.files ?? []"
                  :key="file.id"
                  :name="file.name || '评阅附件'"
                  :size="file.size"
                  :kind="file.type"
                  :previewable="false"
                  :downloadable="file.allowDownload"
                  :busy="downloading === file.id"
                  @download="downloadFile(file)"
                >
                  <template #actions>
                    <UiButton size="sm" variant="ghost" @click="openFile(file)">打开</UiButton>
                  </template>
                </FileRow>
              </article>
            </div>
          </SettingGroup>
        </article>
        <div v-else class="lms-placeholder">
          <p v-if="activity">暂时无法读取活动详情</p>
          <p v-else-if="course">从左侧选择一个活动，查看要求、附件与提交记录。</p>
          <p v-else>选择一门课程开始。</p>
        </div>
      </template>
    </SplitView>
  </section>
</template>

<style scoped>
.lms-favorites {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-bottom: 12px;
}
.lms-favorite {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  height: 24px;
  padding: 0 9px;
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--surface);
  color: var(--text-secondary);
  font-size: 12px;
  cursor: pointer;
}
.lms-favorite:hover {
  background: var(--surface-hover);
  color: var(--text);
}
.lms-favorite.active {
  border-color: var(--accent);
  background: var(--accent-soft);
  color: var(--accent-text);
}
.lms-courses {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.lms-group-title {
  padding: 6px 10px 4px;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 600;
}
.lms-activities {
  display: flex;
  flex-direction: column;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  overflow: hidden;
}
.lms-tabs {
  margin-bottom: 10px;
}
.lms-empty,
.lms-placeholder {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 8px;
  padding: 32px 16px;
  color: var(--text-tertiary);
  font-size: 12.5px;
  text-align: center;
}
.lms-detail {
  display: flex;
  flex-direction: column;
  gap: 14px;
  min-width: 0;
}
.lms-detail-head {
  display: flex;
  flex-direction: column;
  gap: 6px;
}
.lms-detail-meta {
  display: flex;
  align-items: center;
  gap: 8px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.lms-detail-head h3 {
  font-size: 16px;
  font-weight: 600;
  line-height: 1.4;
}
.lms-detail-time {
  color: var(--text-secondary);
  font-size: 12.5px;
  font-variant-numeric: tabular-nums;
}
.lms-description {
  margin: 0;
  padding: 12px 16px 14px;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.7;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.lms-submissions {
  display: flex;
  flex-direction: column;
}
.lms-submission {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 16px;
  border-bottom: 1px solid var(--divider);
}
.lms-submission:last-child {
  border-bottom: 0;
}
.lms-submission header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
}
.lms-submission header strong {
  font-size: 12.5px;
  font-weight: 600;
}
.lms-submission-meta {
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.lms-submission-text,
.lms-submission-comment {
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.65;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.lms-submission-comment {
  padding: 8px 10px;
  border-radius: var(--radius-sm);
  background: var(--surface-subtle);
}
</style>
