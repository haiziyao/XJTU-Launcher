<script setup lang="ts">
import { ref } from 'vue';
import { SlidersHorizontal, Home, Server, Package, Download } from 'lucide-vue-next';
import type { AppConfig } from '../types';
import PluginSourceSettings from './PluginSourceSettings.vue';
import AppUpdateSettings from './AppUpdateSettings.vue';
import SegmentedTabs from './ui/SegmentedTabs.vue';
import ProvidersView from './ProvidersView.vue';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';

const props = defineProps<{
  config: AppConfig;
  saving: boolean;
  initialSection?: 'appearance' | 'home' | 'models' | 'plugins' | 'updates';
  saveSource: (source: string | null) => Promise<boolean>;
}>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const tab = ref<string>(props.initialSection || 'appearance');
const sections = [
  { id: 'appearance', title: '外观与交互', icon: SlidersHorizontal },
  { id: 'home', title: '主页', icon: Home },
  { id: 'models', title: '模型服务', icon: Server },
  { id: 'plugins', title: '插件服务', icon: Package },
  { id: 'updates', title: '软件更新', icon: Download },
] as const;
</script>
<template>
  <section class="desktop-page settings-page">
    <PageHeader title="应用设置" />
    <div class="settings-layout">
      <SegmentedTabs v-model="tab" :tabs="sections" label="设置分类" />
      <div class="settings-detail">
        <Transition name="section" mode="out-in">
          <div v-if="tab === 'appearance'" key="appearance">
            <SettingGroup title="动态效果">
              <SettingRow title="减少动态效果" description="关闭导航、侧栏和页面的过渡动画">
                <UiSwitch
                  :checked="config.reduceMotion"
                  label="减少动态效果"
                  :disabled="saving"
                  @toggle="emit('update', { ...config, reduceMotion: !config.reduceMotion })"
                />
              </SettingRow>
            </SettingGroup>
            <SettingGroup title="侧栏">
              <SettingRow title="展开宽度" description="拖动侧栏右侧分隔线调整，双击分隔线恢复默认">
                <span class="setting-value">{{ config.sidebarWidth }} px</span>
                <button
                  class="btn small"
                  :disabled="saving || config.sidebarWidth === 196"
                  @click="emit('update', { ...config, sidebarWidth: 196 })"
                >
                  恢复默认
                </button>
              </SettingRow>
            </SettingGroup>
          </div>
          <div v-else-if="tab === 'home'" key="home">
            <SettingGroup title="日期与时间">
              <SettingRow title="时钟显示秒" description="在主页时间中显示秒数">
                <UiSwitch
                  :checked="config.showSeconds"
                  label="时钟显示秒"
                  :disabled="saving"
                  @toggle="emit('update', { ...config, showSeconds: !config.showSeconds })"
                />
              </SettingRow>
            </SettingGroup>
          </div>
          <ProvidersView v-else-if="tab === 'models'" key="models" />
          <PluginSourceSettings
            v-else-if="tab === 'plugins'"
            key="plugins"
            :config="config"
            :saving="saving"
            :save-source="saveSource"
            @update="emit('update', $event)"
          />
          <AppUpdateSettings
            v-else
            key="updates"
            :config="config"
            :saving="saving"
            @update="emit('update', $event)"
          />
        </Transition>
      </div>
    </div>
  </section>
</template>

<style scoped>
.settings-layout {
  display: block;
}
.settings-page :deep(.segmented-tabs) {
  background: transparent;
  display: flex;
  flex-wrap: wrap;
  border-bottom: 1px solid var(--border);
  border-radius: 0;
  padding: 0 0 14px;
  gap: 10px;
}
.settings-page :deep(.segmented-tabs button) {
  min-width: 0;
  padding: 6px 12px;
}
.settings-page :deep(.segmented-tabs button.active) {
  color: var(--accent);
  box-shadow: none;
  background: var(--accent-soft);
}
.settings-page :deep(.source-settings .setting-card),
.settings-page :deep(.update-settings .setting-card) {
  border: 0;
  box-shadow: none;
  border-radius: 0;
  background: transparent;
}
.settings-page :deep(.source-settings .setting-row),
.settings-page :deep(.update-settings .setting-row) {
  min-height: 76px;
  padding: 18px 12px;
}
</style>
