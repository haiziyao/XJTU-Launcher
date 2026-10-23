<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { BookOpen } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import { bridge } from '../../bridge';
import { siteQuery } from '../../campusSession';
import type { LmsActivities, LmsActivity, LmsCourse, LmsCourses } from '../../types';

const courses = ref<LmsCourse[]>([]);
const selected = ref<LmsCourse | null>(null);
const activities = ref<LmsActivity[]>([]);
const loading = ref(true);
const error = ref('');

const TYPES: Record<string, string> = {
  assignment: '作业',
  homework: '作业',
  quiz: '测验',
  exercise: '练习',
  exam: '考试',
  forum: '讨论',
  discussion: '讨论',
  material: '资料',
  courseware: '课件',
  document: '文档',
  file: '文件',
  video: '视频',
  live: '直播',
  questionnaire: '问卷',
  announcement: '公告',
};
const typeName = (type: string) => TYPES[type] || type || '活动';
/** Trim ISO timestamps to minutes; the backend passes them through unchanged. */
const short = (value: string) => value.replace('T', ' ').slice(0, 16);

async function loadCourses() {
  loading.value = true;
  error.value = '';
  try {
    const result = await siteQuery<LmsCourses>({ type: 'lmsCourses' }, 'lms', () =>
      bridge.refreshLmsSession(),
    );
    courses.value = result.courses;
  } catch (e) {
    console.error('[campus] lms courses', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function openCourse(course: LmsCourse) {
  selected.value = course;
  activities.value = [];
  loading.value = true;
  error.value = '';
  try {
    const result = await siteQuery<LmsActivities>(
      { type: 'lmsActivities', course: course.id },
      'lms',
      () => bridge.refreshLmsSession(),
    );
    activities.value = result.activities;
  } catch (e) {
    console.error('[campus] lms activities', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function back() {
  selected.value = null;
  activities.value = [];
  error.value = '';
}

onMounted(loadCourses);
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>

  <SettingGroup v-if="!selected" title="我的课程">
    <EmptyState v-if="loading && !courses.length" compact title="正在读取课程…" />
    <EmptyState v-else-if="!courses.length" compact :icon="BookOpen" title="没有找到课程" />
    <SettingRow
      v-for="course in courses"
      :key="course.id"
      clickable
      :disabled="loading"
      :title="course.name"
      :description="
        [
          course.code,
          course.instructors.join('、'),
          [course.year, course.term].filter(Boolean).join(' '),
        ]
          .filter(Boolean)
          .join(' · ')
      "
      @select="openCourse(course)"
    >
      <span class="campus-record-meta">{{
        course.credit == null ? '—' : `${course.credit} 学分`
      }}</span>
    </SettingRow>
  </SettingGroup>

  <SettingGroup v-if="selected" :title="`课程活动 · ${selected.name}`">
    <template #actions>
      <button class="btn ghost small" @click="back">返回课程</button>
    </template>
    <EmptyState v-if="loading && !activities.length" compact title="正在读取活动…" />
    <EmptyState v-else-if="!activities.length" compact title="这门课暂时没有活动" />
    <div v-for="item in activities" :key="item.id" class="campus-record three">
      <span class="campus-record-main">
        <strong>{{ item.title || '未命名活动' }}</strong>
        <small>{{
          [typeName(item.type), item.group ? '小组提交' : ''].filter(Boolean).join(' · ')
        }}</small>
      </span>
      <span v-if="!item.published" class="badge">未发布</span>
      <span v-else />
      <span class="campus-record-meta">{{
        item.end ? `截止 ${short(item.end)}` : '无截止时间'
      }}</span>
    </div>
  </SettingGroup>
</template>
