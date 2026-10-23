<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import {
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  LoaderCircle,
  Plus,
  RefreshCw,
  RotateCcw,
} from 'lucide-vue-next';
import { bridge } from '../bridge';
import { academicWeek, courseHue } from '../domain';
import type { Course, Timetable, TimetableEdit } from '../types';
import PageHeader from './ui/PageHeader.vue';
import EmptyState from './ui/EmptyState.vue';
import TimetableEditor from './TimetableEditor.vue';
import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
const props = defineProps<{
  timetable: Timetable | null;
  connected: boolean;
  loading: boolean;
  error: string;
}>();
const emit = defineEmits<{ refresh: []; account: []; updated: [timetable: Timetable] }>();
const week = ref(1);
const days = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];
const maxWeek = computed(() => {
  if (!props.timetable) return 1;
  const t = props.timetable;
  const end = new Date(t.endDate.slice(0, 10) + 'T12:00:00');
  const calendarWeeks = Number.isNaN(end.getTime()) ? 1 : academicWeek(t.startDate, end);
  return Math.min(60, Math.max(1, calendarWeeks, ...t.courses.flatMap((c) => c.weeks)));
});
watch(
  () => props.timetable,
  (table) => {
    if (table) week.value = Math.max(1, Math.min(maxWeek.value, academicWeek(table.startDate)));
  },
  { immediate: true },
);
const periods = computed(() => Math.max(12, ...(props.timetable?.courses.map((c) => c.end) || [])));
const courses = computed(
  () => props.timetable?.courses.filter((c) => c.weeks.includes(week.value)) || [],
);
function onDay(day: number) {
  return courses.value.filter((c) => c.day === day);
}
const today = new Date().getDay() || 7;
const isCurrentWeek = computed(
  () => !!props.timetable && academicWeek(props.timetable.startDate) === week.value,
);
const editorOpen = ref(false);
/** The course the editor edits; `null` while adding a new one. */
const editing = ref<Course | null>(null);
const saving = ref(false);
const editError = ref('');
function openEditor(course: Course | null) {
  editing.value = course;
  editError.value = '';
  editorOpen.value = true;
}
// A cancelled save drops its error with it; only a restore failure stays on the
// page, because nothing else would show it.
function closeEditor() {
  editorOpen.value = false;
  editError.value = '';
}
function courseTitle(course: Course) {
  return [
    [course.name, course.location, course.teacher].filter(Boolean).join(' · '),
    course.custom ? '自建课程' : course.edited ? '已修改' : '',
    '点击修改',
  ]
    .filter(Boolean)
    .join(' · ');
}
// The backend applies the edit and returns the merged table, so the page never
// has to guess how an override interacts with the school's copy.
async function applyEdit(edit: Omit<TimetableEdit, 'semester'>) {
  if (saving.value || !props.timetable) return;
  saving.value = true;
  editError.value = '';
  try {
    const merged = await bridge.editTimetable({ semester: props.timetable.semester, ...edit });
    emit('updated', merged);
    editorOpen.value = false;
  } catch (error) {
    editError.value = String(error);
  } finally {
    saving.value = false;
  }
}
function restore(course: Course) {
  if (course.id) void applyEdit({ action: 'reset', id: course.id });
}
</script>
<template>
  <section class="desktop-page">
    <PageHeader :title="timetable?.semester || '课程表'">
      <template v-if="timetable" #description>
        第 {{ week }} 周 · 本周 {{ courses.length }} 节课
      </template>
      <template #actions>
        <UiButton v-if="timetable" variant="tinted" size="sm" @click="openEditor(null)">
          <Plus :size="14" />添加课程
        </UiButton>
        <div v-if="timetable" class="week-control">
          <UiIconButton label="上一周" size="sm" :disabled="week <= 1" @click="week--">
            <ChevronLeft :size="15" />
          </UiIconButton>
          <select v-model.number="week" class="week-select" aria-label="选择周次">
            <option v-for="n in maxWeek" :key="n" :value="n">第 {{ n }} 周</option>
          </select>
          <UiIconButton label="下一周" size="sm" :disabled="week >= maxWeek" @click="week++">
            <ChevronRight :size="15" />
          </UiIconButton>
        </div>
        <UiIconButton
          label="刷新课表"
          v-if="connected && timetable"
          title="刷新课表"
          :disabled="loading"
          @click="emit('refresh')"
        >
          <RefreshCw :size="15" :class="{ spinning: loading }" />
        </UiIconButton>
      </template>
    </PageHeader>
    <Transition name="drop">
      <p v-if="(error || (!editorOpen && editError)) && timetable" class="notice" role="alert">
        {{ error || editError }}
      </p>
    </Transition>
    <Transition name="fade" mode="out-in">
      <EmptyState
        v-if="!timetable"
        key="empty"
        :icon="loading ? LoaderCircle : CalendarDays"
        :busy="loading"
        :tone="error ? 'danger' : 'default'"
        :title="connected ? (loading ? '正在同步课表…' : '尚未同步课表') : '未登录校园账号'"
        :description="error || (connected ? '从校园系统获取本学期课表' : '登录校园账号后查看课表')"
      >
        <UiButton
          variant="primary"
          :disabled="loading"
          @click="connected ? emit('refresh') : emit('account')"
        >
          {{ connected ? '同步课表' : '前往校园账号' }}
        </UiButton>
      </EmptyState>
      <div v-else key="grid" class="schedule-scroll">
        <div
          class="timetable-grid"
          :style="{ '--periods': periods }"
          role="table"
          aria-label="每周课程"
        >
          <div class="period-column">
            <div class="day-heading"></div>
            <div v-for="period in periods" :key="period" class="period-number">{{ period }}</div>
          </div>
          <div
            v-for="(day, index) in days"
            :key="day"
            class="day-column"
            :class="{ today: index + 1 === today && isCurrentWeek }"
          >
            <div class="day-heading">{{ day }}</div>
            <Transition name="fade" mode="out-in">
              <div :key="week" class="day-courses">
                <button
                  v-for="(course, i) in onDay(index + 1)"
                  :key="course.id || course.name + i"
                  type="button"
                  class="course-cell"
                  :class="`hue-${course.color ?? courseHue(course.name)}`"
                  :style="{ gridRow: course.start + ' / span ' + (course.end - course.start + 1) }"
                  :title="courseTitle(course)"
                  @click="openEditor(course)"
                >
                  <strong>{{ course.name }}</strong
                  ><small>{{ course.location || '地点待定' }}</small
                  ><small v-if="course.teacher">{{ course.teacher }}</small>
                </button>
              </div>
            </Transition>
          </div>
        </div>
        <div v-if="timetable.hidden?.length" class="hidden-courses">
          <span class="hidden-label">已隐藏的课程</span>
          <button
            v-for="course in timetable.hidden"
            :key="course.id"
            type="button"
            class="hidden-course"
            :title="`恢复 ${course.name}`"
            :aria-label="`恢复 ${course.name}`"
            :disabled="saving"
            @click="restore(course)"
          >
            <span>{{ course.name }}</span
            ><RotateCcw :size="12" />
          </button>
        </div>
      </div>
    </Transition>
    <TimetableEditor
      v-if="editorOpen"
      :course="editing"
      :busy="saving"
      :error="editError"
      @submit="applyEdit"
      @cancel="closeEditor"
    />
  </section>
</template>
