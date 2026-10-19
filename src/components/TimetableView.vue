<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { CalendarDays, ChevronLeft, ChevronRight, LoaderCircle, RefreshCw } from 'lucide-vue-next';
import { academicWeek } from '../domain';
import type { Timetable } from '../types';
import PageHeader from './ui/PageHeader.vue';
import EmptyState from './ui/EmptyState.vue';
const props = defineProps<{
  timetable: Timetable | null;
  connected: boolean;
  loading: boolean;
  error: string;
}>();
const emit = defineEmits<{ refresh: []; account: [] }>();
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
// Same course always gets the same soft color.
function hue(name: string) {
  let sum = 0;
  for (const ch of name) sum = (sum + ch.charCodeAt(0)) % 997;
  return sum % 6;
}
</script>
<template>
  <section class="desktop-page">
    <PageHeader :title="timetable?.semester || '课程表'">
      <template v-if="timetable" #description>
        第 {{ week }} 周 · 本周 {{ courses.length }} 节课
      </template>
      <template #actions>
        <div v-if="timetable" class="week-control">
          <button
            class="btn ghost icon small"
            aria-label="上一周"
            :disabled="week <= 1"
            @click="week--"
          >
            <ChevronLeft :size="15" />
          </button>
          <select v-model.number="week" class="week-select" aria-label="选择周次">
            <option v-for="n in maxWeek" :key="n" :value="n">第 {{ n }} 周</option>
          </select>
          <button
            class="btn ghost icon small"
            aria-label="下一周"
            :disabled="week >= maxWeek"
            @click="week++"
          >
            <ChevronRight :size="15" />
          </button>
        </div>
        <button
          v-if="connected && timetable"
          class="btn ghost icon"
          title="刷新课表"
          aria-label="刷新课表"
          :disabled="loading"
          @click="emit('refresh')"
        >
          <RefreshCw :size="15" :class="{ spinning: loading }" />
        </button>
      </template>
    </PageHeader>
    <Transition name="drop">
      <p v-if="error && timetable" class="notice" role="alert">{{ error }}</p>
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
        <button
          class="btn primary"
          :disabled="loading"
          @click="connected ? emit('refresh') : emit('account')"
        >
          {{ connected ? '同步课表' : '前往校园账号' }}
        </button>
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
                <article
                  v-for="(course, i) in onDay(index + 1)"
                  :key="course.name + i"
                  class="course-cell"
                  :class="`hue-${hue(course.name)}`"
                  :style="{ gridRow: course.start + ' / span ' + (course.end - course.start + 1) }"
                  :title="
                    [course.name, course.location, course.teacher].filter(Boolean).join(' · ')
                  "
                >
                  <strong>{{ course.name }}</strong
                  ><small>{{ course.location || '地点待定' }}</small
                  ><small v-if="course.teacher">{{ course.teacher }}</small>
                </article>
              </div>
            </Transition>
          </div>
        </div>
      </div>
    </Transition>
  </section>
</template>
