<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import {
  Download,
  ExternalLink,
  FolderArchive,
  FolderOpen,
  Save,
  Trash2,
  Upload,
} from 'lucide-vue-next';
import SectionHeader from '../ui/SectionHeader.vue';
import FilterBar from '../ui/FilterBar.vue';
import SplitView from '../ui/SplitView.vue';
import ListItem from '../ui/ListItem.vue';
import IconTile from '../ui/IconTile.vue';
import FileRow from './FileRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import UiButton from '../ui/UiButton.vue';
import MenuSelect from '../ui/MenuSelect.vue';
import { bridge } from '../../bridge';
import { fileSize, updatedInfo } from '../../domain';
import { fileKind } from '../../fileType';
import type { LocalLibraryEntry } from '../../types';

/**
 * 资料库 is the user's own local quick-storage, not the school's course
 * material page (课程资料). Files are copied into the app data folder by the
 * backend, isolated per campus account. Sync and sharing have no server yet and
 * are intentionally absent from this screen.
 */
const entries = ref<LocalLibraryEntry[]>([]);
const loading = ref(true);
const importing = ref(false);
const saving = ref(false);
const action = ref('');
const error = ref('');
const notice = ref('');
const importErrors = ref<string[]>([]);
const query = ref('');
const tag = ref('');
const selectedId = ref<string | null>(null);
const loadedAt = ref<number | null>(null);
const listWidth = ref(320);
const draft = ref({ name: '', course: '', assignment: '', tags: '', note: '' });

const selected = computed(
  () => entries.value.find((entry) => entry.id === selectedId.value) ?? null,
);
const subtitle = computed(() => {
  if (!entries.value.length) return '';
  const total = entries.value.reduce((sum, entry) => sum + entry.size, 0);
  return `${entries.value.length} 个文件 · ${fileSize(total)}`;
});
const tagOptions = computed(() => {
  const tags = [...new Set(entries.value.flatMap((entry) => entry.tags))].sort((left, right) =>
    left.localeCompare(right, 'zh-Hans-CN'),
  );
  return [{ value: '', label: '全部标签' }, ...tags.map((value) => ({ value, label: value }))];
});
const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  return entries.value.filter((entry) => {
    if (tag.value && !entry.tags.includes(tag.value)) return false;
    if (!keyword) return true;
    const haystack = `${entry.name} ${entry.course} ${entry.assignment} ${entry.tags.join(' ')} ${entry.note}`;
    return haystack.toLowerCase().includes(keyword);
  });
});
const draftTags = computed(() => parseTags(draft.value.tags));
const locked = computed(() => saving.value || Boolean(action.value));
const dirty = computed(() => {
  const entry = selected.value;
  if (!entry) return false;
  return (
    draft.value.name.trim() !== entry.name ||
    draft.value.course.trim() !== entry.course ||
    draft.value.assignment.trim() !== entry.assignment ||
    draft.value.note.trim() !== entry.note ||
    draftTags.value.join('\n') !== entry.tags.join('\n')
  );
});

function parseTags(text: string): string[] {
  const tags: string[] = [];
  for (const part of text.split(/[,，、\n]+/)) {
    const value = part.trim();
    if (value && !tags.includes(value)) tags.push(value);
    if (tags.length === 8) break;
  }
  return tags;
}

function fillDraft(entry: LocalLibraryEntry) {
  draft.value = {
    name: entry.name,
    course: entry.course,
    assignment: entry.assignment,
    tags: entry.tags.join('，'),
    note: entry.note,
  };
}

function detailMeta(entry: LocalLibraryEntry): string {
  return [entry.course, entry.assignment, ...entry.tags.map((value) => `#${value}`)]
    .filter(Boolean)
    .join(' · ');
}

function baseName(path: string): string {
  return path.split(/[\\/]/).pop() || path;
}

function message(value: unknown): string {
  return value instanceof Error ? value.message : String(value);
}

async function load(showSpinner = true) {
  if (showSpinner) loading.value = true;
  error.value = '';
  try {
    const list = await bridge.libraryList();
    entries.value = list;
    loadedAt.value = Date.now();
    if (selectedId.value && !list.some((entry) => entry.id === selectedId.value)) {
      selectedId.value = null;
    }
    if (!selectedId.value && list.length) {
      selectedId.value = list[0].id;
      fillDraft(list[0]);
    } else if (selected.value && !dirty.value) {
      // A refresh keeps unsaved edits, but otherwise follows the saved record.
      fillDraft(selected.value);
    }
    if (tag.value && !tagOptions.value.some((option) => option.value === tag.value)) {
      tag.value = '';
    }
  } catch (e) {
    console.error('[campus] local library list', e);
    error.value = message(e);
  } finally {
    loading.value = false;
  }
}

async function choose(id: string) {
  if (id === selectedId.value) return;
  if (dirty.value) {
    const discard = await ask('当前修改尚未保存，切换文件后会丢失。', {
      title: '未保存的修改',
      kind: 'warning',
      okLabel: '丢弃修改',
      cancelLabel: '继续编辑',
    });
    if (!discard) return;
  }
  selectedId.value = id;
  if (selected.value) fillDraft(selected.value);
}

async function addFiles() {
  if (importing.value) return;
  error.value = '';
  notice.value = '';
  importErrors.value = [];
  importing.value = true;
  try {
    const results = await bridge.libraryAdd();
    if (!results.length) return;
    const added = results.filter((result) => result.entry && !result.duplicate);
    const duplicated = results.filter((result) => result.duplicate);
    const failed = results.filter((result) => result.error);
    await load(false);
    const first = added[0]?.entry;
    if (first) {
      selectedId.value = first.id;
      fillDraft(first);
    }
    const parts: string[] = [];
    if (added.length) parts.push(`已添加 ${added.length} 个文件`);
    if (duplicated.length) parts.push(`${duplicated.length} 个文件已存在，未重复保存`);
    if (failed.length) parts.push(`${failed.length} 个文件未能添加`);
    notice.value = parts.join('；');
    importErrors.value = failed.map((result) => `${baseName(result.path)}：${result.error}`);
  } catch (e) {
    console.error('[campus] local library import', e);
    error.value = message(e);
  } finally {
    importing.value = false;
  }
}

async function saveDraft() {
  const entry = selected.value;
  if (!entry || saving.value || action.value) return;
  error.value = '';
  notice.value = '';
  const name = draft.value.name.trim();
  if (!name) {
    error.value = '文件名不能为空';
    return;
  }
  saving.value = true;
  try {
    const updated = await bridge.libraryUpdate(entry.id, {
      name,
      course: draft.value.course.trim(),
      assignment: draft.value.assignment.trim(),
      tags: draftTags.value,
      note: draft.value.note.trim(),
    });
    entries.value = entries.value.map((item) => (item.id === updated.id ? updated : item));
    fillDraft(updated);
    notice.value = '已保存文件信息';
  } catch (e) {
    console.error('[campus] local library update', e);
    error.value = message(e);
  } finally {
    saving.value = false;
  }
}

async function openEntry() {
  const entry = selected.value;
  if (!entry || locked.value) return;
  error.value = '';
  notice.value = '';
  action.value = 'open';
  try {
    await bridge.libraryOpen(entry.id);
  } catch (e) {
    console.error('[campus] local library open', e);
    error.value = message(e);
  } finally {
    action.value = '';
  }
}

async function revealEntry() {
  const entry = selected.value;
  if (!entry || locked.value) return;
  error.value = '';
  notice.value = '';
  action.value = 'reveal';
  try {
    await bridge.libraryReveal(entry.id);
  } catch (e) {
    console.error('[campus] local library reveal', e);
    error.value = message(e);
  } finally {
    action.value = '';
  }
}

async function exportEntry() {
  const entry = selected.value;
  if (!entry || locked.value) return;
  error.value = '';
  notice.value = '';
  action.value = 'export';
  try {
    const saved = await bridge.libraryExport(entry.id, entry.name);
    if (saved) notice.value = `已另存为 ${saved.display}（${fileSize(saved.bytes)}）`;
  } catch (e) {
    console.error('[campus] local library export', e);
    error.value = message(e);
  } finally {
    action.value = '';
  }
}

async function deleteEntry() {
  const entry = selected.value;
  if (!entry || locked.value) return;
  const confirmed = await ask(`将从本机资料库删除“${entry.name}”，此操作不可恢复。`, {
    title: '删除本地文件',
    kind: 'warning',
    okLabel: '删除',
    cancelLabel: '取消',
  });
  if (!confirmed) return;
  error.value = '';
  notice.value = '';
  action.value = 'delete';
  try {
    await bridge.libraryDelete(entry.id);
    selectedId.value = null;
    await load(false);
    notice.value = '已从资料库删除该文件';
  } catch (e) {
    console.error('[campus] local library delete', e);
    error.value = message(e);
  } finally {
    action.value = '';
  }
}

onMounted(() => void load());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="资料库"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load()"
    >
      <template #actions>
        <UiButton size="sm" :disabled="importing" @click="addFiles">
          <Upload :size="14" />
          <span>{{ importing ? '正在添加…' : '添加文件' }}</span>
        </UiButton>
      </template>
    </SectionHeader>

    <FilterBar
      :query="query"
      :active="Boolean(tag)"
      placeholder="搜索文件名、课程或标签"
      label="搜索资料库"
      @update:query="query = $event"
      @clear="
        query = '';
        tag = '';
      "
    >
      <MenuSelect
        :model-value="tag"
        :options="tagOptions"
        label="按标签筛选"
        placeholder="全部标签"
        @update:model-value="tag = $event"
      />
    </FilterBar>

    <InlineAlert v-if="error" tone="error" :title="error" dismissible @dismiss="error = ''" />
    <InlineAlert v-if="importErrors.length" tone="error" title="部分文件未能添加">
      {{ importErrors.join('；') }}
    </InlineAlert>
    <InlineAlert v-if="notice" tone="info" :title="notice" dismissible @dismiss="notice = ''" />

    <EmptyState
      v-if="loading && !entries.length"
      :icon="FolderArchive"
      compact
      busy
      title="正在读取本地资料库…"
    />
    <EmptyState
      v-else-if="!entries.length"
      :icon="FolderArchive"
      :title="error ? '未能读取资料库' : '资料库还是空的'"
      :description="error ? '' : '添加作业、课件或笔记，文件会保存在本机，并按校园账号隔离。'"
    >
      <UiButton v-if="!error" @click="addFiles">添加文件</UiButton>
    </EmptyState>

    <SplitView
      v-else
      v-model:width="listWidth"
      :detail-open="Boolean(selected)"
      list-label="资料库文件"
      detail-label="文件详情"
      @back="selectedId = null"
    >
      <template #list>
        <EmptyState
          v-if="!visible.length"
          :icon="FolderArchive"
          compact
          title="没有符合筛选条件的文件"
          description="换一个关键词或标签试试。"
        />
        <div v-else class="library-list" role="listbox" aria-label="资料库文件">
          <ListItem
            v-for="item in visible"
            :key="item.id"
            :title="item.name"
            :subtitle="detailMeta(item)"
            :selected="item.id === selectedId"
            @select="choose(item.id)"
          >
            <template #leading>
              <IconTile :tone="fileKind(item.name).tone" :size="30">
                <component :is="fileKind(item.name).icon" :size="16" />
              </IconTile>
            </template>
            <template #meta>{{ fileSize(item.size) }}</template>
          </ListItem>
        </div>
      </template>

      <template #detail>
        <div v-if="!selected" class="library-placeholder">
          <p>从左侧选择一个文件，查看或编辑它的信息。</p>
        </div>
        <div v-else class="library-detail">
          <div class="library-file">
            <FileRow
              :name="selected.name"
              :size="selected.size"
              :detail="detailMeta(selected)"
              :previewable="false"
              :downloadable="false"
            >
              <template #actions>
                <UiButton size="sm" variant="ghost" :disabled="locked" @click="openEntry">
                  <ExternalLink :size="14" /><span>打开</span>
                </UiButton>
                <UiButton size="sm" variant="ghost" :disabled="locked" @click="revealEntry">
                  <FolderOpen :size="14" /><span>所在文件夹</span>
                </UiButton>
                <UiButton size="sm" variant="ghost" :disabled="locked" @click="exportEntry">
                  <Download :size="14" />
                  <span>{{ action === 'export' ? '保存中…' : '另存为' }}</span>
                </UiButton>
              </template>
            </FileRow>
          </div>
          <p class="library-added">
            添加于 {{ updatedInfo(selected.addedAt) }}
            <span v-if="selected.updatedAt !== selected.addedAt">
              · 信息更新于 {{ updatedInfo(selected.updatedAt) }}
            </span>
          </p>

          <div class="library-form">
            <label class="library-field">
              <span>文件名</span>
              <input v-model="draft.name" class="text-field" maxlength="200" />
            </label>
            <div class="library-field-row">
              <label class="library-field">
                <span>关联课程</span>
                <input
                  v-model="draft.course"
                  class="text-field"
                  maxlength="120"
                  placeholder="例如：操作系统"
                />
              </label>
              <label class="library-field">
                <span>关联作业</span>
                <input
                  v-model="draft.assignment"
                  class="text-field"
                  maxlength="120"
                  placeholder="例如：第 6 次作业"
                />
              </label>
            </div>
            <label class="library-field">
              <span>标签 <em>用逗号分隔，最多 8 个</em></span>
              <input
                v-model="draft.tags"
                class="text-field"
                maxlength="280"
                placeholder="例如：作业，期末复习"
              />
              <span v-if="draftTags.length" class="library-tags">
                <span v-for="item in draftTags" :key="item">{{ item }}</span>
              </span>
            </label>
            <label class="library-field">
              <span>备注</span>
              <textarea
                v-model="draft.note"
                class="text-field library-note"
                rows="3"
                maxlength="2000"
                placeholder="可选，只保存在本机"
              ></textarea>
            </label>
            <div class="library-form-actions">
              <UiButton
                size="sm"
                :disabled="saving || !dirty || Boolean(action)"
                @click="saveDraft"
              >
                <Save :size="14" /><span>{{ saving ? '保存中…' : '保存信息' }}</span>
              </UiButton>
              <span v-if="dirty" class="library-dirty">有未保存的修改</span>
              <UiButton
                size="sm"
                variant="ghost"
                class="library-delete"
                :disabled="locked"
                @click="deleteEntry"
              >
                <Trash2 :size="14" /><span>删除</span>
              </UiButton>
            </div>
          </div>

          <p class="library-caveat">
            文件保存在本机，按当前校园账号隔离。同步远端和作业分享尚未接入；提交作业仍在思源学堂网页完成。
          </p>
        </div>
      </template>
    </SplitView>
  </section>
</template>

<style scoped>
.library-list {
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.library-list > :deep(.list-item + .list-item) {
  border-top: 1px solid var(--border);
}
.library-placeholder {
  display: grid;
  place-items: center;
  min-height: 220px;
  padding: 24px;
  color: var(--text-tertiary);
  font-size: 12.5px;
  text-align: center;
}
.library-detail {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.library-file {
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.library-file :deep(.file-actions) {
  flex-wrap: wrap;
  justify-content: flex-end;
}
.library-added {
  margin: -4px 2px 0;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.library-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 14px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.library-field {
  display: flex;
  flex-direction: column;
  gap: 5px;
  min-width: 0;
  color: var(--text-secondary);
  font-size: 11.5px;
}
.library-field em {
  color: var(--text-tertiary);
  font-style: normal;
}
.library-field-row {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px;
}
.library-note {
  height: auto;
  min-height: 66px;
  padding: 8px 10px;
  line-height: 1.55;
  resize: vertical;
}
.library-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 5px;
}
.library-tags span {
  padding: 2px 7px;
  border-radius: 999px;
  background: var(--accent-soft);
  color: var(--accent-text);
  font-size: 11px;
}
.library-form-actions {
  display: flex;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}
.library-dirty {
  color: var(--warning);
  font-size: 11.5px;
}
.library-delete {
  margin-left: auto;
  color: var(--danger);
}
.library-caveat {
  margin: 0 2px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.6;
}
@media (max-width: 900px) {
  .library-field-row {
    grid-template-columns: 1fr;
  }
}
</style>
