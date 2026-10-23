<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { Check, Folder, Pencil, Pin, Plus, Search, Trash2, X } from 'lucide-vue-next';
import UiButton from './ui/UiButton.vue';
import UiIconButton from './ui/UiIconButton.vue';
import type { AgentSummary, AgentWorkspace } from '../types';

const props = defineProps<{
  summaries: AgentSummary[];
  workspaces: AgentWorkspace[];
  currentId?: string;
  disabled?: boolean;
}>();
const emit = defineEmits<{
  open: [id: string];
  remove: [summary: AgentSummary];
  rename: [id: string, title: string];
  pin: [id: string, pinned: boolean];
  close: [];
  create: [workspaceId: string | null];
  'create-workspace': [name: string];
}>();

const search = ref('');
const editingId = ref('');
const editingTitle = ref('');
const creating = ref(false);
const workspaceName = ref('');

interface Section {
  key: string;
  title: string;
  workspaceId: string | null;
  rows: AgentSummary[];
}

function dayStart(value: number) {
  const date = new Date(value);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}
const newestFirst = (a: AgentSummary, b: AgentSummary) => b.updatedAt - a.updatedAt;
const sections = computed<Section[]>(() => {
  const needle = search.value.trim().toLowerCase();
  const filtered = props.summaries.filter((summary) =>
    (summary.title + (summary.roleId || '')).toLowerCase().includes(needle),
  );
  const pinned = filtered.filter((summary) => summary.pinned).sort(newestFirst);
  const known = new Set(props.workspaces.map((workspace) => workspace.id));
  const result: Section[] = [];
  if (pinned.length) result.push({ key: 'pinned', title: '置顶', workspaceId: null, rows: pinned });
  for (const workspace of props.workspaces) {
    result.push({
      key: `workspace-${workspace.id}`,
      title: workspace.name,
      workspaceId: workspace.id,
      rows: filtered
        .filter((summary) => !summary.pinned && summary.workspaceId === workspace.id)
        .sort(newestFirst),
    });
  }
  // Conversations without a workspace keep the familiar day grouping.
  const today = dayStart(Date.now());
  const yesterday = today - 86_400_000;
  const buckets = [
    { key: 'today', title: '今天', items: [] as AgentSummary[] },
    { key: 'yesterday', title: '昨天', items: [] as AgentSummary[] },
    { key: 'older', title: '更早', items: [] as AgentSummary[] },
  ];
  for (const summary of filtered) {
    if (summary.pinned) continue;
    if (summary.workspaceId && known.has(summary.workspaceId)) continue;
    const day = dayStart(summary.updatedAt);
    const bucket = day >= today ? buckets[0] : day >= yesterday ? buckets[1] : buckets[2];
    bucket.items.push(summary);
  }
  for (const bucket of buckets) {
    if (bucket.items.length) {
      result.push({
        key: bucket.key,
        title: bucket.title,
        workspaceId: null,
        rows: bucket.items.sort(newestFirst),
      });
    }
  }
  return result;
});
const total = computed(() =>
  sections.value.reduce((count, section) => count + section.rows.length, 0),
);
const empty = computed(
  () => !total.value && !props.workspaces.length && !search.value.trim() && !creating.value,
);

function startRename(summary: AgentSummary) {
  editingId.value = summary.id;
  editingTitle.value = summary.title;
  void nextTick(() => {
    const element = document.querySelector<HTMLInputElement>('.history-rename');
    element?.focus();
    element?.select();
  });
}
function commitRename() {
  const title = editingTitle.value.trim();
  if (title && editingId.value) emit('rename', editingId.value, title);
  editingId.value = '';
}
function startCreating() {
  creating.value = true;
  workspaceName.value = '';
  void nextTick(() => {
    document.querySelector<HTMLInputElement>('.history-workspace-input')?.focus();
  });
}
function commitCreating() {
  const name = workspaceName.value.trim();
  if (name) emit('create-workspace', name);
  creating.value = false;
  workspaceName.value = '';
}
</script>
<template>
  <aside class="history" aria-label="对话管理">
    <header class="history-head">
      <span>对话</span>
      <UiIconButton label="关闭" @click="emit('close')"><X :size="14" /></UiIconButton>
    </header>
    <label class="history-search"
      ><Search :size="13" /><input v-model="search" placeholder="搜索对话" aria-label="搜索对话"
    /></label>
    <div class="history-tools">
      <UiButton variant="tinted" size="sm" @click="emit('create', null)">
        <Plus :size="13" />新对话
      </UiButton>
      <UiButton variant="tinted" size="sm" @click="startCreating">
        <Plus :size="13" />工作区
      </UiButton>
    </div>
    <form v-if="creating" class="history-workspace-form" @submit.prevent="commitCreating">
      <input
        v-model="workspaceName"
        class="history-workspace-input"
        placeholder="工作区名称"
        maxlength="40"
        aria-label="工作区名称"
        @keydown.esc="creating = false"
      />
      <UiIconButton label="创建工作区" type="submit">
        <Check :size="14" />
      </UiIconButton>
    </form>
    <p v-if="empty" class="history-empty">还没有对话</p>
    <p v-else-if="!total && !workspaces.length" class="history-empty">没有匹配的对话</p>
    <div
      v-for="section in sections"
      :key="section.key"
      class="history-group"
      :class="{ 'is-workspace': section.workspaceId }"
    >
      <div class="history-group-head">
        <p class="history-group-title">
          <Folder v-if="section.workspaceId" :size="12" />{{ section.title }}
        </p>
        <UiButton
          v-if="section.workspaceId"
          variant="tinted"
          size="sm"
          :disabled="disabled"
          @click="emit('create', section.workspaceId)"
        >
          <Plus :size="12" />新建
        </UiButton>
      </div>
      <p v-if="section.workspaceId && !section.rows.length" class="history-empty small">
        还没有对话
      </p>
      <div v-for="summary in section.rows" :key="summary.id" class="history-row">
        <template v-if="editingId === summary.id">
          <input
            v-model="editingTitle"
            class="history-rename"
            maxlength="60"
            aria-label="对话标题"
            @keydown.enter.prevent="commitRename"
            @keydown.esc="editingId = ''"
            @blur="commitRename"
          />
        </template>
        <button
          v-else
          class="history-open"
          :class="{ current: currentId === summary.id }"
          :disabled="disabled"
          @click="emit('open', summary.id)"
        >
          <span>{{ summary.title }}</span>
          <small>{{ summary.messages }} 条</small>
        </button>
        <div class="history-actions">
          <UiIconButton
            :label="summary.pinned ? '取消置顶' : '置顶'"
            :active="summary.pinned"
            :aria-pressed="summary.pinned"
            :disabled="disabled"
            @click="emit('pin', summary.id, !summary.pinned)"
          >
            <Pin :size="13" :class="{ pinned: summary.pinned }" />
          </UiIconButton>
          <UiIconButton label="重命名" :disabled="disabled" @click="startRename(summary)">
            <Check v-if="editingId === summary.id" :size="13" /><Pencil v-else :size="13" />
          </UiIconButton>
          <UiIconButton label="删除对话" :disabled="disabled" @click="emit('remove', summary)">
            <Trash2 :size="13" />
          </UiIconButton>
        </div>
      </div>
    </div>
  </aside>
</template>
<style scoped>
.history {
  position: absolute;
  top: 44px;
  left: 0;
  bottom: 0;
  z-index: 21;
  display: flex;
  flex-direction: column;
  width: 320px;
  max-width: 82vw;
  padding: 6px 6px 150px;
  border: 1px solid var(--border);
  border-radius: 0 12px 12px 0;
  background: var(--surface);
  box-shadow: 10px 0 30px rgba(15, 23, 42, 0.12);
  overflow-y: auto;
  /* Solid until the very bottom of the page, then a short fade; the padded
     list keeps every entry above it, so scrolling never hides one. */
  -webkit-mask-image: linear-gradient(
    to bottom,
    #000 calc(100% - 80px),
    transparent calc(100% - 40px)
  );
  mask-image: linear-gradient(to bottom, #000 calc(100% - 80px), transparent calc(100% - 40px));
}
.history-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 4px 6px 8px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.history-search {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 4px 6px;
  padding: 5px 8px;
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-tertiary);
}
.history-search input {
  flex: 1;
  min-width: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 12.5px;
  outline: none;
}
.history-tools {
  display: flex;
  gap: 4px;
  margin: 0 4px 4px;
}
.history-workspace-form {
  display: flex;
  align-items: center;
  gap: 4px;
  margin: 0 4px 6px;
}
.history-workspace-input {
  flex: 1;
  min-width: 0;
  padding: 5px 8px;
  border: 1px solid var(--accent);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: 12.5px;
  outline: none;
}
.history-empty {
  margin: 0;
  padding: 10px 8px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.history-empty.small {
  padding: 4px 8px;
}
.history-group.is-workspace {
  margin: 4px 4px 6px;
  padding: 2px 0 4px;
  border-radius: 10px;
  background: var(--accent-wash);
}
.history-group.is-workspace .history-group-head {
  margin: 4px 8px 2px;
}
.history-group.is-workspace .history-group-title {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  color: var(--accent-text);
  font-weight: 600;
}
.history-group.is-workspace .history-row {
  margin-left: 12px;
}
.history-group.is-workspace .history-open {
  font-size: 12.5px;
}
.history-group-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 6px;
  margin: 8px 8px 2px;
}
.history-group-title {
  margin: 0;
  font-size: 11px;
  color: var(--text-tertiary);
}
.history-row {
  display: flex;
  align-items: center;
  gap: 2px;
  border-radius: 8px;
}
.history-row:hover {
  background: var(--accent-wash);
}
.history-open {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: baseline;
  gap: 8px;
  border: 0;
  background: transparent;
  padding: 8px;
  border-radius: 8px;
  text-align: left;
  font: inherit;
  font-size: 13px;
  color: var(--text);
  cursor: pointer;
}
.history-open span {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.history-open small {
  color: var(--text-tertiary);
  font-size: 11px;
}
.history-open.current {
  background: var(--accent-soft);
  color: var(--accent-text);
}
.history-rename {
  flex: 1;
  min-width: 0;
  margin: 3px 0;
  padding: 5px 8px;
  border: 1px solid var(--accent);
  border-radius: 8px;
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: 13px;
}
.history-actions {
  display: flex;
  align-items: center;
}
.pinned {
  color: var(--accent);
}
</style>
