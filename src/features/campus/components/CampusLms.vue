<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookOpen, ExternalLink, PlayCircle, Presentation, Search, Star } from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import type { StatusTone } from '../../../components/ui/StatusBadge.vue';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import { bridge } from '../../../shared/bridge';
import { siteQuery } from '../campusSession';
import { applyCached, cacheName } from '../../../shared/cache';
import {
  campusDateKey,
  campusTimestamp,
  courseHue,
  dueInfo,
  fileSize,
  lmsTime,
} from '../../../shared/domain';
import type {
  LmsActivities,
  LmsActivity,
  LmsActivityDetail,
  LmsCourse,
  LmsCourses,
  LmsSubmission,
  LmsSubmissions,
  LmsUpload,
} from '../../../types';

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
const query = ref('');

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
const visibleCourses = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  const items = activeSemesterGroup.value?.items ?? [];
  if (!keyword) return items;
  return items.filter((item) =>
    `${item.name} ${item.code} ${item.instructors.join(' ')}`.toLowerCase().includes(keyword),
  );
});
const visibleActivities = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  const items = activeGroup.value?.items ?? [];
  if (!keyword) return items;
  return items.filter((item) => (item.title || '').toLowerCase().includes(keyword));
});
const courseColor = (name: string) => `c${courseHue(name) + 1}`;
const toneClass = (tone: StatusTone | null): string =>
  tone === 'done'
    ? 'ok'
    : tone === 'active'
      ? 'accent'
      : tone === 'urgent'
        ? 'danger'
        : tone === 'near'
          ? 'warn'
          : 'muted';
const subtitle = computed(
  () =>
    [course.value?.name, activeSemesterGroup.value?.title].filter(Boolean).join(' · ') ||
    '课程与活动',
);
const detailType = computed(() => (detail.value ? typeName(detail.value.type) : ''));
const dueAt = computed(() => (detail.value?.end ? campusTimestamp(detail.value.end) : null));

const short = lmsTime;
/** Same-day ranges drop the repeated date so the time column stays narrow. */
const span = (start: string, end: string) => {
  const from = short(start);
  const to = short(end);
  if (!from || !to) return from || to || '时间未公布';
  if (from.slice(0, 10) === to.slice(0, 10)) return `${from.slice(5)} – ${to.slice(11)}`;
  return `${from} – ${to}`;
};
const typeName = (type: string) => TYPE_LABEL[type] || '活动';
const meta = (item: LmsActivity) => {
  if (item.type === 'homework') return item.end ? `截止 ${short(item.end)}` : '无截止时间';
  if (item.type === 'lesson' || item.type === 'lecture_live') return span(item.start, item.end);
  return item.start ? short(item.start) : '';
};
// A submitted homework keeps saying 已提交 after its deadline. "已截止" only
// describes homework the student has not handed in.
const homeworkTone = (item: LmsActivity): { tone: StatusTone; label: string } | null => {
  if (item.type !== 'homework') return null;
  if ((item.submitCount ?? 0) > 0) return { tone: 'done', label: '已提交' };
  if (item.closed) return { tone: 'neutral', label: '已截止' };
  if (!item.end) return null;
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
    return (
      [value.liveName, span(value.liveStart || value.start, value.liveEnd || value.end)]
        .filter(Boolean)
        .join(' · ') || '直播时间未公布'
    );
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
async function openLms() {
  try {
    await bridge.openUrl('https://lms.xjtu.edu.cn');
  } catch (e) {
    issue(e instanceof Error ? e.message : String(e));
  }
}

const busy = computed(() => loadingCourses.value || loadingActivities.value || loadingDetail.value);
const ext = (name?: string) => {
  const suffix = (name ?? '').split('.').pop();
  return suffix && suffix !== name ? suffix.slice(0, 4).toUpperCase() : '文件';
};
function pickSemester(event: Event) {
  changeSemester((event.target as HTMLSelectElement).value);
}

onMounted(async () => {
  await loadPrefs();
  await loadCourses();
});
</script>
<template>
  <CampusHeaderSlot slot="tools">
    <select class="cv-select" :value="activeSemester" aria-label="学期" @change="pickSemester">
      <option v-for="option in semesterOptions" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
    <label class="cv-search">
      <Search :size="13" aria-hidden="true" />
      <input
        v-model="query"
        type="search"
        placeholder="搜索课程或活动"
        aria-label="搜索课程或活动"
        spellcheck="false"
      />
    </label>
    <button type="button" class="cv-btn" :disabled="busy" @click="refresh">刷新</button>
  </CampusHeaderSlot>

  <div
    class="cv-cols lcr"
    style="grid-template-columns: minmax(184px, 214px) minmax(0, 1fr) minmax(248px, 300px)"
  >
    <div class="cv-panel cv-list">
      <template v-if="favoriteCourses.length">
        <div class="cv-lh">关注</div>
        <button
          v-for="item in favoriteCourses"
          :key="`fav-${item.id}`"
          type="button"
          class="cv-item"
          :class="{ on: course?.id === item.id }"
          @click="openFavorite(item)"
        >
          <i class="cv-cs" :class="courseColor(item.name)" aria-hidden="true"></i>
          <b>{{ item.name }}</b>
          <Star class="cv-star" :size="11" fill="currentColor" aria-hidden="true" />
        </button>
      </template>

      <div class="cv-lh">
        {{ activeSemesterGroup?.title || '课程' }}<span>{{ visibleCourses.length }}</span>
      </div>
      <button
        v-for="item in visibleCourses"
        :key="item.id"
        type="button"
        class="cv-item"
        :class="{ on: course?.id === item.id }"
        @click="openCourse(item)"
      >
        <i class="cv-cs" :class="courseColor(item.name)" aria-hidden="true"></i>
        <b>{{ item.name }}</b>
        <span class="cv-n">{{ item.credit == null ? '' : `${item.credit} 学分` }}</span>
        <span class="s">{{
          [item.code, item.instructors.join('、')].filter(Boolean).join(' · ')
        }}</span>
      </button>

      <div v-if="loadingCourses && !courses.length" class="cv-lh">正在读取课程…</div>
      <div v-else-if="!courses.length" class="cv-empty" style="min-height: 120px">
        <BookOpen :size="20" /><b>没有找到课程</b>
      </div>
    </div>

    <div class="cv-center">
      <template v-if="course">
        <div class="cv-ch">
          <b>{{ course.name }}</b>
          <span class="meta">
            {{
              [
                course.code,
                course.instructors.join('、'),
                course.credit == null ? '' : `${course.credit} 学分`,
              ]
                .filter(Boolean)
                .join(' · ')
            }}
          </span>
          <span class="end">
            <button type="button" class="cv-btn sm" @click="toggleFavorite">
              <Star class="cv-star" :size="12" :fill="isFavorite ? 'currentColor' : 'none'" />
              {{ isFavorite ? '已关注' : '关注' }}
            </button>
          </span>
        </div>

        <div class="cv-seg" style="align-self: flex-start">
          <button
            v-for="group in categories"
            :key="group.id"
            type="button"
            :class="{ on: activeGroup?.id === group.id }"
            @click="active = group.id"
          >
            {{ group.title }}<span class="n">{{ group.items.length }}</span>
          </button>
        </div>

        <div v-if="loadingActivities && !activities.length" class="cv-empty">
          <BookOpen :size="20" /><b>正在读取课程活动…</b>
        </div>
        <div v-else-if="!visibleActivities.length" class="cv-empty">
          <BookOpen :size="20" />
          <b>{{ query ? '没有匹配的活动' : '这门课在这个分类下没有活动' }}</b>
        </div>
        <div v-else class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 1fr 116px 64px">
            <span>活动</span><span class="cv-r">时间</span><span class="cv-r">状态</span>
          </div>
          <button
            v-for="item in visibleActivities"
            :key="item.id"
            type="button"
            class="cv-tr"
            style="--cols: 1fr 116px 64px; height: 38px"
            :class="{ sel: activity?.id === item.id }"
            @click="openActivity(item)"
          >
            <span class="c">
              <span class="cv-tt">
                <span class="cv-ellip">{{ item.title || '未命名活动' }}</span>
                <small>{{ typeName(item.type) }}</small>
              </span>
            </span>
            <span class="cv-r cv-num cv-sec cv-ellip">{{ meta(item) }}</span>
            <span class="cv-r">
              <span
                v-if="homeworkTone(item)"
                class="cv-badge"
                :class="toneClass(homeworkTone(item)!.tone)"
              >
                {{ homeworkTone(item)!.label }}
              </span>
              <span v-else-if="!item.published" class="cv-badge muted">未发布</span>
              <span v-else class="cv-muted">—</span>
            </span>
          </button>
        </div>
      </template>

      <div v-else-if="loadingCourses && !courses.length" class="cv-empty">
        <BookOpen :size="20" /><b>正在读取课程…</b>
      </div>
      <div v-else class="cv-empty">
        <BookOpen :size="20" /><b>选择一门课程</b>
        <span>左侧按学期列出思源学堂的课程。</span>
      </div>
    </div>

    <div class="cv-panel cv-detail">
      <template v-if="loadingDetail && !detail">
        <div class="cv-placeholder">正在读取活动详情…</div>
      </template>
      <template v-else-if="detail">
        <div class="cv-type">
          {{ detailType }}
          <span class="cv-badge" :class="toneClass(statusTone)">{{ statusLabel }}</span>
        </div>
        <h3>{{ detail.title || '活动详情' }}</h3>
        <p
          style="font-size: 12px; margin: 0"
          :class="
            detail.type === 'homework' && dueAt && dueAt < Date.now() ? 'cv-due-today' : 'cv-sec'
          "
        >
          {{ timeRow }}
        </p>

        <dl class="cv-kv">
          <template v-if="detail.type === 'homework'">
            <dt>提交</dt>
            <dd>{{ submissionLine || '—' }}</dd>
          </template>
          <template v-if="detail.type === 'lesson' && detail.duration">
            <dt>时长</dt>
            <dd>{{ Math.max(1, Math.round(detail.duration / 60)) }} 分钟</dd>
          </template>
          <template v-if="scoreLine">
            <dt>成绩</dt>
            <dd>{{ scoreLine }}</dd>
          </template>
        </dl>

        <div v-if="detail.description" class="cv-blk">
          <h4>说明</h4>
          <p>{{ detail.description }}</p>
        </div>

        <div v-if="detail.uploads.length" class="cv-blk">
          <h4>
            附件<span>{{ detail.uploads.length }}</span>
          </h4>
          <div v-for="file in detail.uploads" :key="file.id" class="cv-file">
            <i class="cv-ft">{{ ext(file.name) }}</i>
            <span class="cv-ellip">{{ file.name || '未命名附件' }}</span>
            <span class="sz cv-num">{{ fileSize(file.size) }}</span>
            <button type="button" class="cv-btn sm ghost" @click="openFile(file)">打开</button>
            <button
              type="button"
              class="cv-btn sm ghost"
              :disabled="downloading === file.id"
              @click="downloadFile(file)"
            >
              下载
            </button>
          </div>
        </div>

        <div v-if="detail.type === 'lesson'" class="cv-blk">
          <h4>回放</h4>
          <template v-if="detail.player">
            <div class="cv-file">
              <span class="cv-sec">观看课程回放</span>
              <button type="button" class="cv-btn sm" @click="openReplay(detail.player!)">
                <PlayCircle :size="12" />打开
              </button>
            </div>
            <div class="cv-file">
              <span class="cv-sec">截取讲义</span>
              <button type="button" class="cv-btn sm" @click="startCapture(detail)">
                <Presentation :size="12" />截取
              </button>
            </div>
          </template>
          <p v-else-if="detail.replayCode" class="cv-nodata">
            已登记回放，但当前没有可用的播放地址。
          </p>
          <p v-else class="cv-nodata">这门录播课还没有生成回放。</p>
        </div>

        <div v-if="detail.type === 'lecture_live'" class="cv-blk">
          <h4>直播</h4>
          <p>{{ timeRow }}</p>
          <p v-if="detail.replayCode" class="cv-nodata">
            直播回放暂未开放，可在思源学堂网页中查看。
          </p>
        </div>

        <div v-if="detail.type === 'homework'" class="cv-blk">
          <h4>
            提交记录<span>{{ newestFirst.length }}</span>
          </h4>
          <p v-if="loadingSubmissions && !submissions" class="cv-nodata">正在读取提交记录…</p>
          <p v-else-if="!newestFirst.length" class="cv-nodata">还没有提交记录。</p>
          <div v-for="item in newestFirst" :key="item.id" class="cv-order">
            <div class="l1">
              <b>{{ item.latest ? '最新版本' : '历史版本' }}</b>
              <span class="cv-badge" :class="toneClass(submissionTone(item))">
                {{ submissionLabel(item) }}
              </span>
            </div>
            <span class="cv-muted cv-num">{{ submissionMeta(item) }}</span>
            <span v-if="item.comment" class="cv-sec">{{ item.comment }}</span>
            <span v-if="item.instructorComment" class="cv-sec"
              >教师评语：{{ item.instructorComment }}</span
            >
            <div
              v-for="file in [...item.files, ...(item.correction?.files ?? [])]"
              :key="file.id"
              class="cv-file"
            >
              <i class="cv-ft">{{ ext(file.name) }}</i>
              <span class="cv-ellip">{{ file.name || '附件' }}</span>
              <button type="button" class="cv-btn sm ghost" @click="openFile(file)">打开</button>
            </div>
          </div>
        </div>

        <div class="cv-blk">
          <p>作业只能在这里查看要求与提交记录；提交和撤回仍在思源学堂网页办理。</p>
        </div>
        <div class="cv-foot">
          <button type="button" class="cv-btn" @click="openLms">
            <ExternalLink :size="13" />在思源学堂打开
          </button>
        </div>
      </template>
      <div v-else class="cv-placeholder">
        {{ course ? '选择左侧的一个活动，查看要求、附件与提交记录。' : '选择一门课程开始。' }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.cv-select {
  max-width: 190px;
}
/* A followed course is marked with a star, so it keeps star colour instead of
   inheriting the black text of the row or the neutral border of the button. */
.cv-star {
  color: #f0b429;
  flex-shrink: 0;
}
</style>
