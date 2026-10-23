<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
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
import AgentPresets from './AgentPresets.vue';
import AgentMemories from './AgentMemories.vue';

import UiButton from './ui/UiButton.vue';
const emit = defineEmits<{ agent: [] }>();
const tabs = [
  { id: 'models', title: '模型服务' },
  { id: 'tools', title: '接入工具' },
  { id: 'presets', title: '角色预设' },
  { id: 'memory', title: '全局记忆' },
  { id: 'market', title: '市场' },
];
const tab = ref('models');
const tools = ref<AgentToolInfo[]>([]);
const loading = ref(false);
const error = ref('');
const campusTools = computed(() => tools.value.filter((tool) => tool.source === 'campus'));

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
onMounted(() => void loadTools());
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="Agent 设置">
      <template #actions>
        <UiButton @click="emit('agent')"><Bot :size="14" />打开 Agent</UiButton>
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
      <SettingGroup
        title="校园工具"
        footnote="校园工具使用当前登录账号查询；插件被停用或账号失效时，工具会返回明确错误。"
      >
        <EmptyState
          v-if="loading && !tools.length"
          compact
          :icon="LoaderCircle"
          busy
          title="正在读取工具…"
        />
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
      <AgentPresets kind="skill" />
      <McpSettings :search="''" section="servers" />
      <SettingGroup
        title="MCP 工具"
        footnote="MCP 工具会在 Agent 使用已启用的 MCP 服务时自动加入。"
      >
        <EmptyState
          compact
          :icon="Plug"
          title="随 MCP 服务加载"
          description="在下方 MCP 服务中启用后，工具会出现在对话里。"
        />
      </SettingGroup>
    </div>

    <div v-else-if="tab === 'presets'" role="tabpanel">
      <AgentPresets kind="role" />
      <AgentPresets kind="task" />
    </div>

    <AgentMemories v-else-if="tab === 'memory'" />

    <McpSettings v-else :search="''" section="market" />
  </section>
</template>
