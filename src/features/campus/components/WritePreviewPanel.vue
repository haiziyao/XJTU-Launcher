<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { CircleAlert, CircleCheck } from 'lucide-vue-next';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import UiButton from '../../../components/ui/UiButton.vue';
import { fileSize } from '../../../shared/domain';
import type { WriteOutcome, WritePreview } from '../../../types';

/**
 * Preview and confirm every school write (LAYOUT §7, SPEC §4.5). The panel never
 * submits on its own: it shows exactly what will be sent, disables the button
 * when the preview expires, and for an `unknown` result offers only “查看提交
 * 记录” — no retry, because the school may already have the submission.
 */
const props = withDefaults(
  defineProps<{
    preview: WritePreview | null;
    busy?: boolean;
    result?: WriteOutcome | null;
    /** Extra line under the title (deadline + countdown). */
    context?: string;
    cancelLabel?: string;
    confirmLabel?: string;
    /** Fixed clock for tests; omitted means ticking real time. */
    now?: number;
  }>(),
  {
    busy: false,
    result: null,
    context: '',
    cancelLabel: '返回修改',
    confirmLabel: '确认提交',
    now: 0,
  },
);
const emit = defineEmits<{
  cancel: [];
  confirm: [captcha: string | null];
  viewRecords: [];
  done: [];
}>();

const startedAt = ref(Date.now());
const clock = ref(props.now || Date.now());
const captcha = ref('');
let timer: ReturnType<typeof setInterval> | undefined;

watch(
  () => props.preview?.id,
  () => {
    startedAt.value = Date.now();
    captcha.value = '';
    clock.value = props.now || Date.now();
  },
  { immediate: true },
);
if (!props.now) {
  timer = setInterval(() => (clock.value = Date.now()), 1000);
}
onBeforeUnmount(() => {
  if (timer) clearInterval(timer);
});

const expiresIn = computed(() => props.preview?.expiresInSeconds ?? 300);
const remaining = computed(() => {
  if (!props.preview) return 0;
  const end = startedAt.value + expiresIn.value * 1000;
  return Math.max(0, Math.round((end - clock.value) / 1000));
});
const expired = computed(() => remaining.value <= 0);
const soon = computed(() => !expired.value && remaining.value <= 60);
const expiryText = computed(() => {
  if (expired.value) return '预览已过期，请重新生成。';
  if (soon.value) return '预览即将过期。';
  return '';
});

const destructive = computed(() => props.preview?.irreversible ?? false);
const captchaKind = computed(() => props.preview?.captcha?.kind ?? null);

function confirm() {
  if (!props.preview || expired.value || props.busy) return;
  emit('confirm', captcha.value.trim() || null);
}
</script>

<template>
  <section class="write-panel" aria-labelledby="write-title">
    <!-- Result replaces the form; the same panel is the only place a write reports back. -->
    <template v-if="result">
      <header class="write-head">
        <h2 id="write-title">{{ preview?.title || '操作结果' }}</h2>
      </header>
      <div class="write-body">
        <div v-if="result.status === 'done'" class="write-result done">
          <CircleCheck :size="26" aria-hidden="true" />
          <strong>已完成</strong>
          <p>{{ result.message }}</p>
          <ul v-if="preview?.files?.length" class="write-files">
            <li v-for="file in preview.files" :key="file.name">
              <span>{{ file.name }}</span
              ><span>{{ fileSize(file.size) }}</span>
            </li>
          </ul>
        </div>
        <InlineAlert v-else-if="result.status === 'rejected'" tone="error" title="操作被拒绝">
          {{ result.message }}
        </InlineAlert>
        <InlineAlert v-else tone="warning" title="提交结果未知">
          提交结果未知，请先查看提交记录确认，确认前不要重复提交。
          <template v-if="result.message"><br />{{ result.message }}</template>
        </InlineAlert>
      </div>
      <footer class="write-foot">
        <template v-if="result.status === 'done'">
          <UiButton @click="emit('viewRecords')">查看提交记录</UiButton>
          <UiButton variant="primary" @click="emit('done')">完成</UiButton>
        </template>
        <template v-else-if="result.status === 'unknown'">
          <UiButton variant="primary" @click="emit('viewRecords')">查看提交记录</UiButton>
        </template>
        <template v-else>
          <UiButton @click="emit('cancel')">返回修改</UiButton>
        </template>
      </footer>
    </template>

    <template v-else-if="preview">
      <header class="write-head">
        <p class="write-kicker">预览</p>
        <h2 id="write-title">{{ preview.title }}</h2>
        <p v-if="context" class="write-context">{{ context }}</p>
      </header>

      <div class="write-body">
        <dl class="write-summary">
          <template v-for="entry in preview.summary" :key="entry.label">
            <dt>{{ entry.label }}</dt>
            <dd>{{ entry.value }}</dd>
          </template>
        </dl>

        <div v-if="preview.files?.length" class="write-attachments">
          <p class="write-section-label">文件</p>
          <ul class="write-files">
            <li v-for="file in preview.files" :key="file.name">
              <span class="write-file-name">{{ file.name }}</span>
              <span class="write-file-size">{{ fileSize(file.size) }}</span>
            </li>
          </ul>
          <p class="write-total">
            合计 {{ preview.files.length }} 个文件 ·
            {{ fileSize(preview.files.reduce((sum, file) => sum + file.size, 0)) }}
          </p>
        </div>

        <InlineAlert
          v-for="warning in preview.warnings"
          :key="warning"
          tone="warning"
          :title="warning"
        />

        <InlineAlert tone="info">预览不会上传或提交，确认后只提交一次。</InlineAlert>

        <p v-if="destructive" class="write-note">提交后能否撤回以学校设置为准。</p>
      </div>

      <footer class="write-foot write-confirm">
        <div v-if="preview.captcha" class="write-captcha">
          <img :src="preview.captcha.image" alt="验证码" class="write-captcha-image" />
          <input
            v-if="captchaKind === 'text'"
            v-model="captcha"
            class="text-field"
            placeholder="输入验证码"
            aria-label="验证码"
          />
          <p v-else class="write-note">请按住滑块完成验证。</p>
        </div>
        <div class="write-actions">
          <div class="write-expiry">
            <span v-if="expiryText" :class="{ danger: expired }">{{ expiryText }}</span>
          </div>
          <UiButton :disabled="busy" @click="emit('cancel')">{{ cancelLabel }}</UiButton>
          <UiButton
            variant="primary"
            :danger="destructive"
            :disabled="busy || expired"
            @click="confirm"
          >
            {{ busy ? '正在提交…' : confirmLabel }}
          </UiButton>
        </div>
      </footer>
    </template>

    <div v-else class="write-body">
      <p class="write-note">没有待确认的操作。</p>
    </div>
  </section>
</template>

<style scoped>
.write-panel {
  display: flex;
  flex-direction: column;
  min-height: 0;
  max-height: 100%;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-sm);
  overflow: hidden;
}
.write-head {
  padding: 16px 18px 12px;
  border-bottom: 1px solid var(--divider);
}
.write-kicker {
  margin-bottom: 2px;
  color: var(--text-tertiary);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.write-head h2 {
  font-size: 17px;
  font-weight: 600;
  line-height: 1.35;
}
.write-context {
  margin-top: 4px;
  color: var(--text-secondary);
  font-size: 12px;
}
.write-body {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 12px;
  min-height: 0;
  padding: 16px 18px;
  overflow: auto;
}
.write-summary {
  display: grid;
  grid-template-columns: minmax(56px, auto) 1fr;
  gap: 8px 14px;
  margin: 0;
  font-size: 12.5px;
}
.write-summary dt {
  color: var(--text-tertiary);
}
.write-summary dd {
  margin: 0;
  color: var(--text);
  overflow-wrap: anywhere;
}
.write-section-label {
  margin-bottom: 6px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.write-files {
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}
.write-files li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  min-height: 32px;
  padding: 6px 10px;
  font-size: 12px;
}
.write-files li + li {
  border-top: 1px solid var(--divider);
}
.write-file-name {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.write-file-size {
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
  white-space: nowrap;
}
.write-total {
  margin-top: 6px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.write-note {
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.6;
}
.write-foot {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 8px;
  padding: 12px 18px;
  border-top: 1px solid var(--border);
  background: var(--surface-subtle);
}
.write-confirm {
  flex-direction: column;
  align-items: stretch;
  gap: 10px;
}
.write-captcha {
  display: flex;
  align-items: center;
  gap: 10px;
}
.write-captcha-image {
  height: 34px;
  border: 1px solid var(--border);
  border-radius: var(--radius-xs);
  background: #fff;
}
.write-captcha .text-field {
  flex: 1;
}
.write-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}
.write-expiry {
  flex: 1;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.write-expiry .danger {
  color: var(--danger);
}
.write-result {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 6px;
  padding: 18px 12px 8px;
  text-align: center;
}
.write-result.done {
  color: var(--success-text);
}
.write-result strong {
  font-size: 14px;
  font-weight: 600;
}
.write-result p {
  color: var(--text-secondary);
  font-size: 12.5px;
}
.write-result .write-files {
  width: 100%;
  margin-top: 8px;
  color: var(--text);
  text-align: left;
}
</style>
