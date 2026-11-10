<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { IconCopy, IconPlugConnected, IconRefresh } from '@tabler/icons-vue';
import type { AppConfig, ExposeAccess, ExposeStatus, ExposeTool } from '../../types';
import { bridge } from '../../shared/bridge';
import SettingGroup from '../../components/ui/SettingGroup.vue';
import SettingRow from '../../components/ui/SettingRow.vue';
import IconTile from '../../components/ui/IconTile.vue';
import UiButton from '../../components/ui/UiButton.vue';
import UiSwitch from '../../components/ui/UiSwitch.vue';

/**
 * The local MCP endpoint. The personal page and the Agent basic tab show the
 * switch and connection details (port, address, command, tool count); the
 * Agent "MCP 能力" tab lists the tools instead. All read and write the same
 * config fields.
 */
const props = withDefaults(
  defineProps<{
    config: AppConfig;
    saving: boolean;
    title?: string;
    /** `details` shows how to connect; `tools` shows the per-tool switches. */
    mode?: 'details' | 'tools';
  }>(),
  {
    title: '开放接口',
    mode: 'details',
  },
);
const emit = defineEmits<{ update: [config: AppConfig] }>();

const status = ref<ExposeStatus>({
  running: false,
  enabled: false,
  port: props.config.mcpServerPort,
  address: `http://127.0.0.1:${props.config.mcpServerPort}/mcp`,
  error: null,
});
const tools = ref<ExposeTool[]>([]);
const busy = ref(false);
/** Avoids flashing "not running" before the first status reply arrives. */
const ready = ref(false);
const serverError = ref('');
const copied = ref('');
const portDraft = ref(String(props.config.mcpServerPort));
let copyTimer: number | undefined;
const command = computed(() => `claude mcp add --transport http xjtu ${status.value.address}`);
const portChanged = computed(() => {
  const port = Number(portDraft.value);
  if (!Number.isInteger(port) || port < 1024 || port > 65535) return false;
  return port !== props.config.mcpServerPort || !status.value.running;
});
const portInvalid = computed(() => {
  if (portDraft.value.trim() === '') return false;
  const port = Number(portDraft.value);
  return !Number.isInteger(port) || port < 1024 || port > 65535;
});
const toolGroups = computed(() => {
  const order: string[] = [];
  const groups = new Map<string, ExposeTool[]>();
  for (const tool of tools.value) {
    if (!groups.has(tool.category)) {
      groups.set(tool.category, []);
      order.push(tool.category);
    }
    groups.get(tool.category)?.push(tool);
  }
  return order.map((title) => ({ title, tools: groups.get(title) ?? [] }));
});
const accessCopy: Record<ExposeAccess, string> = {
  read: '读',
  write: '写',
  update: '改',
  delete: '删',
};
/** Explained once in the first group's header, like a chart legend. */
const legendKinds: ExposeAccess[] = ['read', 'write', 'update', 'delete'];

async function loadServer() {
  try {
    status.value = await bridge.exposeStatus();
    portDraft.value = String(status.value.port);
  } catch (e) {
    serverError.value = String(e);
  } finally {
    ready.value = true;
  }
}
async function loadTools() {
  try {
    tools.value = await bridge.exposeTools();
  } catch (e) {
    serverError.value = String(e);
  }
}
/** The listener may fall back to a free port, so the saved port follows it. */
function adopt(next: ExposeStatus, enabled: boolean) {
  status.value = next;
  portDraft.value = String(next.port);
  emit('update', { ...props.config, mcpServerEnabled: enabled, mcpServerPort: next.port });
}
async function toggleServer() {
  if (busy.value) return;
  busy.value = true;
  serverError.value = '';
  const enabled = !props.config.mcpServerEnabled;
  try {
    adopt(await bridge.setExposeEnabled(enabled), enabled);
  } catch (e) {
    // The config keeps the previous value, so the switch falls back with it.
    serverError.value = String(e);
    await loadServer();
  } finally {
    busy.value = false;
  }
}
/** Startup restore can fail before the page exists; let the user try again. */
async function retryStart() {
  if (busy.value) return;
  busy.value = true;
  serverError.value = '';
  try {
    adopt(await bridge.setExposeEnabled(true), true);
  } catch (e) {
    serverError.value = String(e);
    await loadServer();
  } finally {
    busy.value = false;
  }
}
/** Restart on the typed port; a busy one is replaced by a free port. */
async function applyPort() {
  if (busy.value || !portChanged.value) return;
  busy.value = true;
  serverError.value = '';
  try {
    adopt(await bridge.setExposePort(Number(portDraft.value)), true);
  } catch (e) {
    serverError.value = String(e);
    await loadServer();
  } finally {
    busy.value = false;
  }
}
function isToolOpen(name: string) {
  return !props.config.mcpServerDisabledTools.includes(name);
}
function onToolToggle(name: string) {
  const next = new Set(props.config.mcpServerDisabledTools);
  if (isToolOpen(name)) next.add(name);
  else next.delete(name);
  emit('update', { ...props.config, mcpServerDisabledTools: [...next] });
}
async function copy(text: string, key: string) {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // WKWebView may refuse the async clipboard without a permission grant.
    const area = document.createElement('textarea');
    area.value = text;
    area.setAttribute('readonly', '');
    area.style.position = 'fixed';
    area.style.opacity = '0';
    document.body.append(area);
    area.select();
    const copiedNow = document.execCommand('copy');
    area.remove();
    if (!copiedNow) {
      serverError.value = '无法复制到剪贴板';
      return;
    }
  }
  copied.value = key;
  window.clearTimeout(copyTimer);
  copyTimer = window.setTimeout(() => (copied.value = ''), 1600);
}
onMounted(() => {
  void loadServer();
  void loadTools();
});
onBeforeUnmount(() => window.clearTimeout(copyTimer));
</script>
<template>
  <!-- The tool list lives on its own page, so only the details copy carries
       the group heading; the switch card above the tool sections stays bare. -->
  <SettingGroup :title="mode === 'details' ? title : undefined">
    <SettingRow title="MCP 模式">
      <template #leading>
        <IconTile tone="blue" :size="30"><IconPlugConnected :size="16" /></IconTile>
      </template>
      <UiSwitch
        :checked="config.mcpServerEnabled"
        label="MCP 模式"
        :disabled="saving || busy"
        @toggle="toggleServer"
      />
    </SettingRow>

    <template v-if="config.mcpServerEnabled">
      <div v-if="ready && !status.running" class="connect-error">
        <span>{{ status.error || serverError || 'MCP 服务未启动' }}</span>
        <UiButton variant="ghost" size="sm" :disabled="busy" @click="retryStart">
          <IconRefresh :size="13" />重试
        </UiButton>
      </div>
      <div v-else-if="ready" class="connect-detail">
        <div class="connect-line">
          <span class="connect-label">端口</span>
          <input
            v-model="portDraft"
            class="text-field connect-port"
            inputmode="numeric"
            maxlength="5"
            aria-label="MCP 端口"
            :disabled="busy"
          />
          <UiButton size="sm" :disabled="busy || !portChanged" @click="applyPort">应用</UiButton>
        </div>
        <p v-if="portInvalid" class="connect-hint" aria-live="polite">端口需在 1024–65535 之间。</p>
        <div class="connect-line">
          <span class="connect-label">地址</span>
          <code>{{ status.address }}</code>
          <UiButton variant="ghost" size="sm" @click="copy(status.address, 'address')">
            <IconCopy :size="13" />{{ copied === 'address' ? '已复制' : '复制' }}
          </UiButton>
        </div>
        <div class="connect-line">
          <span class="connect-label">命令</span>
          <code>{{ command }}</code>
          <UiButton variant="ghost" size="sm" @click="copy(command, 'command')">
            <IconCopy :size="13" />{{ copied === 'command' ? '已复制' : '复制' }}
          </UiButton>
        </div>
        <p class="connect-tools">共 {{ tools.length }} 个工具</p>
      </div>
    </template>
    <p v-else-if="serverError" class="connect-error">{{ serverError }}</p>
  </SettingGroup>

  <template v-if="mode === 'tools'">
    <SettingGroup v-for="(group, index) in toolGroups" :key="group.title" :title="group.title">
      <template v-if="index === 0" #actions>
        <span class="tool-legend">
          <span v-for="kind in legendKinds" :key="kind">
            <i class="tool-mark" :class="kind" aria-hidden="true"></i>{{ accessCopy[kind] }}
          </span>
        </span>
      </template>
      <SettingRow
        v-for="tool in group.tools"
        :key="tool.name"
        :title="tool.title"
        :description="tool.summary"
      >
        <span class="tool-access">
          <i
            v-for="kind in tool.access"
            :key="kind"
            class="tool-mark"
            :class="kind"
            role="img"
            :aria-label="accessCopy[kind]"
          ></i>
        </span>
        <UiSwitch
          :checked="isToolOpen(tool.name)"
          :label="`开放${tool.title}`"
          :disabled="saving"
          @toggle="onToolToggle(tool.name)"
        />
      </SettingRow>
    </SettingGroup>
  </template>
</template>
<style scoped>
.connect-hint {
  margin: 4px 0 0 42px;
  font-size: 12px;
  color: var(--danger);
}
</style>
