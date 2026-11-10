<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import {
  IconCheck,
  IconDownload,
  IconServer,
  IconHome,
  IconColorPicker,
  IconAdjustmentsHorizontal,
} from '@tabler/icons-vue';
import type { AppConfig } from '../types';
import { DEFAULT_SIDEBAR_WIDTH } from '../domain';
import {
  ACCENT_PRESETS,
  WALLPAPER_MODES,
  normalizeAccent,
  normalizeWallpaperDisplay,
  wallpaperMode,
  wallpaperPanelOpacity,
} from '../theme';
import { PETS, normalizePet, type PetId } from '../pets';
import PetArt from './PetArt.vue';
import AppUpdateSettings from './AppUpdateSettings.vue';
import StorageSettings from './StorageSettings.vue';
import PageTabs from './ui/PageTabs.vue';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';
import SegmentedTabs from './ui/SegmentedTabs.vue';

import UiButton from './ui/UiButton.vue';
const props = defineProps<{
  config: AppConfig;
  saving: boolean;
  initialSection?: 'appearance' | 'home' | 'updates' | 'storage';
}>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const tab = ref<string>(props.initialSection || 'appearance');
watch(
  () => props.initialSection,
  (section) => {
    if (section) tab.value = section;
  },
);
const accent = computed(() => normalizeAccent(props.config.accent));
const customAccent = computed(() => !ACCENT_PRESETS.some((p) => p.value === accent.value));
const currentPet = computed(() => normalizePet(props.config.pet));
// The wallpaper preset also drives the top-right switch on the main window.
const wallpaperDisplay = computed(() => normalizeWallpaperDisplay(props.config.wallpaperDisplay));
const wallpaperTabs = WALLPAPER_MODES.map((mode) => ({ id: mode.id, title: mode.label }));
const wallpaperPanel = computed(() =>
  wallpaperPanelOpacity(props.config.wallpaperPanel, wallpaperMode(wallpaperDisplay.value)),
);
function setWallpaperDisplay(id: string) {
  const next = normalizeWallpaperDisplay(id);
  if (next !== wallpaperDisplay.value) emit('update', { ...props.config, wallpaperDisplay: next });
}
function setWallpaperPanel(value: number) {
  if (!Number.isFinite(value)) return;
  emit('update', {
    ...props.config,
    wallpaperPanel: { ...props.config.wallpaperPanel, [wallpaperDisplay.value]: value },
  });
}
function setPet(id: PetId) {
  if (id !== currentPet.value) emit('update', { ...props.config, pet: id });
}
function setAccent(value: string) {
  const next = normalizeAccent(value);
  if (next !== accent.value) emit('update', { ...props.config, accent: next });
}
const sections = [
  { id: 'appearance', title: '外观与交互', icon: IconAdjustmentsHorizontal },
  { id: 'home', title: '主页', icon: IconHome },
  { id: 'storage', title: '存储', icon: IconServer },
  { id: 'updates', title: '软件更新', icon: IconDownload },
] as const;
</script>
<template>
  <section class="desktop-page settings-page">
    <PageHeader title="应用设置" />
    <div class="settings-layout">
      <PageTabs v-model="tab" :tabs="sections" label="设置分类" />
      <div class="settings-detail">
        <Transition name="section" mode="out-in">
          <div v-if="tab === 'appearance'" key="appearance">
            <SettingGroup title="主题">
              <SettingRow title="主题色" description="用于侧边栏选中项和界面的强调色">
                <div class="accent-picker" role="group" aria-label="主题色">
                  <button
                    v-for="preset in ACCENT_PRESETS"
                    :key="preset.value"
                    type="button"
                    class="accent-swatch"
                    :class="{ active: accent === preset.value }"
                    :style="{ '--swatch': preset.value }"
                    :title="preset.name"
                    :aria-label="preset.name"
                    :aria-pressed="accent === preset.value"
                    :disabled="saving"
                    @click="setAccent(preset.value)"
                  >
                    <IconCheck v-if="accent === preset.value" :size="12" aria-hidden="true" />
                  </button>
                  <label
                    class="accent-swatch custom"
                    :class="{ active: customAccent, picked: customAccent }"
                    :style="{ '--swatch': accent }"
                    title="自定义颜色"
                  >
                    <input
                      type="color"
                      :value="accent"
                      :disabled="saving"
                      aria-label="自定义主题色"
                      @change="setAccent(($event.target as HTMLInputElement).value)"
                    />
                    <IconCheck v-if="customAccent" :size="12" aria-hidden="true" />
                    <IconColorPicker v-else :size="12" aria-hidden="true" />
                  </label>
                </div>
              </SettingRow>
            </SettingGroup>
            <SettingGroup title="侧边栏宠物">
              <div class="pet-picker" role="radiogroup" aria-label="侧边栏宠物">
                <button
                  v-for="pet in PETS"
                  :key="pet.id"
                  type="button"
                  class="pet-option"
                  :class="{ active: currentPet === pet.id }"
                  role="radio"
                  :aria-checked="currentPet === pet.id"
                  :disabled="saving"
                  @click="setPet(pet.id)"
                >
                  <span class="pet-option-art"><PetArt :variant="pet.id" /></span>
                  <span>{{ pet.name }}</span>
                </button>
              </div>
            </SettingGroup>
            <SettingGroup title="页面标签">
              <SettingRow
                title="标签底框"
                description="给页面顶部的横向标签加上浅色圆角底框，壁纸下更容易看清"
              >
                <UiSwitch
                  :checked="config.pageTabSurfaces"
                  label="标签底框"
                  :disabled="saving"
                  @toggle="emit('update', { ...config, pageTabSurfaces: !config.pageTabSurfaces })"
                />
              </SettingRow>
            </SettingGroup>
            <SettingGroup title="壁纸显示">
              <SettingRow title="显示样式" description="右上角按钮会在这三档之间循环">
                <SegmentedTabs
                  :model-value="wallpaperDisplay"
                  :tabs="wallpaperTabs"
                  label="壁纸显示样式"
                  compact
                  @update:model-value="setWallpaperDisplay"
                />
              </SettingRow>
              <SettingRow
                title="面板不透明度"
                :description="
                  config.wallpaper ? '只影响壁纸下的蒙版、面板与控件' : '设置壁纸后生效'
                "
              >
                <div class="wallpaper-opacity">
                  <input
                    type="range"
                    min="40"
                    max="100"
                    step="2"
                    :value="wallpaperPanel"
                    :disabled="saving || !config.wallpaper"
                    :aria-label="`面板不透明度 ${wallpaperPanel}%`"
                    @input="setWallpaperPanel(Number(($event.target as HTMLInputElement).value))"
                  />
                  <span class="setting-value">{{ wallpaperPanel }}%</span>
                </div>
              </SettingRow>
            </SettingGroup>
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
                <UiButton
                  size="sm"
                  :disabled="saving || config.sidebarWidth === DEFAULT_SIDEBAR_WIDTH"
                  @click="emit('update', { ...config, sidebarWidth: DEFAULT_SIDEBAR_WIDTH })"
                >
                  恢复默认
                </UiButton>
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
          <AppUpdateSettings
            v-else-if="tab === 'updates'"
            key="updates"
            :config="config"
            :saving="saving"
            @update="emit('update', $event)"
          />
          <StorageSettings
            v-else
            key="storage"
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
.accent-picker {
  display: flex;
  align-items: center;
  gap: 8px;
}
.accent-swatch {
  position: relative;
  display: grid;
  place-items: center;
  width: 22px;
  height: 22px;
  padding: 0;
  border: 0;
  border-radius: 50%;
  background: var(--swatch);
  color: #fff;
  cursor: pointer;
  box-shadow: inset 0 0 0 1px rgba(15, 23, 42, 0.12);
  transition:
    transform var(--duration-fast) var(--ease-out),
    box-shadow var(--duration) ease;
}
.accent-swatch:hover:not(:disabled) {
  transform: scale(1.12);
}
.accent-swatch.active {
  box-shadow:
    0 0 0 2px var(--surface),
    0 0 0 4px var(--swatch);
}
.accent-swatch.custom {
  overflow: hidden;
  background: var(--surface);
  color: var(--text-tertiary);
  box-shadow: inset 0 0 0 1px var(--border-strong);
}
.accent-swatch.custom.picked {
  background: var(--swatch);
  color: var(--accent-contrast);
  box-shadow: inset 0 0 0 1px rgba(15, 23, 42, 0.12);
}
.accent-swatch.custom.active {
  box-shadow:
    0 0 0 2px var(--surface),
    0 0 0 4px var(--swatch);
}
.accent-swatch.custom input {
  position: absolute;
  inset: 0;
  width: 100%;
  height: 100%;
  padding: 0;
  border: 0;
  opacity: 0;
  cursor: pointer;
}
.accent-swatch.custom input:disabled {
  cursor: default;
}
.pet-picker {
  display: grid;
  grid-template-columns: repeat(6, minmax(0, 1fr));
  gap: 6px;
  padding: 12px;
}
.pet-option {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  padding: 8px 4px 6px;
  border: 1px solid transparent;
  border-radius: var(--radius);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  cursor: pointer;
  transition:
    background var(--duration-fast) ease,
    border-color var(--duration) ease,
    color var(--duration) ease;
}
.pet-option:hover:not(:disabled):not(.active) {
  background: var(--accent-wash);
}
.pet-option.active {
  border-color: var(--accent-outline);
  background: var(--accent-tint);
  color: var(--accent-text);
}
.pet-option-art {
  display: block;
  width: 40px;
  height: 40px;
}
.wallpaper-opacity {
  display: flex;
  align-items: center;
  gap: 10px;
  min-width: 180px;
}
.wallpaper-opacity input[type='range'] {
  flex: 1;
  min-width: 120px;
  accent-color: var(--accent);
}
.settings-layout {
  display: block;
}
.settings-page :deep(.update-settings .setting-card) {
  border: 0;
  box-shadow: none;
  border-radius: 0;
  background: transparent;
}
.settings-page :deep(.update-settings .setting-row) {
  min-height: 76px;
  padding: 18px 12px;
}
</style>
