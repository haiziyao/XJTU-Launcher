<script setup lang="ts">
import { computed, nextTick, ref } from 'vue';
import { Check, Pencil, Pin, Search, Trash2, X } from 'lucide-vue-next';
import type { AgentSummary } from '../types';

const props = defineProps<{
  summaries: AgentSummary[];
  currentId?: string;
  disabled?: boolean;
}>();
const emit = defineEmits<{
  open: [id: string];
  remove: [summary: AgentSummary];
  rename: [id: string, title: string];
  pin: [id: string, pinned: boolean];
  close: [];
}>();

const search = ref('');
const editingId = ref('');
const editingTitle = ref('');

function dayStart(value: number) {
  const date = new Date(value);
  date.setHours(0, 0, 0, 0);
  return date.getTime();
}
const groups = computed(() => {
  const needle = search.value.trim().toLowerCase();
  const filtered = props.summaries.filter((summary) =>
    (summary.title + (summary.roleId || '')).toLowerCase().includes(needle),
  );
  const today = dayStart(Date.now());
  const yesterday = today - 86_400_000;
  const pinned: AgentSummary[] = [];
  const buckets: { title: string; items: AgentSummary[] }[] = [
    { title: '今天', items: [] },
    { title: '昨天', items: [] },
    { title: '更早', items: [] },
  ];
  for (const summary of filtered) {
    if (summary.pinned) {
      pinned.push(summary);
      continue;
    }
    const day = dayStart(summary.updatedAt);
    const bucket = day >= today ? buckets[0] : day >= yesterday ? buckets[1] : buckets[2];
    bucket.items.push(summary);
  }
  const result: { title: string; items: AgentSummary[] }[] = [];
  if (pinned.length) result.push({ title: '置顶', items: pinned });
  for (const bucket of buckets) if (bucket.items.length) result.push(bucket);
  return result;
});
const total = computed(() => groups.value.reduce((count, group) => count + group.items.length, 0));

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
</script>
<template>
  <aside class="history" aria-label="历史对话">
    <header class="history-head">
      <span>历史记录</span>
      <button class="icon-button" aria-label="关闭" @click="emit('close')"><X :size="14" /></button>
    </header>
    <label class="history-search"
      ><Search :size="13" /><input v-model="search" placeholder="搜索对话" aria-label="搜索对话"
    /></label>
    <p v-if="!total" class="history-empty">{{ search ? '没有匹配的对话' : '还没有对话' }}</p>
    <div v-for="group in groups" :key="group.title" class="history-group">
      <p class="history-group-title">{{ group.title }}</p>
      <div v-for="summary in group.items" :key="summary.id" class="history-row">
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
          <button
            class="icon-button"
            :disabled="disabled"
            :aria-label="summary.pinned ? '取消置顶' : '置顶'"
            :aria-pressed="summary.pinned"
            @click="emit('pin', summary.id, !summary.pinned)"
          >
            <Pin :size="13" :class="{ pinned: summary.pinned }" />
          </button>
          <button
            class="icon-button"
            :disabled="disabled"
            aria-label="重命名"
            @click="startRename(summary)"
          >
            <Check v-if="editingId === summary.id" :size="13" /><Pencil v-else :size="13" />
          </button>
          <button
            class="icon-button"
            :disabled="disabled"
            aria-label="删除对话"
            @click="emit('remove', summary)"
          >
            <Trash2 :size="13" />
          </button>
        </div>
      </div>
    </div>
  </aside>
</template>
<style scoped>
.history {
  position: absolute;
  top: 46px;
  right: 0;
  z-index: 21;
  width: 300px;
  max-height: 60vh;
  overflow-y: auto;
  padding: 6px;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--surface);
  box-shadow: 0 16px 40px rgba(15, 23, 42, 0.14);
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
.history-empty {
  margin: 0;
  padding: 12px 8px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.history-group-title {
  margin: 8px 8px 2px;
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
  background: var(--surface-hover);
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
.icon-button {
  width: 24px;
  height: 24px;
  border: 0;
  border-radius: 6px;
  background: transparent;
  color: var(--text-tertiary);
  display: grid;
  place-items: center;
  cursor: pointer;
}
.icon-button:hover:not(:disabled) {
  background: var(--surface-pressed);
  color: var(--text);
}
.pinned {
  color: var(--accent);
}
</style>
