<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { BookOpen } from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import SegmentedTabs from '../ui/SegmentedTabs.vue';
import EmptyState from '../ui/EmptyState.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import { applyCached, writeCached } from '../../cache';
import UiButton from '../ui/UiButton.vue';
import type {
  LmsActivities,
  LmsActivity,
  LmsActivityDetail,
  LmsCourse,
  LmsCourses,
  LmsUpload,
} from '../../types';

const courses = ref<LmsCourse[]>([]);
const selected = ref<LmsCourse | null>(null);
const activities = ref<LmsActivity[]>([]);
const activity = ref<LmsActivity | null>(null);
const detail = ref<LmsActivityDetail | null>(null);
const loading = ref(true);
const downloading = ref(0);
const error = ref('');
const notice = ref('');

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
const typeName = (type: string) => TYPE_LABEL[type] || '活动';
/** Activities are grouped by what the student needs to do with them. */
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
/** Courses come back in no useful order; group them by academic year and term,
 * newest first, so the current semester is at the top of the page. */
const semesters = computed(() => {
  const groups = new Map<string, { id: string; title: string; rank: number; items: LmsCourse[] }>();
  for (const course of courses.value) {
    const year = course.year.trim();
    const term = course.term.trim();
    const title = [year, term].filter(Boolean).join(' · ') || '未标注学期';
    let group = groups.get(title);
    if (!group) {
      const start = Number(year.slice(0, 4));
      group = {
        id: title,
        title,
        rank: Number.isFinite(start) ? start * 10 + (TERM_RANK[term] || 0) : -Infinity,
        items: [],
      };
      groups.set(title, group);
    }
    group.items.push(course);
  }
  return [...groups.values()].sort((a, b) => b.rank - a.rank);
});
/** Only categories that actually have activities become tabs. */
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
const active = ref('');
const tabs = computed(() =>
  categories.value.map((group) => ({
    id: group.id,
    title: `${group.title} ${group.items.length}`,
  })),
);
const activeGroup = computed(
  () => categories.value.find((group) => group.id === active.value) || categories.value[0],
);

/** Trim ISO timestamps to minutes; the backend passes them through unchanged. */
const short = (value: string) => (value ? value.replace('T', ' ').slice(0, 16) : '');
const span = (start: string, end: string) =>
  [short(start), short(end)].filter(Boolean).join(' – ') || '时间未公布';
const size = (bytes: number | null) => {
  if (bytes == null) return '';
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
};
const minutes = (seconds: number | null) =>
  seconds == null ? '' : `${Math.max(1, Math.round(seconds / 60))} 分钟`;
const meta = (item: LmsActivity) => {
  if (item.type === 'homework') return item.end ? `截止 ${short(item.end)}` : '无截止时间';
  if (item.type === 'lesson') return span(item.start, item.end);
  if (item.type === 'lecture_live') return span(item.start, item.end);
  return item.start ? short(item.start) : '';
};
const fileMeta = (file: LmsUpload) => {
  const kinds: Record<string, string> = {
    document: '文档',
    video: '视频',
    audio: '音频',
    image: '图片',
    scorm: '课件',
  };
  return [kinds[file.type] || '文件', size(file.size)].filter(Boolean).join(' · ');
};
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
const submission = computed(() => {
  const value = detail.value;
  if (!value || value.type !== 'homework') return '';
  const count = value.submitCount == null ? '—' : value.submitCount;
  const allowed = value.submitTimes == null ? '' : ` / 可提交 ${value.submitTimes} 次`;
  return `已提交 ${count} 次${allowed}${value.submitByGroup ? ' · 小组提交' : ''}`;
});
const score = computed(() => {
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

function issue(text: string) {
  notice.value = '';
  error.value = text;
}

async function loadCourses() {
  await applyCached<LmsCourses>(
    'lmsCourses',
    undefined,
    (value) => (courses.value = value.courses),
  );
  loading.value = true;
  error.value = '';
  try {
    const result = await siteQuery<LmsCourses>({ type: 'lmsCourses' }, 'lms', () =>
      bridge.refreshLmsSession(),
    );
    courses.value = result.courses;
    void writeCached('lmsCourses', undefined, result);
  } catch (e) {
    console.error('[campus] lms courses', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
}

async function openCourse(course: LmsCourse) {
  selected.value = course;
  activity.value = null;
  detail.value = null;
  activities.value = [];
  const params = { course: course.id };
  await applyCached<LmsActivities>(
    'lmsActivities',
    params,
    (value) => (activities.value = value.activities),
  );
  loading.value = true;
  error.value = '';
  notice.value = '';
  try {
    const result = await siteQuery<LmsActivities>(
      { type: 'lmsActivities', course: course.id },
      'lms',
      () => bridge.refreshLmsSession(),
    );
    activities.value = result.activities;
    void writeCached('lmsActivities', params, result);
    active.value = categories.value[0]?.id || '';
  } catch (e) {
    console.error('[campus] lms activities', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
}

async function openActivity(item: LmsActivity) {
  activity.value = item;
  detail.value = null;
  loading.value = true;
  error.value = '';
  notice.value = '';
  try {
    detail.value = await siteQuery<LmsActivityDetail>(
      { type: 'lmsActivity', activity: item.id },
      'lms',
      () => bridge.refreshLmsSession(),
    );
  } catch (e) {
    console.error('[campus] lms activity', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    loading.value = false;
  }
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
  const target = await save({ defaultPath: file.name || '附件' });
  if (!target) return;
  downloading.value = file.id;
  try {
    const bytes = await bridge.lmsSaveFile(file.id, target);
    notice.value = `已保存到 ${target}${bytes ? `（${size(bytes)}）` : ''}`;
  } catch (e) {
    console.error('[campus] lms save file', e);
    issue(e instanceof Error ? e.message : String(e));
  } finally {
    downloading.value = 0;
  }
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

function backToCourses() {
  selected.value = null;
  activity.value = null;
  detail.value = null;
  activities.value = [];
  active.value = '';
  error.value = '';
  notice.value = '';
}

function backToActivities() {
  activity.value = null;
  detail.value = null;
  error.value = '';
  notice.value = '';
}

onMounted(loadCourses);
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>
  <p v-if="notice" class="notice success">{{ notice }}</p>

  <!-- Course list, one group per semester -->
  <template v-if="!selected">
    <EmptyState v-if="loading && !courses.length" compact title="正在读取课程…" />
    <EmptyState v-else-if="!courses.length" compact :icon="BookOpen" title="没有找到课程" />
    <SettingGroup v-for="semester in semesters" :key="semester.id" :title="semester.title">
      <SettingRow
        v-for="course in semester.items"
        :key="course.id"
        clickable
        :disabled="loading"
        :title="course.name"
        :description="[course.code, course.instructors.join('、')].filter(Boolean).join(' · ')"
        @select="openCourse(course)"
      >
        <span class="campus-record-meta">{{
          course.credit == null ? '—' : `${course.credit} 学分`
        }}</span>
      </SettingRow>
    </SettingGroup>
  </template>

  <!-- Categorised activities of one course -->
  <template v-else-if="!activity">
    <div class="campus-subhead">
      <UiButton variant="ghost" size="sm" @click="backToCourses">返回课程</UiButton>
      <strong>{{ selected.name }}</strong>
    </div>
    <EmptyState v-if="loading && !activities.length" compact title="正在读取课程活动…" />
    <EmptyState
      v-else-if="!activities.length"
      compact
      :title="error ? '暂时无法读取课程活动' : '这门课暂时没有活动'"
    />
    <template v-else>
      <SegmentedTabs v-model="active" class="lms-tabs" compact :tabs="tabs" label="活动分类" />
      <SettingGroup v-if="activeGroup">
        <SettingRow
          v-for="item in activeGroup.items"
          :key="item.id"
          clickable
          :disabled="loading"
          :title="item.title || '未命名活动'"
          :description="meta(item)"
          @select="openActivity(item)"
        >
          <span v-if="!item.published" class="badge">未发布</span>
          <span v-else class="badge accent">{{ typeName(item.type) }}</span>
        </SettingRow>
      </SettingGroup>
    </template>
  </template>

  <!-- Activity detail -->
  <template v-else>
    <div class="campus-subhead">
      <UiButton variant="ghost" size="sm" @click="backToActivities">返回活动</UiButton>
      <strong>{{ detail?.title || activity.title || '活动详情' }}</strong>
    </div>
    <EmptyState v-if="loading && !detail" compact title="正在读取活动详情…" />
    <template v-else-if="detail">
      <SettingGroup title="活动信息">
        <SettingRow title="类型">
          <span class="badge accent">{{ typeName(detail.type) }}</span>
          <span v-if="!detail.published" class="badge">未发布</span>
        </SettingRow>
        <SettingRow :title="detail.type === 'homework' ? '截止时间' : '时间'">
          <span class="setting-value">{{ timeRow }}</span>
        </SettingRow>
        <SettingRow v-if="detail.type === 'lesson' && detail.duration" title="时长">
          <span class="setting-value">{{ minutes(detail.duration) }}</span>
        </SettingRow>
        <SettingRow v-if="submission" title="提交情况">
          <span class="setting-value">{{ submission }}</span>
        </SettingRow>
        <SettingRow v-if="score" title="成绩">
          <span class="setting-value">{{ score }}</span>
        </SettingRow>
      </SettingGroup>

      <SettingGroup v-if="detail.description" title="活动说明">
        <p class="lms-description">{{ detail.description }}</p>
      </SettingGroup>

      <SettingGroup v-if="detail.uploads.length" title="附件">
        <SettingRow
          v-for="file in detail.uploads"
          :key="file.id"
          :title="file.name || '未命名附件'"
          :description="fileMeta(file)"
        >
          <UiButton size="sm" @click="openFile(file)">打开</UiButton>
          <UiButton
            size="sm"
            v-if="file.allowDownload"
            :disabled="downloading === file.id"
            @click="downloadFile(file)"
          >
            {{ downloading === file.id ? '下载中…' : '下载' }}
          </UiButton>
        </SettingRow>
      </SettingGroup>

      <SettingGroup v-if="detail.type === 'lesson'" title="回放">
        <SettingRow
          v-if="detail.player"
          title="观看课程回放"
          description="在校园窗口中播放，会话随当前账号"
        >
          <UiButton size="sm" @click="openReplay(detail.player)">打开回放</UiButton>
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
    </template>
    <EmptyState v-else compact title="暂时无法读取活动详情" />
  </template>
</template>

<style scoped>
/* Sub-page heading: back action plus the course or activity being viewed. */
.campus-subhead {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 26px;
  margin-bottom: 14px;
}
.campus-subhead strong {
  overflow: hidden;
  font-size: 13.5px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
/* Category filter bar under the course heading. */
.lms-tabs {
  margin: 0 0 14px;
}
/* Activity descriptions arrive as plain text with line breaks preserved. */
.lms-description {
  margin: 0;
  padding: 12px 16px 14px;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.7;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
</style>
