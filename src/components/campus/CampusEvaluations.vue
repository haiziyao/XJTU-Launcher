<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { ClipboardCheck } from 'lucide-vue-next';
import SettingGroup from '../ui/SettingGroup.vue';
import SettingRow from '../ui/SettingRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import { bridge } from '../../bridge';
import type { EvaluationList, EvaluationOutcome, EvaluationPreview } from '../../types';

const list = ref<EvaluationList | null>(null);
const selected = ref<string[]>([]);
const rank = ref('2');
const comment = ref('');
const preview = ref<EvaluationPreview | null>(null);
const outcomes = ref<EvaluationOutcome[] | null>(null);
const loading = ref(false);
const submitting = ref(false);
const error = ref('');

const rankOptions = Array.from({ length: 5 }, (_, index) => ({
  value: String(index + 1),
  label: `第 ${index + 1} 档（1 为最高）`,
}));
const allSelected = computed(
  () => !!list.value?.rows.length && selected.value.length === list.value.rows.length,
);
const courseCount = computed(() => preview.value?.courses.length ?? 0);

async function load() {
  loading.value = true;
  error.value = '';
  try {
    list.value = (await bridge.schoolQuery({
      type: 'evaluations',
      finished: false,
    })) as EvaluationList;
    selected.value = list.value.rows.map((row) => row.id);
    outcomes.value = null;
  } catch (e) {
    console.error('[campus] evaluations list', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

function toggleAll() {
  selected.value = allSelected.value ? [] : (list.value?.rows ?? []).map((row) => row.id);
}

async function prepare() {
  if (!selected.value.length) return;
  loading.value = true;
  error.value = '';
  try {
    preview.value = await bridge.schoolPrepareEvaluations({
      ids: selected.value,
      rank: Number(rank.value),
      comment: comment.value.trim(),
    });
    outcomes.value = null;
  } catch (e) {
    console.error('[campus] evaluation preview', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function cancelPreview() {
  const current = preview.value;
  preview.value = null;
  if (current) {
    try {
      await bridge.schoolCancelEvaluations(current.id);
    } catch {
      // The preview expires on its own; failing to cancel locally is harmless.
    }
  }
}

async function submit() {
  if (!preview.value) return;
  const confirmed = await ask(`提交 ${courseCount.value} 份评教？提交后无法撤回。`, {
    title: '提交评教',
    kind: 'warning',
    okLabel: '提交',
    cancelLabel: '取消',
  });
  if (!confirmed) return;
  submitting.value = true;
  error.value = '';
  try {
    outcomes.value = await bridge.schoolSubmitEvaluations(preview.value.id);
    preview.value = null;
    await load();
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    submitting.value = false;
  }
}

const statusLabel = (status: string) =>
  ({
    submitted: '已提交',
    failed: '失败',
    skipped: '已跳过',
    cancelled: '已取消',
    unknown: '结果不明',
  })[status] ?? status;

onMounted(load);
onBeforeUnmount(() => {
  if (preview.value) void bridge.schoolCancelEvaluations(preview.value.id).catch(() => {});
});
</script>

<template>
  <p v-if="error" class="notice" role="alert">{{ error }}</p>
  <SettingGroup v-if="!preview" title="待评问卷">
    <EmptyState v-if="loading && !list" compact title="正在读取评教问卷…" />
    <EmptyState
      v-else-if="!list?.rows.length"
      compact
      :icon="ClipboardCheck"
      title="没有待评问卷"
      description="学校开放新的评教后，这里会显示对应课程。"
    />
    <template v-else>
      <label class="eval-row eval-head">
        <input
          type="checkbox"
          :checked="allSelected"
          aria-label="全选待评问卷"
          @change="toggleAll"
        />
        <span class="campus-record-main">
          <strong>{{ selected.length }} / {{ list.rows.length }} 份问卷</strong>
          <small>勾选要评的课程；预览不会提交</small>
        </span>
      </label>
      <label v-for="row in list.rows" :key="row.id" class="eval-row">
        <input v-model="selected" type="checkbox" :value="row.id" />
        <span class="campus-record-main">
          <strong>{{ row.course }}</strong>
          <small>{{
            [row.teacher, row.term, row.deadline && `截止 ${row.deadline}`]
              .filter(Boolean)
              .join(' · ')
          }}</small>
        </span>
      </label>
      <SettingRow title="评分模板" description="1 为最高档，预览会显示实际选项">
        <MenuSelect
          :model-value="rank"
          :options="rankOptions"
          label="评分模板"
          @update:model-value="rank = $event"
        />
      </SettingRow>
      <SettingRow title="文字意见" description="可选；必填文字题为空时不能提交">
        <textarea
          v-model="comment"
          class="text-field eval-comment"
          maxlength="4000"
          aria-label="文字意见"
        ></textarea>
      </SettingRow>
      <div class="campus-form-actions">
        <button class="btn primary" :disabled="loading || !selected.length" @click="prepare">
          {{ loading ? '生成预览…' : '预览答案' }}
        </button>
      </div>
    </template>
  </SettingGroup>

  <template v-else>
    <SettingGroup :title="`提交预览 · ${courseCount} 份`">
      <p class="eval-hint">预览不会提交。确认答案无误后，再点击“提交评教”；提交由你手动触发。</p>
      <div v-for="course in preview.courses" :key="course.questionnaire.id" class="eval-course">
        <strong>{{ course.questionnaire.course }} · {{ course.questionnaire.teacher }}</strong>
        <div v-for="answer in course.answers" :key="answer.id" class="eval-answer">
          <span>{{ answer.question }}</span>
          <span>{{ answer.answer }}</span>
        </div>
      </div>
      <div class="campus-form-actions">
        <button class="btn" :disabled="submitting" @click="cancelPreview">取消预览</button>
        <button class="btn primary" :disabled="submitting" @click="submit">
          {{ submitting ? '提交中…' : '提交评教' }}
        </button>
      </div>
    </SettingGroup>
  </template>

  <SettingGroup v-if="outcomes?.length" title="提交结果">
    <div v-for="row in outcomes" :key="row.questionnaireId" class="campus-record two">
      <span class="campus-record-main">
        <strong>{{ row.course }}</strong>
        <small>{{ row.message }}</small>
      </span>
      <span class="badge" :class="{ accent: row.status === 'submitted' }">{{
        statusLabel(row.status)
      }}</span>
    </div>
  </SettingGroup>
</template>

<style scoped>
.eval-row {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 16px;
  border-top: 1px solid var(--divider);
  cursor: pointer;
}
.eval-row input {
  margin: 0;
  accent-color: var(--accent);
}
.eval-head {
  cursor: default;
}
.eval-comment {
  height: 72px;
  padding: 8px 10px;
  resize: vertical;
}
.eval-hint {
  padding: 12px 16px;
  color: var(--text-secondary);
  font-size: 12.5px;
}
.eval-course {
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 16px;
  border-top: 1px solid var(--divider);
}
.eval-course strong {
  font-size: 13px;
  font-weight: 500;
}
.eval-answer {
  display: flex;
  justify-content: space-between;
  gap: 16px;
  color: var(--text-secondary);
  font-size: 12.5px;
}
.eval-answer span:last-child {
  color: var(--text);
  text-align: right;
}
</style>
