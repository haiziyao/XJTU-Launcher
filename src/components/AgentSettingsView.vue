<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { Bot, LoaderCircle, Plug, Wrench } from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { AgentToolInfo } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SegmentedTabs from './ui/SegmentedTabs.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import ProvidersView from './ProvidersView.vue';
import McpSettings from './McpSettings.vue';

const emit = defineEmits<{ agent: [] }>();
const tabs = [
  { id: 'models', title: '模型服务' },
  { id: 'tools', title: '接入工具' },
  { id: 'mcp', title: 'MCP 服务' },
  { id: 'market', title: 'MCP 市场' },
];
const tab = ref('models');
const tools = ref<AgentToolInfo[]>([]);
const loading = ref(false);
const error = ref('');
const campusTools = computed(() => tools.value.filter((tool) => tool.source === 'campus'));
const mcpTools = computed(() => tools.value.filter((tool) => tool.source === 'mcp'));

async function loadTools() {
  if (loading.value) return;
  loading.value = true;
  error.value = '';
  try {
    tools.value = await bridge.agentTools();
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}
watch(tab, (value) => {
  if (value === 'tools' && !tools.value.length) void loadTools();
});
onMounted(() => void loadTools());
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="Agent 设置">
      <template #actions>
        <button class="btn" @click="emit('agent')"><Bot :size="14" />打开 Agent</button>
      </template>
    </PageHeader>
    <SegmentedTabs v-model="tab" :tabs="tabs" label="Agent 设置分类" />
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <div v-if="tab === 'models'" role="tabpanel">
      <ProvidersView />
    </div>
    <div v-else-if="tab === 'tools'" role="tabpanel">
      <EmptyState v-if="loading && !tools.length" :icon="LoaderCircle" busy title="正在读取工具…" />
      <template v-else>
        <SettingGroup
          title="校园工具"
          footnote="校园工具使用当前登录账号查询；插件被停用或账号失效时，工具会返回明确错误。"
        >
          <SettingRow
            v-for="tool in campusTools"
            :key="tool.name"
            :title="tool.name"
            :description="tool.description"
          >
            <template #leading
              ><IconTile :size="34"><Wrench :size="17" /></IconTile
            ></template>
          </SettingRow>
          <EmptyState
            v-if="!campusTools.length && !loading"
            compact
            :icon="Wrench"
            title="没有可用的校园工具"
          />
        </SettingGroup>
        <SettingGroup
          title="MCP 工具"
          footnote="MCP 工具随启用的 MCP 服务加载；本页只列出已内置的工具定义。"
        >
          <SettingRow
            v-for="tool in mcpTools"
            :key="tool.name"
            :title="tool.name"
            :description="tool.description"
          >
            <template #leading
              ><IconTile :size="34"><Plug :size="17" /></IconTile
            ></template>
          </SettingRow>
          <EmptyState
            v-if="!mcpTools.length"
            compact
            :icon="Plug"
            title="尚未启用 MCP 服务"
            description="在“MCP 服务”中添加并启用后，工具会自动加入 Agent。"
          >
            <button class="btn small" @click="tab = 'mcp'">前往 MCP 服务</button>
          </EmptyState>
        </SettingGroup>
      </template>
    </div>
    <McpSettings v-else-if="tab === 'mcp'" :search="''" section="servers" />
    <McpSettings v-else :search="''" section="market" />
  </section>
</template>
