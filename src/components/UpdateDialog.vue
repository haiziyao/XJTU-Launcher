<script setup lang="ts">
import { computed } from 'vue';
import { X } from 'lucide-vue-next';
import { bridge } from '../bridge';
import { renderMarkdown } from '../markdown';
import { updateStatus } from '../updates';
import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{ close: [] }>();

const status = computed(() => updateStatus.value);
const busy = computed(() => ['downloading', 'installing'].includes(status.value?.phase ?? ''));
const percent = computed(() =>
  status.value?.total
    ? Math.min(100, Math.round((status.value.received / status.value.total) * 100))
    : undefined,
);
const notes = computed(() =>
  status.value?.notes ? renderMarkdown(status.value.notes) : '<p>此版本未提供更新说明。</p>',
);
const progress = computed(() => {
  if (status.value?.phase === 'installing') return '正在安装…';
  if (status.value?.phase === 'downloading') {
    return percent.value === undefined ? '正在下载…' : `正在下载 ${percent.value}%`;
  }
  return '';
});

async function install() {
  try {
    await bridge.installUpdate();
  } catch {
    /* The shared status carries the message into the dialog. */
  }
}
async function restart() {
  await bridge.restartUpdatedApp().catch(() => undefined);
}
function noteClick(event: MouseEvent) {
  const anchor = (event.target as Element | null)?.closest('a');
  if (!(anchor instanceof HTMLAnchorElement)) return;
  event.preventDefault();
  void bridge.openUrl(anchor.href);
}
</script>

<template>
  <Teleport to="body">
    <div v-if="props.open" class="update-backdrop" @click.self="emit('close')">
      <section class="update-dialog" role="dialog" aria-modal="true" aria-labelledby="update-title">
        <header class="update-head">
          <div>
            <p class="update-kicker">发现新版本</p>
            <h2 id="update-title">XJTU Launcher {{ status?.version || '' }}</h2>
            <p class="update-current">当前版本 {{ status?.currentVersion || '—' }}</p>
          </div>
          <UiIconButton label="关闭" @click="emit('close')"><X :size="16" /></UiIconButton>
        </header>
        <div class="update-notes" @click="noteClick" v-html="notes"></div>
        <template v-if="busy">
          <progress :value="percent" max="100" aria-label="更新进度" />
          <p class="update-progress" role="status">{{ progress }}</p>
        </template>
        <p v-if="status?.message" class="notice" role="alert">{{ status.message }}</p>
        <footer class="update-actions">
          <UiButton @click="emit('close')">稍后</UiButton>
          <UiButton v-if="status?.phase === 'installed'" variant="primary" @click="restart">
            重新启动
          </UiButton>
          <UiButton
            v-else
            variant="primary"
            :disabled="busy || !status?.version || status?.phase === 'unconfigured'"
            @click="install"
          >
            {{ busy ? '正在更新…' : '下载并更新' }}
          </UiButton>
        </footer>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.update-backdrop {
  position: fixed;
  z-index: 120;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.32);
}
.update-dialog {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: 480px;
  max-width: 100%;
  max-height: calc(100vh - 64px);
  padding: 20px 22px 18px;
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.update-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.update-kicker {
  margin: 0;
  color: var(--accent-text);
  font-size: 11px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.update-head h2 {
  margin: 4px 0 2px;
  font-size: 17px;
  font-weight: 650;
}
.update-current {
  margin: 0;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.update-notes {
  max-height: 46vh;
  overflow-y: auto;
  padding-right: 4px;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.7;
}
.update-notes :deep(h1),
.update-notes :deep(h2),
.update-notes :deep(h3) {
  margin: 14px 0 6px;
  color: var(--text);
  font-size: 13px;
  font-weight: 600;
}
.update-notes :deep(h1:first-child),
.update-notes :deep(h2:first-child),
.update-notes :deep(h3:first-child) {
  margin-top: 0;
}
.update-notes :deep(p) {
  margin: 8px 0;
}
.update-notes :deep(ul),
.update-notes :deep(ol) {
  margin: 8px 0;
  padding-left: 18px;
}
.update-notes :deep(li) {
  margin: 3px 0;
}
.update-notes :deep(code) {
  padding: 1px 5px;
  border-radius: var(--radius-xs);
  background: var(--surface-sunken);
  font-family: var(--font-mono);
  font-size: 11.5px;
}
.update-notes :deep(a) {
  color: var(--accent-text);
}
progress {
  width: 100%;
  height: 5px;
  accent-color: var(--accent);
}
.update-progress {
  margin: 0;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.update-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
</style>
