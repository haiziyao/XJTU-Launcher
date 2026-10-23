<script setup lang="ts">
import { computed, ref } from 'vue';
import { Check, Download, Home, Package, Pipette, SlidersHorizontal } from 'lucide-vue-next';
import type { AppConfig } from '../types';
import { DEFAULT_SIDEBAR_WIDTH } from '../domain';
import { ACCENT_PRESETS, normalizeAccent } from '../theme';
import { PETS, normalizePet, type PetId } from '../pets';
import PetArt from './PetArt.vue';
import PluginSourceSettings from './PluginSourceSettings.vue';
import AppUpdateSettings from './AppUpdateSettings.vue';
import SegmentedTabs from './ui/SegmentedTabs.vue';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';

const props = defineProps<{
  config: AppConfig;
  saving: boolean;
  initialSection?: 'appearance' | 'home' | 'plugins' | 'updates';
  saveSource: (source: string | null) => Promise<boolean>;
}>();
const emit = defineEmits<{ update: [config: AppConfig] }>();
const tab = ref<string>(props.initialSection || 'appearance');
const accent = computed(() => normalizeAccent(props.config.accent));
const customAccent = computed(() => !ACCENT_PRESETS.some((p) => p.value === accent.value));
const currentPet = computed(() => normalizePet(props.config.pet));
function setPet(id: PetId) {
  if (id !== currentPet.value) emit('update', { ...props.config, pet: id });
}
function setAccent(value: string) {
  const next = normalizeAccent(value);
  if (next !== accent.value) emit('update', { ...props.config, accent: next });
}
const sections = [
  { id: 'appearance', title: '外观与交互', icon: SlidersHorizontal },
  { id: 'home', title: '主页', icon: Home },
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
                    <Check v-if="accent === preset.value" :size="12" aria-hidden="true" />
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
                    <Check v-if="customAccent" :size="12" aria-hidden="true" />
                    <Pipette v-else :size="12" aria-hidden="true" />
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
                  :disabled="saving || config.sidebarWidth === DEFAULT_SIDEBAR_WIDTH"
                  @click="emit('update', { ...config, sidebarWidth: DEFAULT_SIDEBAR_WIDTH })"
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
/* 应用设置 is a flat page with many rows; full-strength separators turn it into
   a stack of bright lines, so keep the row dividers faint here. */
.settings-page :deep(.setting-card > * + *::before) {
  border-top-color: rgba(15, 23, 42, 0.05);
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
