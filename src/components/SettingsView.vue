<script setup lang="ts">
import { ref } from 'vue';
import { SlidersHorizontal, Home, Server } from 'lucide-vue-next';
import type { AppConfig } from '../types';
import ProvidersView from './ProvidersView.vue';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
import { useSlidingIndicator } from '../useSlidingIndicator';
const props = defineProps<{
  config: AppConfig;
  saving: boolean;
  initialSection?: 'appearance' | 'home' | 'models';
}>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const tab = ref(props.initialSection || 'appearance');
const sections = [
  { id: 'appearance', title: '外观与交互', icon: SlidersHorizontal },
  { id: 'home', title: '主页', icon: Home },
  { id: 'models', title: '模型服务', icon: Server },
] as const;
const sectionNav = ref<HTMLElement | null>(null);
const indicator = useSlidingIndicator(sectionNav, () => tab.value);
</script>
<template>
  <section class="desktop-page settings-page">
    <PageHeader title="应用设置" />
    <div class="settings-layout">
      <nav ref="sectionNav" class="settings-sections" aria-label="设置分类">
        <span
          class="settings-indicator"
          :class="{ animated: indicator.animated.value }"
          :style="indicator.style.value"
          aria-hidden="true"
        ></span>
        <button
          v-for="section in sections"
          :key="section.id"
          :class="{ active: tab === section.id }"
          :data-indicator="section.id"
          :aria-current="tab === section.id ? 'page' : undefined"
          @click="tab = section.id"
        >
          <component :is="section.icon" :size="15" /><span>{{ section.title }}</span>
        </button>
      </nav>
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
          <ProvidersView v-else key="models" />
        </Transition>
      </div>
    </div>
  </section>
</template>
