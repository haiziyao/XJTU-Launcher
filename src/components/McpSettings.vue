<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { IconLoader2, IconPlus, IconPlugConnected } from '@tabler/icons-vue';
import { bridge } from '../bridge';
import type { McpServer, McpTool, McpTransport } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import UiSwitch from './ui/UiSwitch.vue';
import SettingsDialog from './ui/SettingsDialog.vue';

import UiButton from './ui/UiButton.vue';

interface Draft {
  id: string | null;
  name: string;
  type: 'stdio' | 'http';
  command: string;
  /** One argument per line so paths with spaces survive. */
  args: string;
  url: string;
  vars: { key: string; value: string }[];
}

const servers = ref<McpServer[]>([]);
const loading = ref(true);
const busy = ref('');
const error = ref('');
const status = ref('');
const selected = ref<McpServer | null>(null);
const probed = ref<McpTool[] | null>(null);
const draft = ref<Draft | null>(null);

const dialogOpen = computed(() => selected.value !== null || draft.value !== null);
const dialogTitle = computed(() => {
  if (draft.value) return draft.value.id ? '编辑 MCP 服务' : '新增 MCP 服务';
  return selected.value?.name ?? '';
});

function draftFrom(server?: McpServer): Draft {
  if (!server) {
    return { id: null, name: '', type: 'stdio', command: '', args: '', url: '', vars: [] };
  }
  const transport = server.transport;
  return {
    id: server.id,
    name: server.name,
    type: transport.type,
    command: transport.type === 'stdio' ? transport.command : '',
    args: transport.type === 'stdio' ? (transport.args || []).join('\n') : '',
    url: transport.type === 'http' ? transport.url : '',
    vars: Object.keys(
      transport.type === 'stdio' ? transport.env || {} : transport.headers || {},
    ).map((key) => ({ key, value: '' })),
  };
}
function transportOf(value: Draft): McpTransport {
  const vars: Record<string, string> = {};
  for (const entry of value.vars) {
    const key = entry.key.trim();
    if (key) vars[key] = entry.value;
  }
  if (value.type === 'stdio') {
    return {
      type: 'stdio',
      command: value.command.trim(),
      args: value.args
        .split('\n')
        .map((line) => line.trim())
        .filter(Boolean),
      env: vars,
    };
  }
  return { type: 'http', url: value.url.trim(), headers: vars };
}
const valid = computed(() => {
  const value = draft.value;
  if (!value || !value.name.trim()) return false;
  if (value.type === 'stdio') return !!value.command.trim();
  return (
    /^https:\/\/\S+$/.test(value.url.trim()) ||
    /^http:\/\/(localhost|127\.0\.0\.1)/.test(value.url.trim())
  );
});

async function load() {
  loading.value = true;
  try {
    servers.value = await bridge.mcpServers();
    if (selected.value) {
      selected.value = servers.value.find((item) => item.id === selected.value?.id) ?? null;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
function openView(server: McpServer) {
  error.value = '';
  status.value = '';
  probed.value = null;
  selected.value = server;
  draft.value = null;
}
function openAdd() {
  error.value = '';
  status.value = '';
  probed.value = null;
  selected.value = null;
  draft.value = draftFrom();
}
function close() {
  selected.value = null;
  draft.value = null;
  error.value = '';
  status.value = '';
}
async function toggle(server: McpServer) {
  busy.value = server.id;
  error.value = '';
  try {
    await bridge.setMcpEnabled(server.id, !server.enabled);
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
async function probe(server: McpServer) {
  busy.value = server.id;
  error.value = '';
  status.value = '';
  try {
    probed.value = await bridge.probeMcpServer(server.id);
    status.value = `${probed.value.length} 个工具`;
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
async function remove(server: McpServer) {
  if (
    !(await ask(`移除 MCP 服务“${server.name}”？`, {
      title: '移除 MCP 服务',
      kind: 'warning',
      okLabel: '移除',
      cancelLabel: '取消',
    }))
  )
    return;
  busy.value = server.id;
  error.value = '';
  try {
    await bridge.removeMcpServer(server.id);
    close();
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
async function save() {
  if (!valid.value || !draft.value || busy.value) return;
  busy.value = 'save';
  error.value = '';
  try {
    await bridge.saveMcpServer({
      id: draft.value.id,
      name: draft.value.name.trim(),
      transport: transportOf(draft.value),
    });
    draft.value = null;
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
onMounted(() => void load());
</script>
<template>
  <SettingGroup title="MCP 服务">
    <Transition name="drop">
      <p v-if="error && !dialogOpen" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <EmptyState
      v-if="loading && !servers.length"
      compact
      :icon="IconLoader2"
      busy
      title="正在读取 MCP 配置…"
    />
    <SettingRow
      v-for="server in servers"
      :key="server.id"
      clickable
      :title="server.name"
      :disabled="!!busy"
      @select="openView(server)"
    >
      <template #leading
        ><IconTile :size="30"><IconPlugConnected :size="16" /></IconTile
      ></template>
      <span v-if="!server.enabled" class="badge">已停用</span>
    </SettingRow>
    <SettingRow clickable title="新增 MCP 服务" :disabled="!!busy" @select="openAdd">
      <template #leading
        ><IconTile :size="30"><IconPlus :size="16" /></IconTile
      ></template>
    </SettingRow>
  </SettingGroup>

  <SettingsDialog :open="dialogOpen" :title="dialogTitle" wide @close="close">
    <template v-if="selected">
      <div class="mcp-detail">
        <span>{{ selected.kind === 'stdio' ? '本地进程' : '远程 HTTP' }}</span>
        <code>{{ selected.target }}</code>
      </div>
      <SettingRow title="启用">
        <UiSwitch
          :checked="selected.enabled"
          :label="selected.name + '启用'"
          :disabled="!!busy"
          @toggle="toggle(selected)"
        />
      </SettingRow>
      <Transition name="drop">
        <p v-if="error" class="notice" role="alert">{{ error }}</p>
      </Transition>
      <p v-if="status" class="notice success" role="status">{{ status }}</p>
      <ul v-if="probed?.length" class="mcp-tools">
        <li v-for="tool in probed" :key="tool.name">
          <strong>{{ tool.name }}</strong>
          <span>{{ tool.description }}</span>
        </li>
      </ul>
    </template>

    <form v-else-if="draft" class="mcp-form" @submit.prevent="save">
      <label class="mcp-field"
        ><span>名称</span
        ><input v-model="draft.name" maxlength="40" placeholder="例如：GitHub" required
      /></label>
      <div class="segmented-field" role="radiogroup" aria-label="连接方式">
        <button
          type="button"
          :class="{ selected: draft.type === 'stdio' }"
          @click="draft.type = 'stdio'"
        >
          本地命令
        </button>
        <button
          type="button"
          :class="{ selected: draft.type === 'http' }"
          @click="draft.type = 'http'"
        >
          HTTP
        </button>
      </div>
      <template v-if="draft.type === 'stdio'">
        <label class="mcp-field"
          ><span>命令</span><input v-model="draft.command" placeholder="npx" required
        /></label>
        <label class="mcp-field"
          ><span>参数（每行一个）</span
          ><textarea
            v-model="draft.args"
            rows="3"
            placeholder="-y&#10;@modelcontextprotocol/server-filesystem"
          ></textarea>
        </label>
      </template>
      <label v-else class="mcp-field"
        ><span>地址</span><input v-model="draft.url" placeholder="https://example.com/mcp" required
      /></label>
      <div class="mcp-vars">
        <div class="mcp-vars-head">
          <span>{{ draft.type === 'stdio' ? '环境变量' : '请求头' }}</span>
          <UiButton
            variant="ghost"
            size="sm"
            type="button"
            @click="draft.vars.push({ key: '', value: '' })"
          >
            <IconPlus :size="13" />添加
          </UiButton>
        </div>
        <div v-for="(entry, index) in draft.vars" :key="index" class="mcp-var">
          <input v-model="entry.key" placeholder="KEY" aria-label="名称" />
          <input v-model="entry.value" type="password" placeholder="值" aria-label="值" />
          <UiButton
            variant="ghost"
            size="sm"
            type="button"
            aria-label="删除此行"
            @click="draft.vars.splice(index, 1)"
          >
            删除
          </UiButton>
        </div>
        <p v-if="draft.vars.length" class="mcp-hint">编辑时留空表示保留已保存的值。</p>
      </div>
      <Transition name="drop">
        <p v-if="error" class="notice" role="alert">{{ error }}</p>
      </Transition>
    </form>

    <template #footer>
      <template v-if="selected">
        <UiButton variant="ghost" size="sm" danger :disabled="!!busy" @click="remove(selected)"
          >移除</UiButton
        >
        <UiButton size="sm" :disabled="!!busy" @click="probe(selected)">测试连接</UiButton>
        <UiButton
          variant="primary"
          size="sm"
          :disabled="!!busy"
          @click="draft = draftFrom(selected)"
          >编辑</UiButton
        >
      </template>
      <template v-else>
        <UiButton variant="ghost" size="sm" :disabled="!!busy" @click="close">取消</UiButton>
        <UiButton variant="primary" size="sm" :disabled="!valid || !!busy" @click="save">
          {{ busy === 'save' ? '保存中…' : '保存' }}
        </UiButton>
      </template>
    </template>
  </SettingsDialog>
</template>
<style scoped>
.mcp-detail {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.mcp-detail code {
  overflow-wrap: anywhere;
  color: var(--text);
  font-size: 12.5px;
}
.mcp-tools {
  margin: 0;
  padding: 0;
  list-style: none;
  display: grid;
  gap: 8px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.mcp-tools li {
  display: grid;
  gap: 2px;
}
.mcp-tools strong {
  color: var(--text);
  font-weight: 600;
}
.mcp-form {
  display: grid;
  gap: 14px;
}
.mcp-field {
  display: grid;
  gap: 6px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.mcp-field input,
.mcp-field textarea,
.mcp-var input {
  border: 1px solid var(--control-border);
  border-radius: var(--radius);
  background: var(--control-bg);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  padding: 8px 10px;
  resize: vertical;
}
.mcp-field input:focus,
.mcp-field textarea:focus,
.mcp-var input:focus {
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
  outline: none;
}
.segmented-field {
  display: inline-flex;
  border: 1px solid var(--border);
  border-radius: 9px;
  overflow: hidden;
  width: fit-content;
}
.segmented-field button {
  border: 0;
  background: transparent;
  color: var(--text-secondary);
  padding: 7px 16px;
  font: inherit;
  font-size: 13px;
  cursor: pointer;
}
.segmented-field button.selected {
  background: var(--accent-soft);
  color: var(--accent);
}
.mcp-vars {
  display: grid;
  gap: 8px;
}
.mcp-vars-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  color: var(--text-tertiary);
}
.mcp-var {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1.4fr) auto;
  gap: 8px;
}
.mcp-hint {
  margin: 0;
  font-size: 12px;
  color: var(--text-tertiary);
}
</style>
