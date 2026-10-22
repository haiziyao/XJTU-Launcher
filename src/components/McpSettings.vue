<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import { LoaderCircle, Plus, Plug, RefreshCw, Server, Wrench } from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { McpCatalog, McpEntry, McpServer, McpTool, McpTransport } from '../types';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import UiSwitch from './ui/UiSwitch.vue';

const props = defineProps<{ search: string }>();
const emit = defineEmits<{ settings: [] }>();

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
const open = ref('');
const tools = ref<Record<string, McpTool[]>>({});
const draft = ref<Draft | null>(null);
const catalog = ref<McpCatalog>({ source: null, defaultSource: null, servers: [] });
const catalogLoading = ref(false);
const catalogLoaded = ref(false);
const installing = ref<{ entry: McpEntry; values: Record<string, string> } | null>(null);
let disposed = false;

const needle = computed(() => props.search.trim().toLowerCase());
const filteredServers = computed(() =>
  servers.value.filter((server) =>
    (server.name + server.target).toLowerCase().includes(needle.value),
  ),
);
const filteredMarket = computed(() =>
  catalog.value.servers.filter((entry) =>
    (entry.name + entry.description).toLowerCase().includes(needle.value),
  ),
);

function describe(transport: McpTransport) {
  return transport.type === 'stdio'
    ? [transport.command, ...(transport.args || [])].join(' ')
    : transport.url;
}
function draftFrom(server?: McpServer): Draft {
  if (!server) {
    return {
      id: null,
      name: '',
      type: 'stdio',
      command: '',
      args: '',
      url: '',
      vars: [],
    };
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
    const next = await bridge.mcpServers();
    if (!disposed) servers.value = next;
  } catch (e) {
    if (!disposed) error.value = String(e);
  } finally {
    loading.value = false;
  }
}
async function refreshCatalog() {
  if (catalogLoading.value) return;
  catalogLoading.value = true;
  error.value = '';
  try {
    const next = await bridge.mcpCatalog();
    if (!disposed) {
      catalog.value = next;
      catalogLoaded.value = true;
    }
  } catch (e) {
    if (!disposed) error.value = String(e);
  } finally {
    catalogLoading.value = false;
  }
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
    const result = await bridge.probeMcpServer(server.id);
    tools.value = { ...tools.value, [server.id]: result };
    status.value = `${server.name} 已连接，${result.length} 个工具`;
  } catch (e) {
    error.value = `${server.name}：${String(e)}`;
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
    if (open.value === server.id) open.value = '';
    const rest = { ...tools.value };
    delete rest[server.id];
    tools.value = rest;
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
  status.value = '';
  try {
    await bridge.saveMcpServer({
      id: draft.value.id,
      name: draft.value.name.trim(),
      transport: transportOf(draft.value),
    });
    status.value = draft.value.id ? 'MCP 服务已更新' : 'MCP 服务已添加';
    draft.value = null;
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
function startInstall(entry: McpEntry) {
  const values: Record<string, string> = {};
  for (const field of entry.fields) values[field.key] = '';
  if (!entry.fields.length) {
    void install(entry, values);
    return;
  }
  installing.value = { entry, values };
}
async function install(entry: McpEntry, values: Record<string, string>) {
  busy.value = entry.id;
  error.value = '';
  status.value = '';
  try {
    await bridge.installMcpEntry(entry.id, values);
    installing.value = null;
    status.value = `${entry.name} 已添加到 MCP 服务`;
    await load();
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
onMounted(() => {
  void load();
  void refreshCatalog();
});
onBeforeUnmount(() => {
  disposed = true;
});
</script>
<template>
  <div class="mcp-panel">
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <Transition name="drop">
      <p v-if="status" class="notice success" role="status">{{ status }}</p>
    </Transition>

    <SettingGroup
      v-if="!draft"
      title="MCP 服务"
      footnote="启用的 MCP 服务会作为工具提供给 Agent。凭据保存在本机 mcp.json，不会回显。"
    >
      <template #actions>
        <button class="btn small" :disabled="!!busy" @click="draft = draftFrom()">
          <Plus :size="14" />添加 MCP
        </button>
      </template>
      <EmptyState
        v-if="loading && !servers.length"
        compact
        :icon="LoaderCircle"
        busy
        title="正在读取 MCP 配置…"
      />
      <EmptyState
        v-else-if="!filteredServers.length"
        compact
        :icon="Plug"
        :title="needle ? '没有匹配的 MCP 服务' : '尚未添加 MCP 服务'"
        :description="needle ? '' : '可手动添加命令或 HTTP 地址，也可以从下方市场安装。'"
      />
      <div v-for="server in filteredServers" :key="server.id" class="mcp-item">
        <SettingRow
          :title="server.name"
          :description="server.kind === 'stdio' ? '本地进程' : '远程 HTTP'"
        >
          <template #leading
            ><IconTile :size="34"><Plug :size="18" /></IconTile
          ></template>
          <code class="mcp-target" :title="server.target">{{ server.target }}</code>
          <span v-if="tools[server.id]" class="badge">{{ tools[server.id].length }} 个工具</span>
          <UiSwitch
            :checked="server.enabled"
            :label="server.name + '启用'"
            :disabled="!!busy"
            @toggle="toggle(server)"
          />
          <button
            class="btn ghost icon"
            :aria-label="server.name + '操作'"
            :aria-expanded="open === server.id"
            @click="open = open === server.id ? '' : server.id"
          >
            <Wrench :size="16" />
          </button>
        </SettingRow>
        <div v-if="open === server.id" class="mcp-actions">
          <button class="btn small" :disabled="!!busy" @click="probe(server)">测试连接</button>
          <button class="btn small" :disabled="!!busy" @click="draft = draftFrom(server)">
            编辑
          </button>
          <button class="btn ghost small danger" :disabled="!!busy" @click="remove(server)">
            移除
          </button>
        </div>
        <ul v-if="tools[server.id]?.length" class="mcp-tools">
          <li v-for="tool in tools[server.id]" :key="tool.name">
            <strong>{{ tool.name }}</strong>
            <span>{{ tool.description }}</span>
          </li>
        </ul>
      </div>
    </SettingGroup>

    <form v-else class="mcp-form" @submit.prevent="save">
      <h3>{{ draft.id ? '编辑 MCP 服务' : '添加 MCP 服务' }}</h3>
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
          <button
            type="button"
            class="btn ghost small"
            @click="draft.vars.push({ key: '', value: '' })"
          >
            <Plus :size="13" />添加
          </button>
        </div>
        <div v-for="(entry, index) in draft.vars" :key="index" class="mcp-var">
          <input v-model="entry.key" placeholder="KEY" aria-label="名称" />
          <input v-model="entry.value" type="password" placeholder="值" aria-label="值" />
          <button
            type="button"
            class="btn ghost small"
            aria-label="删除此行"
            @click="draft.vars.splice(index, 1)"
          >
            删除
          </button>
        </div>
        <p v-if="draft.vars.length" class="mcp-hint">编辑时留空表示保留已保存的值。</p>
      </div>
      <div class="mcp-form-actions">
        <button type="button" class="btn ghost" :disabled="!!busy" @click="draft = null">
          取消
        </button>
        <button type="submit" class="btn primary" :disabled="!valid || !!busy">
          {{ busy === 'save' ? '保存中…' : '保存' }}
        </button>
      </div>
    </form>

    <SettingGroup
      title="MCP 市场"
      footnote="目录由插件服务端提供；未配置插件服务时只能手动添加。安装前请确认命令与来源可信，本地 MCP 会启动对应进程。"
    >
      <template #actions>
        <button
          class="btn ghost icon"
          title="刷新 MCP 目录"
          aria-label="刷新 MCP 目录"
          :disabled="catalogLoading"
          @click="refreshCatalog"
        >
          <RefreshCw :size="15" :class="{ spin: catalogLoading }" />
        </button>
      </template>
      <EmptyState
        v-if="catalogLoading && !catalogLoaded"
        compact
        :icon="RefreshCw"
        busy
        title="正在获取 MCP 目录…"
      />
      <EmptyState
        v-else-if="!catalog.source"
        compact
        :icon="Server"
        title="尚未配置插件服务"
        description="配置插件服务地址后即可浏览 MCP 市场。"
      >
        <button class="btn small" @click="emit('settings')">插件服务设置</button>
      </EmptyState>
      <EmptyState
        v-else-if="!filteredMarket.length"
        compact
        :icon="Server"
        :title="needle ? '没有匹配的 MCP' : '目录中暂无 MCP'"
      />
      <div v-for="entry in filteredMarket" :key="entry.id" class="mcp-market-item">
        <SettingRow :title="entry.name" :description="entry.description">
          <template #leading
            ><IconTile :size="34"><Server :size="17" /></IconTile
          ></template>
          <button class="btn small" :disabled="!!busy" @click="startInstall(entry)">
            {{ busy === entry.id ? '添加中…' : '添加' }}
          </button>
        </SettingRow>
        <form
          v-if="installing?.entry.id === entry.id"
          class="mcp-install"
          @submit.prevent="install(entry, installing.values)"
        >
          <code class="mcp-target">{{ describe(entry.transport) }}</code>
          <label v-for="field in entry.fields" :key="field.key" class="mcp-field">
            <span>{{ field.label }}</span>
            <input
              v-model="installing.values[field.key]"
              :type="field.secret ? 'password' : 'text'"
              :placeholder="field.placeholder"
            />
          </label>
          <div class="mcp-form-actions">
            <button type="button" class="btn ghost small" @click="installing = null">取消</button>
            <button type="submit" class="btn primary small" :disabled="!!busy">确认添加</button>
          </div>
        </form>
      </div>
    </SettingGroup>
  </div>
</template>
<style scoped>
.mcp-item {
  border-bottom: 1px solid var(--border);
}
.mcp-item:last-child {
  border-bottom: 0;
}
.mcp-target {
  max-width: 280px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: var(--text-tertiary);
  font-size: 12px;
}
.mcp-actions {
  display: flex;
  gap: 8px;
  padding: 0 12px 14px 62px;
}
.mcp-tools {
  margin: 0;
  padding: 0 12px 14px 62px;
  list-style: none;
  display: grid;
  gap: 6px;
}
.mcp-tools li {
  display: flex;
  gap: 10px;
  font-size: 12px;
  color: var(--text-tertiary);
}
.mcp-tools strong {
  color: var(--text);
  font-weight: 600;
}
.mcp-form {
  display: grid;
  gap: 14px;
  padding: 18px 14px;
  border: 1px solid var(--border);
  border-radius: 14px;
  margin-bottom: 30px;
}
.mcp-form h3 {
  margin: 0;
  font-size: 15px;
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
  border: 1px solid var(--border);
  border-radius: 9px;
  background: var(--surface);
  color: var(--text);
  font: inherit;
  font-size: 13px;
  padding: 8px 10px;
  resize: vertical;
}
.mcp-field input:focus,
.mcp-field textarea:focus,
.mcp-var input:focus {
  outline: 2px solid var(--accent-soft);
  outline-offset: -1px;
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
.mcp-form-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
}
.mcp-install {
  display: grid;
  gap: 12px;
  padding: 0 12px 16px 62px;
}
</style>
