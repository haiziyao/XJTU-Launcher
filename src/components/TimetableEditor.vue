<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { formatWeeks, parseWeeks, courseHue } from '../domain';
import type { Course, TimetableEdit } from '../types';
import UiButton from './ui/UiButton.vue';
/**
 * One course form, used both for adding a course and for editing a school one.
 * It only assembles the request; the page calls the backend and reports errors
 * back through `error`, so a failed save keeps the typed values.
 */
const props = defineProps<{
  /** The course being edited; `null` adds a new one. */
  course: Course | null;
  busy: boolean;
  error: string;
}>();
const emit = defineEmits<{
  submit: [edit: Omit<TimetableEdit, 'semester'>];
  cancel: [];
}>();
const name = ref(props.course?.name ?? '');
const teacher = ref(props.course?.teacher ?? '');
const location = ref(props.course?.location ?? '');
const day = ref(props.course?.day ?? 1);
const start = ref(props.course?.start ?? 1);
const end = ref(props.course?.end ?? 2);
const weeksText = ref(formatWeeks(props.course?.weeks ?? []));
const color = ref(props.course?.color ?? courseHue(props.course?.name ?? ''));
const confirming = ref<'delete' | 'reset' | null>(null);
// An untouched form is not an error yet; the first edit or save explains.
const touched = ref(false);
const dialog = ref<HTMLElement | null>(null);

const isCustom = computed(() => props.course?.custom === true);
const isEdited = computed(() => props.course?.edited === true);
const weeks = computed(() => parseWeeks(weeksText.value));
// The backend trims and validates the same way; catching it here keeps a
// rejected save from looking like a network problem.
const problem = computed(() => {
  if (!name.value.trim()) return '请填写课程名称';
  if (name.value.trim().length > 80) return '课程名称不能超过 80 个字';
  if (teacher.value.trim().length > 80) return '教师不能超过 80 个字';
  if (location.value.trim().length > 80) return '地点不能超过 80 个字';
  if (end.value < start.value) return '结束节次不能早于开始节次';
  return weeks.value.error;
});
const message = computed(() => (touched.value ? problem.value : '') || props.error);
const periods = Array.from({ length: 20 }, (_, index) => index + 1);
const weekdays = ['周一', '周二', '周三', '周四', '周五', '周六', '周日'];

// Shrinking the start below the end keeps a valid range without a second step.
watch(start, (value) => {
  if (end.value < value) end.value = value;
});
function save() {
  touched.value = true;
  if (props.busy || problem.value) return;
  emit('submit', {
    action: 'save',
    ...(props.course?.id ? { id: props.course.id } : {}),
    course: {
      name: name.value.trim(),
      teacher: teacher.value.trim(),
      location: location.value.trim(),
      day: day.value,
      start: start.value,
      end: end.value,
      weeks: weeks.value.weeks,
      color: color.value,
    },
  });
}
function hide() {
  if (props.course?.id) emit('submit', { action: 'hide', id: props.course.id });
}
function reset() {
  if (props.course?.id) emit('submit', { action: 'reset', id: props.course.id });
}
// Deleting or resetting cannot be undone, so the button asks once first.
function confirm() {
  const action = confirming.value;
  confirming.value = null;
  if (action === 'delete') hide();
  if (action === 'reset') reset();
}
function onKey(event: KeyboardEvent) {
  if (event.key !== 'Escape' || props.busy) return;
  if (confirming.value) confirming.value = null;
  else emit('cancel');
}
onMounted(async () => {
  window.addEventListener('keydown', onKey);
  await nextTick();
  dialog.value?.querySelector<HTMLInputElement>('input')?.focus();
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <Teleport to="body">
    <div class="editor-backdrop" @click.self="!busy && emit('cancel')">
      <form
        ref="dialog"
        class="editor-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="editor-title"
        @submit.prevent="save"
        @input="touched = true"
        @change="touched = true"
      >
        <header class="editor-header">
          <h2 id="editor-title">{{ course ? '编辑课程' : '添加课程' }}</h2>
          <p>
            {{
              course && !isCustom
                ? '修改只保存在本机，不会提交到学校。'
                : '这门课只出现在本机课表。'
            }}
          </p>
        </header>
        <div class="editor-grid">
          <label class="editor-field span-2">
            <span>课程名称</span>
            <input v-model="name" class="text-field" maxlength="80" placeholder="例如 高等数学" />
          </label>
          <label class="editor-field">
            <span>教师</span>
            <input v-model="teacher" class="text-field" maxlength="80" placeholder="可选" />
          </label>
          <label class="editor-field">
            <span>地点</span>
            <input v-model="location" class="text-field" maxlength="80" placeholder="可选" />
          </label>
          <label class="editor-field span-2">
            <span>周次</span>
            <input v-model="weeksText" class="text-field" placeholder="1-16" />
            <small>支持 1-16、1-8,10-16 与 1-8单，最多 60 周</small>
          </label>
          <div class="editor-field span-2">
            <span>时间</span>
            <div class="editor-inline">
              <select v-model.number="day" class="text-field" aria-label="星期">
                <option v-for="(label, index) in weekdays" :key="label" :value="index + 1">
                  {{ label }}
                </option>
              </select>
              <select v-model.number="start" class="text-field" aria-label="开始节次">
                <option v-for="period in periods" :key="period" :value="period">
                  第 {{ period }} 节
                </option>
              </select>
              <span class="editor-to">到</span>
              <select v-model.number="end" class="text-field" aria-label="结束节次">
                <option v-for="period in periods" :key="period" :value="period">
                  第 {{ period }} 节
                </option>
              </select>
            </div>
          </div>
          <div class="editor-field span-2">
            <span>颜色</span>
            <div class="editor-colors" role="radiogroup" aria-label="课程颜色">
              <button
                v-for="hue in 6"
                :key="hue"
                type="button"
                class="swatch"
                :class="[`hue-${hue - 1}`, { selected: color === hue - 1 }]"
                role="radio"
                :aria-checked="color === hue - 1"
                :aria-label="`颜色 ${hue}`"
                @click="color = hue - 1"
              ></button>
            </div>
          </div>
        </div>
        <p v-if="message" class="editor-error" role="alert">{{ message }}</p>
        <div v-if="confirming" class="editor-confirm">
          <span>{{
            confirming === 'delete'
              ? '删除后无法恢复这门自建课程。'
              : '这会清除本机对这门课的修改，恢复学校的原始内容。'
          }}</span>
          <UiButton type="button" size="sm" :disabled="busy" @click="confirming = null">
            取消
          </UiButton>
          <UiButton type="button" size="sm" danger :disabled="busy" @click="confirm">{{
            busy ? '正在处理…' : '确认'
          }}</UiButton>
        </div>
        <footer v-else class="editor-actions">
          <div class="editor-left">
            <UiButton
              v-if="course && isCustom"
              type="button"
              size="sm"
              danger
              @click="confirming = 'delete'"
            >
              删除
            </UiButton>
            <UiButton v-else-if="course" type="button" size="sm" :disabled="busy" @click="hide">
              隐藏
            </UiButton>
            <UiButton
              v-if="course && isEdited"
              type="button"
              size="sm"
              variant="ghost"
              :disabled="busy"
              @click="confirming = 'reset'"
            >
              恢复默认
            </UiButton>
          </div>
          <div class="editor-right">
            <UiButton type="button" size="sm" :disabled="busy" @click="emit('cancel')">
              取消
            </UiButton>
            <UiButton type="submit" variant="primary" size="sm" :disabled="busy">
              {{ busy ? '正在保存…' : '保存' }}
            </UiButton>
          </div>
        </footer>
      </form>
    </div>
  </Teleport>
</template>

<style scoped>
.editor-backdrop {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.32);
}
.editor-dialog {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: 420px;
  max-width: 100%;
  padding: 20px 22px 18px;
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.editor-header h2 {
  font-size: 14px;
  font-weight: 600;
}
.editor-header p {
  margin-top: 2px;
  color: var(--text-tertiary);
  font-size: 12px;
}
.editor-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px 12px;
}
.editor-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
  color: var(--text-secondary);
  font-size: 12px;
}
.editor-field small {
  color: var(--text-tertiary);
  font-size: 11px;
}
.editor-field.span-2 {
  grid-column: span 2;
}
.editor-inline {
  display: flex;
  align-items: center;
  gap: 6px;
}
.editor-inline .text-field {
  flex: 1;
  min-width: 0;
}
.editor-to {
  color: var(--text-tertiary);
}
.editor-colors {
  display: flex;
  gap: 8px;
}
.swatch {
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--swatch);
  cursor: pointer;
}
.swatch.hue-0 {
  --swatch: #1f6fe5;
}
.swatch.hue-1 {
  --swatch: #2c9a5c;
}
.swatch.hue-2 {
  --swatch: #7a55e3;
}
.swatch.hue-3 {
  --swatch: #d9822b;
}
.swatch.hue-4 {
  --swatch: #1a9aa5;
}
.swatch.hue-5 {
  --swatch: #d9506a;
}
.swatch.selected {
  box-shadow:
    0 0 0 2px var(--surface),
    0 0 0 4px var(--accent);
}
.swatch:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.editor-error {
  color: var(--danger);
  font-size: 12px;
}
.editor-confirm {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.editor-confirm span {
  flex: 1;
  color: var(--text-secondary);
}
.editor-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.editor-left,
.editor-right {
  display: flex;
  gap: 8px;
}
.editor-right {
  margin-left: auto;
}
</style>
