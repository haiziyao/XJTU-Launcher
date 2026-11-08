<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { open } from '@tauri-apps/plugin-dialog';
import { IconArrowRight, IconFolderOpen, IconUpload } from '@tabler/icons-vue';
import { fileSize } from '../domain';
import type { StoragePreview, StorageReport, StorageStatus } from '../types';
import { bridge } from '../bridge';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiButton from './ui/UiButton.vue';

/**
 * The one visible folder for the user's own files: `workspace` (资料库),
 * `plugin` (插件资源) and `download`. Settings, caches and credentials stay in
 * `~/.xjtu-launcher`.
 * Moving the folder copies first and only deletes the old files after the new
 * root has been recorded, so the dialog spells out what moves before asking,
 * and shows the leftover warnings afterwards.
 */
const status = ref<StorageStatus | null>(null);
const error = ref('');
const busy = ref(false);

type Step = 'closed' | 'confirm' | 'running' | 'done';
const step = ref<Step>('closed');
const target = ref('');
const preview = ref<StoragePreview | null>(null);
const report = ref<StorageReport | null>(null);
const understood = ref(false);

const total = computed(() => (status.value ? fileSize(status.value.totalBytes) : ''));
const folder = computed(() => status.value?.path ?? '');

async function load() {
  try {
    status.value = await bridge.storageStatus();
  } catch (e) {
    error.value = String(e);
  }
}
onMounted(() => void load());

function reset() {
  step.value = 'closed';
  target.value = '';
  preview.value = null;
  report.value = null;
  understood.value = false;
  error.value = '';
}
async function chooseFolder() {
  error.value = '';
  try {
    const picked = await open({ directory: true, multiple: false, title: '选择新的数据文件夹' });
    if (!picked || Array.isArray(picked)) return;
    target.value = picked;
    step.value = 'confirm';
    busy.value = true;
    try {
      preview.value = await bridge.storagePreview(picked);
    } catch (e) {
      preview.value = null;
      error.value = String(e);
    } finally {
      busy.value = false;
    }
  } catch (e) {
    error.value = String(e);
  }
}
async function migrate() {
  if (!understood.value || !preview.value || busy.value) return;
  step.value = 'running';
  error.value = '';
  busy.value = true;
  try {
    report.value = await bridge.storageMigrate(target.value);
    step.value = 'done';
    await load();
  } catch (e) {
    error.value = String(e);
    step.value = 'confirm';
  } finally {
    busy.value = false;
  }
}
async function reveal() {
  try {
    await bridge.storageReveal();
  } catch (e) {
    error.value = String(e);
  }
}
</script>

<template>
  <SettingGroup
    title="文件位置"
    footnote="设置、缓存与账号密码在 ~/.xjtu-launcher；这里放你自己的文件和插件资源。"
  >
    <div class="storage-folder">
      <IconUpload :size="16" aria-hidden="true" />
      <div class="storage-folder-copy">
        <code class="storage-path" :title="folder">{{ folder || '读取中…' }}</code>
        <small>
          {{ status?.isDefault ? '默认位置' : `你自己选择的位置 · 默认 ${status?.defaultPath}` }}
        </small>
      </div>
      <div class="row-actions">
        <UiButton size="sm" variant="ghost" :disabled="!status" @click="reveal">
          <IconFolderOpen :size="14" />打开
        </UiButton>
        <UiButton size="sm" :disabled="busy || !status" @click="chooseFolder">更改位置…</UiButton>
      </div>
    </div>
    <SettingRow
      title="占用空间"
      :description="status ? `${status.totalFiles} 个文件 · 下载默认保存到 download 文件夹` : ''"
    >
      <span class="setting-value">{{ total }}</span>
    </SettingRow>
    <div v-if="status?.entries.length" class="storage-breakdown">
      <div v-for="entry in status.entries" :key="entry.name" class="storage-entry">
        <span>{{ entry.label }}</span>
        <span>{{ entry.files ? fileSize(entry.bytes) : '空' }}</span>
      </div>
    </div>
    <Transition name="drop">
      <p v-if="error && step === 'closed'" class="notice storage-error" role="alert">{{ error }}</p>
    </Transition>

    <!-- Kept inside the group so this component has one root element; the
         settings page fades whole sections in and out. -->
    <Teleport to="body">
      <div v-if="step !== 'closed'" class="storage-backdrop" @click.self="!busy && reset()">
        <div class="storage-dialog" role="dialog" aria-modal="true" aria-labelledby="storage-title">
          <header class="storage-header">
            <h2 id="storage-title">更改文件位置</h2>
            <p>
              工作台（资料库）、插件资源与下载会一起搬到新位置，设置和缓存留在 ~/.xjtu-launcher。
            </p>
          </header>

          <template v-if="step === 'running'">
            <p class="storage-note">正在迁移，请不要关闭应用…</p>
          </template>

          <template v-else-if="step === 'done' && report">
            <p class="storage-note">
              已迁移 {{ report.moved.length }} 项文件，共 {{ fileSize(report.totalBytes) }}。
            </p>
            <ul v-if="report.warnings.length" class="storage-warnings">
              <li v-for="warning in report.warnings" :key="warning">{{ warning }}</li>
            </ul>
            <p class="storage-note">
              新位置：<code>{{ report.to }}</code>
            </p>
          </template>

          <template v-else-if="step === 'confirm'">
            <div class="storage-move">
              <code class="storage-path">{{ preview?.from || '读取中…' }}</code>
              <IconArrowRight :size="14" aria-hidden="true" />
              <!-- Shown as `~/...` by the backend, never as a user's own name.
                   The raw dialog path stays hidden until the preview returns. -->
              <code class="storage-path">{{ preview?.to || '读取中…' }}</code>
            </div>
            <div v-if="busy" class="storage-note">正在检查目标文件夹…</div>
            <template v-else-if="preview">
              <div class="storage-list">
                <div v-for="entry in preview.entries" :key="entry.name" class="storage-entry">
                  <span>{{ entry.label }}</span>
                  <span>{{ fileSize(entry.bytes) }}</span>
                </div>
              </div>
              <p class="storage-note">
                合计 {{ fileSize(preview.totalBytes) }} / {{ preview.totalFiles }} 个文件。
                迁移会先复制并核对，再切换位置，最后删除旧位置里的这些文件；本次启动的应用继续使用新位置。
              </p>
              <label class="storage-confirm">
                <input v-model="understood" type="checkbox" />
                <span>我了解迁移完成后，旧位置里的这些文件会被删除</span>
              </label>
              <p v-if="error" class="notice" role="alert">{{ error }}</p>
            </template>
            <p v-else-if="error" class="notice" role="alert">{{ error }}</p>
          </template>

          <footer class="storage-actions">
            <template v-if="step === 'done'">
              <UiButton variant="primary" size="sm" @click="reset">完成</UiButton>
            </template>
            <template v-else-if="step === 'confirm'">
              <UiButton size="sm" :disabled="busy" @click="reset">取消</UiButton>
              <UiButton
                variant="primary"
                size="sm"
                :disabled="busy || !understood || !preview"
                @click="migrate"
              >
                开始迁移
              </UiButton>
            </template>
            <template v-else-if="step !== 'running'">
              <UiButton size="sm" @click="reset">关闭</UiButton>
            </template>
          </footer>
        </div>
      </div>
    </Teleport>
  </SettingGroup>
</template>

<style scoped>
.storage-folder {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 16px;
  color: var(--text-tertiary);
  position: relative;
}
.storage-folder-copy {
  display: flex;
  flex: 1;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.storage-folder-copy > small {
  color: var(--text-tertiary);
  font-size: 12px;
}
.storage-path {
  display: block;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12.5px;
}
.storage-breakdown {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 2px 24px;
  padding: 4px 16px 16px;
}
.storage-entry {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  color: var(--text-secondary);
  font-size: 12px;
  line-height: 1.8;
}
.storage-error {
  margin: 0 16px 14px;
}
.storage-backdrop {
  position: fixed;
  z-index: 100;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.32);
}
.storage-dialog {
  display: flex;
  flex-direction: column;
  gap: 14px;
  width: 520px;
  max-width: 100%;
  padding: 20px 22px 18px;
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
}
.storage-header h2 {
  font-size: 14px;
  font-weight: 600;
}
.storage-header p,
.storage-note {
  color: var(--text-tertiary);
  font-size: 12px;
  line-height: 1.6;
  overflow-wrap: anywhere;
}
.storage-header p {
  margin-top: 2px;
}
.storage-move {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 10px 12px;
  border-radius: var(--radius);
  background: var(--surface-subtle);
}
.storage-move .storage-path {
  flex: 1;
  min-width: 0;
}
.storage-list {
  max-height: 220px;
  padding: 4px 12px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: var(--radius);
}
.storage-confirm {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.5;
}
.storage-confirm input {
  margin-top: 3px;
}
.storage-warnings {
  margin: 0;
  padding-left: 18px;
  color: var(--warning);
  font-size: 12px;
  line-height: 1.6;
}
.storage-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
</style>
