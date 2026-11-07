<script setup lang="ts">
import { ref } from 'vue';
import { Bot, PanelLeft } from 'lucide-vue-next';
import type { AppConfig } from '../types';
import PageHeader from './ui/PageHeader.vue';
import PageTabs from './ui/PageTabs.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import IconTile from './ui/IconTile.vue';
import UiButton from './ui/UiButton.vue';
import UiSwitch from './ui/UiSwitch.vue';
import ProvidersView from './ProvidersView.vue';
import McpSettings from './McpSettings.vue';
import AgentPresets from './AgentPresets.vue';
import AgentMemories from './AgentMemories.vue';
import McpExposeSettings from './McpExposeSettings.vue';

const props = defineProps<{ config: AppConfig; saving: boolean }>();
const emit = defineEmits<{ agent: []; update: [config: AppConfig] }>();
const tabs = [
  { id: 'basic', title: '基础设置' },
  { id: 'abilities', title: '能力设置' },
  { id: 'mcp', title: 'MCP 能力' },
];
const tab = ref('basic');

function toggleAgentSidebar() {
  emit('update', {
    ...props.config,
    agentSidebarVisible: !props.config.agentSidebarVisible,
  });
}
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="Agent 设置">
      <template #actions>
        <UiButton size="sm" @click="emit('agent')"><Bot :size="14" />打开 Agent</UiButton>
      </template>
    </PageHeader>
    <PageTabs v-model="tab" :tabs="tabs" label="Agent 设置分类" />

    <div v-if="tab === 'basic'" role="tabpanel">
      <SettingGroup title="侧边栏">
        <SettingRow title="在侧边栏显示 Agent">
          <template #leading>
            <IconTile tone="blue" :size="30"><PanelLeft :size="16" /></IconTile>
          </template>
          <UiSwitch
            :checked="config.agentSidebarVisible"
            label="在侧边栏显示 Agent"
            :disabled="saving"
            @toggle="toggleAgentSidebar"
          />
        </SettingRow>
      </SettingGroup>
      <McpExposeSettings
        title="MCP 模式"
        :config="config"
        :saving="saving"
        @update="emit('update', $event)"
      />
    </div>

    <div v-else-if="tab === 'abilities'" role="tabpanel">
      <ProvidersView />
      <McpSettings />
      <AgentPresets kind="skill" />
      <AgentPresets kind="role" />
      <AgentPresets kind="task" />
      <AgentMemories />
    </div>

    <McpExposeSettings
      v-else
      mode="tools"
      title="MCP 能力"
      :config="config"
      :saving="saving"
      @update="emit('update', $event)"
    />
  </section>
</template>
