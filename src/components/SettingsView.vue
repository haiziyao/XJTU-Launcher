<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { Cat, Check, Download, Home, Package, Pipette, SlidersHorizontal } from 'lucide-vue-next';
import type { AppConfig } from '../types';
import { DEFAULT_SIDEBAR_WIDTH } from '../domain';
import { ACCENT_PRESETS, normalizeAccent } from '../theme';
import { PETS, normalizePet, type PetId } from '../pets';
import PetArt from './PetArt.vue';
import PluginSourceSettings from './PluginSourceSettings.vue';
import AppUpdateSettings from './AppUpdateSettings.vue';
import PageTabs from './ui/PageTabs.vue';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import UiSwitch from './ui/UiSwitch.vue';

import UiButton from './ui/UiButton.vue';
import { bridge } from '../bridge';
import deepseekGirl from '../assets/pets/deepseek-girl.webp';
import whalechan from '../assets/pets/deepseek-whalechan.webp';
import gptDragon from '../assets/pets/gpt-dragon-girl.webp';
import claude from '../assets/pets/claude.webp';
const props = defineProps<{
  config: AppConfig;
  saving: boolean;
  initialSection?: 'appearance' | 'home' | 'pet' | 'plugins' | 'updates';
  saveSource: (source: string | null) => Promise<boolean>;
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
  { id: 'pet', title: '桌面宠物', icon: Cat },
  { id: 'plugins', title: '插件服务', icon: Package },
  { id: 'updates', title: '软件更新', icon: Download },
] as const;

const DESKTOP_PETS = [
  { id: 'deepseek-girl', name: 'DeepSeek 娘', file: deepseekGirl, height: 2288 },
  { id: 'deepseek-whalechan', name: '鲸鱼娘', file: whalechan, height: 2288 },
  { id: 'gpt-dragon-girl', name: 'GPT 龙娘', file: gptDragon, height: 2288 },
  { id: 'claude', name: 'Claude', file: claude, height: 1872 },
] as const;

const BAR_STYLES = [
  { id: 'glass-light', name: '玻璃' },
  { id: 'glass-dark', name: '深色玻璃' },
  { id: 'solid', name: '纯白' },
] as const;

const PET_SOURCES = [
  {
    id: 'campus',
    title: '校园通知',
    description: '教务处、学生处等栏目出现新通知时提示',
    off: false,
  },
  { id: 'agent', title: 'Agent 任务', description: '任务完成或出错时提示', off: false },
  {
    id: 'class',
    title: '上课提醒',
    description: '课表只有节次、没有上课时间，暂未接入',
    off: true,
  },
  { id: 'update', title: '更新提示', description: '应用有可用更新时提示', off: false },
] as const;

function setDesktopPet(id: string) {
  if (id !== props.config.petCharacter) emit('update', { ...props.config, petCharacter: id });
}
function setBarStyle(id: string) {
  if (id !== props.config.petBarStyle) emit('update', { ...props.config, petBarStyle: id });
}
function toggleSource(id: string) {
  const next = new Set(props.config.petSources);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  emit('update', { ...props.config, petSources: [...next] });
}
function previewPet() {
  void bridge.petPreview();
}
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
          <div v-else-if="tab === 'pet'" key="pet">
            <SettingGroup title="桌面宠物">
              <SettingRow
                title="最小化时显示桌宠"
                description="主窗口最小化后出现浮动桌宠和通知横条；关闭窗口仍然是关闭"
              >
                <UiSwitch
                  :checked="config.petDesktopEnabled"
                  label="最小化时显示桌宠"
                  :disabled="saving"
                  @toggle="
                    emit('update', { ...config, petDesktopEnabled: !config.petDesktopEnabled })
                  "
                />
              </SettingRow>
              <SettingRow title="角色" description="四只内置角色，之后可继续扩展">
                <div class="pet-picker desk-pet-picker" role="radiogroup" aria-label="桌面宠物角色">
                  <button
                    v-for="item in DESKTOP_PETS"
                    :key="item.id"
                    type="button"
                    class="pet-option"
                    :class="{ active: config.petCharacter === item.id }"
                    role="radio"
                    :aria-checked="config.petCharacter === item.id"
                    :disabled="saving"
                    @click="setDesktopPet(item.id)"
                  >
                    <span class="desk-pet-art" aria-hidden="true">
                      <i
                        :style="{
                          backgroundImage: `url(${item.file})`,
                          backgroundSize: `1536px ${item.height}px`,
                        }"
                      />
                    </span>
                    <span>{{ item.name }}</span>
                  </button>
                </div>
              </SettingRow>
              <SettingRow title="横条风格" description="半透明玻璃、深色玻璃或纯白">
                <div class="desk-styles" role="group" aria-label="横条风格">
                  <button
                    v-for="style in BAR_STYLES"
                    :key="style.id"
                    type="button"
                    class="desk-style"
                    :class="{ active: config.petBarStyle === style.id }"
                    :aria-pressed="config.petBarStyle === style.id"
                    :disabled="saving"
                    @click="setBarStyle(style.id)"
                  >
                    {{ style.name }}
                  </button>
                </div>
              </SettingRow>
              <SettingRow title="只保留桌宠" description="隐藏通知横条，只显示桌宠">
                <UiSwitch
                  :checked="config.petBarHidden"
                  label="只保留桌宠"
                  :disabled="saving"
                  @toggle="emit('update', { ...config, petBarHidden: !config.petBarHidden })"
                />
              </SettingRow>
              <SettingRow title="台词气泡" description="和桌宠互动时冒出一句短台词">
                <UiSwitch
                  :checked="config.petBubbleEnabled"
                  label="台词气泡"
                  :disabled="saving"
                  @toggle="
                    emit('update', { ...config, petBubbleEnabled: !config.petBubbleEnabled })
                  "
                />
              </SettingRow>
            </SettingGroup>
            <SettingGroup title="桌面通知">
              <SettingRow
                v-for="source in PET_SOURCES"
                :key="source.id"
                :title="source.title"
                :description="source.description"
              >
                <span v-if="source.off" class="setting-value">暂不可用</span>
                <UiSwitch
                  v-else
                  :checked="config.petSources.includes(source.id)"
                  :label="source.title"
                  :disabled="saving"
                  @toggle="toggleSource(source.id)"
                />
              </SettingRow>
              <SettingRow title="预览" description="不最小化主窗口，直接查看桌宠效果">
                <UiButton size="sm" :disabled="saving" @click="previewPet">预览桌宠</UiButton>
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
.desk-pet-picker {
  grid-template-columns: repeat(4, minmax(0, 1fr));
  padding: 0;
}
.desk-pet-art {
  position: relative;
  display: block;
  width: 44px;
  height: 44px;
  overflow: hidden;
}
.desk-pet-art i {
  position: absolute;
  left: 0;
  top: 0;
  width: 192px;
  height: 208px;
  background-repeat: no-repeat;
  background-position: 0 0;
  transform: scale(0.21);
  transform-origin: top left;
}
.desk-styles {
  display: flex;
  gap: 6px;
}
.desk-style {
  border: 1px solid var(--border);
  border-radius: 999px;
  background: var(--surface);
  color: var(--text-secondary);
  font: inherit;
  font-size: 11.5px;
  padding: 3px 11px;
  cursor: pointer;
}
.desk-style.active {
  background: var(--accent-soft);
  border-color: transparent;
  color: var(--accent-text);
}
.settings-layout {
  display: block;
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
