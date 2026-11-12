<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { ClipboardCheck } from 'lucide-vue-next';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import { bridge } from '../../../shared/bridge';
import type { EvaluationList, EvaluationOutcome, EvaluationPreview } from '../../../types';

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
const meta = computed(() =>
  list.value ? `${selected.value.length} / ${list.value.rows.length} 份已选` : '',
);
const answerRows = computed(
  () =>
    preview.value?.courses.flatMap((course) =>
      course.answers.map((answer, index) => ({
        key: `${course.questionnaire.id}:${answer.id}`,
        course: course.questionnaire.course,
        teacher: course.questionnaire.teacher,
        index: index + 1,
        question: answer.question,
        answer: answer.answer,
      })),
    ) ?? [],
);

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
const statusTone = (status: string) =>
  status === 'submitted'
    ? 'ok'
    : status === 'failed'
      ? 'danger'
      : status === 'unknown'
        ? 'warn'
        : 'muted';

onMounted(load);
onBeforeUnmount(() => {
  if (preview.value) void bridge.schoolCancelEvaluations(preview.value.id).catch(() => {});
});
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <button type="button" class="cv-btn" :disabled="loading" @click="load">刷新</button>
  </CampusHeaderSlot>

  <div
    class="cv-cols lcr"
    style="grid-template-columns: minmax(200px, 230px) minmax(0, 1fr) minmax(240px, 280px)"
  >
    <div class="cv-panel cv-list">
      <template v-if="!preview">
        <div class="cv-lh">
          待评 {{ list?.rows.length ?? 0 }}
          <button type="button" class="cv-link" style="margin-left: auto" @click="toggleAll">
            {{ allSelected ? '全不选' : '全选' }}
          </button>
        </div>
        <div v-if="loading && !list" class="cv-lh">正在读取评教问卷…</div>
        <div v-else-if="!list?.rows.length" class="cv-empty" style="min-height: 140px">
          <ClipboardCheck :size="20" /><b>没有待评问卷</b>
          <span>学校开放新的评教后，这里会显示对应课程。</span>
        </div>
        <label v-for="row in list?.rows ?? []" :key="row.id" class="ev-item">
          <input v-model="selected" type="checkbox" :value="row.id" />
          <b>{{ row.course }}</b>
          <span class="s">
            {{
              [row.teacher, row.term, row.deadline && `截止 ${row.deadline}`]
                .filter(Boolean)
                .join(' · ')
            }}
          </span>
        </label>
      </template>
      <template v-else>
        <div class="cv-lh">预览 {{ courseCount }} 份</div>
        <div v-for="course in preview.courses" :key="course.questionnaire.id" class="ev-item done">
          <b>{{ course.questionnaire.course }}</b>
          <span class="s">{{ course.questionnaire.teacher }} · 预览答案</span>
        </div>
      </template>
    </div>

    <div class="cv-center">
      <template v-if="preview">
        <div class="cv-ch">
          <b>预览答案</b>
          <span class="meta">{{ courseCount }} 份课程 · {{ answerRows.length }} 题</span>
        </div>
        <div class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 28px 1fr 128px">
            <span>#</span><span>题目</span><span class="cv-r">答案</span>
          </div>
          <template v-for="course in preview.courses" :key="course.questionnaire.id">
            <div class="cv-grp">
              {{ course.questionnaire.course }}
              <span class="n">{{ course.questionnaire.teacher }}</span>
            </div>
            <div
              v-for="(answer, index) in course.answers"
              :key="answer.id"
              class="cv-tr"
              style="--cols: 28px 1fr 128px"
            >
              <span class="cv-muted cv-num">{{ index + 1 }}</span>
              <span class="cv-ellip">{{ answer.question }}</span>
              <span class="cv-r cv-sec cv-ellip">{{ answer.answer }}</span>
            </div>
          </template>
        </div>
      </template>
      <template v-else>
        <div class="cv-ch">
          <b>待评问卷</b>
          <span class="meta">预览不会提交</span>
        </div>
        <div v-if="!list?.rows.length" class="cv-empty">
          <ClipboardCheck :size="20" /><b>没有待评问卷</b>
          <span>学校开放新的评教后，这里会显示对应课程。</span>
        </div>
        <div v-else class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 1fr 92px 92px 96px">
            <span>课程</span><span>教师</span><span>学期</span><span class="cv-r">截止</span>
          </div>
          <div
            v-for="row in list.rows"
            :key="row.id"
            class="cv-tr"
            style="--cols: 1fr 92px 92px 96px"
            :class="{ 'cv-muted': !selected.includes(row.id) }"
          >
            <span class="cv-ellip">{{ row.course }}</span>
            <span class="cv-sec cv-ellip">{{ row.teacher || '—' }}</span>
            <span class="cv-muted cv-ellip">{{ row.term || '—' }}</span>
            <span class="cv-r cv-num cv-muted">{{ row.deadline || '—' }}</span>
          </div>
        </div>
      </template>

      <div v-if="outcomes?.length" class="cv-panel" style="margin-top: 2px">
        <div class="cv-grp" style="position: static; top: auto">提交结果</div>
        <div
          v-for="row in outcomes"
          :key="row.questionnaireId"
          class="cv-tr"
          style="--cols: 1fr auto; height: auto; padding: 6px 12px"
        >
          <span class="cv-tt">
            <span class="cv-ellip">{{ row.course }}</span>
            <small>{{ row.message }}</small>
          </span>
          <span class="cv-badge" :class="statusTone(row.status)">{{
            statusLabel(row.status)
          }}</span>
        </div>
      </div>
    </div>

    <div class="cv-panel cv-detail">
      <template v-if="!preview">
        <div class="cv-type">评教设置</div>
        <dl class="cv-kv">
          <dt>已选</dt>
          <dd>{{ selected.length }} 门课程</dd>
          <dt>评分</dt>
          <dd>
            <select v-model="rank" class="cv-select" aria-label="评分模板">
              <option v-for="option in rankOptions" :key="option.value" :value="option.value">
                {{ option.label }}
              </option>
            </select>
          </dd>
        </dl>
        <div class="cv-blk">
          <h4>文字意见</h4>
          <textarea
            v-model="comment"
            class="ev-ta"
            maxlength="4000"
            placeholder="可选；必填文字题为空时不能提交"
            aria-label="文字意见"
          ></textarea>
        </div>
        <div class="cv-blk">
          <p>预览会显示实际选项，但不会提交。提交由你手动触发，提交后不能在应用内撤回。</p>
        </div>
        <div class="cv-foot">
          <button
            type="button"
            class="cv-btn primary"
            :disabled="loading || !selected.length"
            @click="prepare"
          >
            {{ loading ? '生成预览…' : `预览 ${selected.length} 门` }}
          </button>
        </div>
      </template>
      <template v-else>
        <div class="cv-type">
          提交预览
          <span class="cv-badge accent">未提交</span>
        </div>
        <h3>{{ courseCount }} 门课程</h3>
        <dl class="cv-kv">
          <dt>评分</dt>
          <dd>{{ rankOptions.find((option) => option.value === rank)?.label }}</dd>
          <dt>文字</dt>
          <dd>{{ comment.trim() || '未填写' }}</dd>
        </dl>
        <div class="cv-blk">
          <p>提交后不能在应用内撤回。结果不明时不会自动重试，请到评教系统核对。</p>
        </div>
        <div class="cv-foot">
          <button type="button" class="cv-btn primary" :disabled="submitting" @click="submit">
            {{ submitting ? '提交中…' : `确认提交 ${courseCount} 门` }}
          </button>
          <button type="button" class="cv-btn" :disabled="submitting" @click="cancelPreview">
            取消预览
          </button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.ev-item {
  display: grid;
  grid-template-columns: 16px 1fr;
  column-gap: 8px;
  align-items: center;
  padding: 6px 8px;
  border-radius: 6px;
  cursor: pointer;
}
.ev-item:hover {
  background: var(--surface-hover);
}
.ev-item input {
  margin: 0;
  accent-color: var(--accent);
}
.ev-item b {
  font-size: 12.5px;
  font-weight: 500;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ev-item .s {
  grid-column: 2;
  color: var(--text-tertiary);
  font-size: 11px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.ev-item.done {
  cursor: default;
  opacity: 0.7;
}
.ev-ta {
  height: 74px;
  padding: 8px 10px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font-size: 12px;
  line-height: 1.6;
  resize: vertical;
}
.ev-ta:focus-visible {
  outline: 2px solid var(--accent);
  outline-offset: 1px;
}
</style>
