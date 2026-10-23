<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { Brain, LoaderCircle, Plus, Trash2 } from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { AgentMemory } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import EmptyState from './ui/EmptyState.vue';

const entries = ref<AgentMemory[]>([]);
const loading = ref(true);
const busy = ref(false);
const error = ref('');
const status = ref('');
const draft = ref('');
const editing = ref<string | null>(null);

async function load() {
  loading.value = true;
  try {
    entries.value = await bridge.agentMemories();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
function edit(entry: AgentMemory) {
  editing.value = entry.id;
  draft.value = entry.content;
}
async function save() {
  const content = draft.value.trim();
  if (!content || busy.value) return;
  busy.value = true;
  error.value = '';
  try {
    await bridge.agentSaveMemory(editing.value, content);
    status.value = editing.value ? '记忆已更新' : '记忆已保存';
    draft.value = '';
    editing.value = null;
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
async function remove(entry: AgentMemory) {
  if (
    !(await ask('删除这条记忆？', {
      title: '删除记忆',
      kind: 'warning',
      okLabel: '删除',
      cancelLabel: '取消',
    }))
  )
    return;
  busy.value = true;
  try {
    await bridge.agentRemoveMemory(entry.id);
    if (editing.value === entry.id) {
      editing.value = null;
      draft.value = '';
    }
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
onMounted(() => void load());
</script>
<template>
  <div>
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <Transition name="drop">
      <p v-if="status" class="notice success" role="status">{{ status }}</p>
    </Transition>
    <SettingGroup
      title="全局记忆"
      footnote="记忆会附加到每次对话的系统提示词。Agent 也可以在你明确要求时自行记录；这里可以手动增删改。"
    >
      <form class="memory-form" @submit.prevent="save">
        <textarea
          v-model="draft"
          rows="3"
          maxlength="1000"
          placeholder="例如：习惯用简洁的中文回答；每周三下午没有课。"
        ></textarea>
        <div class="memory-actions">
          <button
            v-if="editing"
            type="button"
            class="btn ghost small"
            @click="
              editing = null;
              draft = '';
            "
          >
            取消编辑
          </button>
          <button type="submit" class="btn primary small" :disabled="!draft.trim() || busy">
            <Plus :size="13" />{{ editing ? '保存修改' : '添加记忆' }}
          </button>
        </div>
      </form>
      <EmptyState
        v-if="loading && !entries.length"
        compact
        :icon="LoaderCircle"
        busy
        title="正在读取记忆…"
      />
      <EmptyState
        v-else-if="!entries.length"
        compact
        :icon="Brain"
        title="还没有长期记忆"
        description="告诉 Agent“记住我喜欢简洁回答”，它也能自己写入。"
      />
      <div v-for="entry in entries" :key="entry.id" class="memory-row">
        <span class="memory-text">{{ entry.content }}</span>
        <button class="btn small" :disabled="busy" @click="edit(entry)">编辑</button>
        <button
          class="btn ghost icon"
          :disabled="busy"
          aria-label="删除记忆"
          @click="remove(entry)"
        >
          <Trash2 :size="15" />
        </button>
      </div>
    </SettingGroup>
  </div>
</template>
<style scoped>
.memory-form {
  display: grid;
  gap: 8px;
  padding: 14px 12px;
}
.memory-form textarea {
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  padding: 8px 10px;
  resize: vertical;
}
.memory-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.memory-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px;
}
.memory-text {
  flex: 1;
  min-width: 0;
  font-size: 13px;
  line-height: 1.55;
  white-space: pre-wrap;
  word-break: break-word;
}
</style>
