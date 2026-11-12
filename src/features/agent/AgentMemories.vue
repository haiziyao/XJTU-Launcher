<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { IconBrain, IconLoader2, IconPlus, IconTrash } from '@tabler/icons-vue';
import { bridge } from '../../shared/bridge';
import type { AgentMemory } from '../../types';
import SettingGroup from '../../components/ui/SettingGroup.vue';
import SettingRow from '../../components/ui/SettingRow.vue';
import EmptyState from '../../components/ui/EmptyState.vue';
import IconTile from '../../components/ui/IconTile.vue';
import SettingsDialog from '../../components/ui/SettingsDialog.vue';

import UiButton from '../../components/ui/UiButton.vue';
const entries = ref<AgentMemory[]>([]);
const loading = ref(true);
const busy = ref(false);
const error = ref('');
const now = ref<AgentMemory | 'new' | null>(null);
const content = ref('');

const open = computed(() => now.value !== null);
const title = computed(() => (now.value === 'new' ? '新增记忆' : '记忆'));
const saveable = computed(() => !!content.value.trim() && !busy.value);

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
function openEntry(entry: AgentMemory) {
  error.value = '';
  now.value = entry;
  content.value = entry.content;
}
function openNew() {
  error.value = '';
  now.value = 'new';
  content.value = '';
}
function close() {
  now.value = null;
  content.value = '';
  error.value = '';
}
async function save() {
  if (!saveable.value || now.value === null) return;
  busy.value = true;
  error.value = '';
  try {
    await bridge.agentSaveMemory(now.value === 'new' ? null : now.value.id, content.value.trim());
    close();
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = false;
  }
}
function summary(text: string) {
  return text.length > 44 ? `${text.slice(0, 44)}…` : text;
}
function removeCurrent() {
  const entry = now.value;
  if (entry && entry !== 'new') void remove(entry);
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
  error.value = '';
  try {
    await bridge.agentRemoveMemory(entry.id);
    close();
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
  <Transition name="drop">
    <p v-if="error && !open" class="notice" role="alert">{{ error }}</p>
  </Transition>
  <SettingGroup title="全局记忆">
    <EmptyState
      v-if="loading && !entries.length"
      compact
      :icon="IconLoader2"
      busy
      title="正在读取记忆…"
    />
    <EmptyState v-else-if="!entries.length" compact :icon="IconBrain" title="还没有长期记忆" />
    <SettingRow
      v-for="entry in entries"
      :key="entry.id"
      clickable
      :title="summary(entry.content)"
      :disabled="busy"
      @select="openEntry(entry)"
    />
    <SettingRow clickable title="新增记忆" :disabled="busy" @select="openNew">
      <template #leading
        ><IconTile :size="30"><IconPlus :size="16" /></IconTile
      ></template>
    </SettingRow>
  </SettingGroup>

  <SettingsDialog :open="open" :title="title" wide @close="close">
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <textarea
      v-model="content"
      class="memory-field"
      rows="6"
      maxlength="1000"
      placeholder="例如：习惯用简洁的中文回答；每周三下午没有课。"
    ></textarea>

    <template #footer>
      <UiButton
        v-if="now && now !== 'new'"
        variant="ghost"
        size="sm"
        danger
        :disabled="busy"
        @click="removeCurrent"
      >
        <IconTrash :size="13" />删除
      </UiButton>
      <UiButton variant="primary" size="sm" :disabled="!saveable" @click="save">
        {{ busy ? '保存中…' : '保存' }}
      </UiButton>
    </template>
  </SettingsDialog>
</template>
<style scoped>
.memory-field {
  border: 1px solid var(--control-border);
  border-radius: var(--radius);
  background: var(--control-bg);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  line-height: 1.6;
  padding: 8px 10px;
  resize: vertical;
}
.memory-field:focus {
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
  outline: none;
}
</style>
