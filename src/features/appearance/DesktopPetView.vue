<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { bridge } from '../../shared/bridge';
import type { PetPluginSettings } from '../../types';
import PageHeader from '../../components/ui/PageHeader.vue';
import SettingGroup from '../../components/ui/SettingGroup.vue';
import SettingRow from '../../components/ui/SettingRow.vue';
import UiButton from '../../components/ui/UiButton.vue';
import UiSwitch from '../../components/ui/UiSwitch.vue';
import deepseekGirl from '../../assets/pets/deepseek-girl.webp';
import whalechan from '../../assets/pets/deepseek-whalechan.webp';
import gptDragon from '../../assets/pets/gpt-dragon-girl.webp';
import claude from '../../assets/pets/claude.webp';

const PET_PLUGIN_ID = 'cn.xjtu.desktop-pet';

const DEFAULT_SETTINGS: PetPluginSettings = {
  character: 'deepseek-girl',
  barStyle: 'glass-light',
  barHidden: false,
  bubble: true,
  sources: ['campus', 'agent', 'update'],
  position: null,
};

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

const settings = ref<PetPluginSettings>({ ...DEFAULT_SETTINGS });
const error = ref('');
const saving = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;

function patch(change: Partial<PetPluginSettings>) {
  settings.value = { ...settings.value, ...change };
  scheduleSave();
}

function scheduleSave() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void save(), 260);
}

async function save() {
  if (timer) {
    clearTimeout(timer);
    timer = undefined;
  }
  if (disposed || saving.value) return;
  saving.value = true;
  error.value = '';
  try {
    const stored = await bridge.savePluginSettings<PetPluginSettings>(
      PET_PLUGIN_ID,
      settings.value,
    );
    if (!disposed) settings.value = { ...DEFAULT_SETTINGS, ...stored };
  } catch (e) {
    if (!disposed) error.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

function toggleSource(id: string) {
  const next = new Set(settings.value.sources);
  if (next.has(id)) next.delete(id);
  else next.add(id);
  patch({ sources: [...next] });
}

function preview() {
  void bridge.petPreview().catch((e) => {
    error.value = e instanceof Error ? e.message : String(e);
  });
}

onMounted(() => {
  void bridge
    .pluginSettings<PetPluginSettings>(PET_PLUGIN_ID)
    .then((stored) => {
      if (!disposed) settings.value = { ...DEFAULT_SETTINGS, ...stored };
    })
    .catch((e) => {
      if (!disposed) error.value = e instanceof Error ? e.message : String(e);
    });
});
onBeforeUnmount(() => {
  disposed = true;
  if (timer) void save();
});
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="桌面宠物" />
    <p v-if="error" class="notice" role="alert">{{ error }}</p>
    <SettingGroup title="桌面宠物">
      <SettingRow title="角色" description="四只内置角色，之后可继续扩展">
        <div class="pet-picker desk-pet-picker" role="radiogroup" aria-label="桌面宠物角色">
          <button
            v-for="item in DESKTOP_PETS"
            :key="item.id"
            type="button"
            class="pet-option"
            :class="{ active: settings.character === item.id }"
            role="radio"
            :aria-checked="settings.character === item.id"
            :disabled="saving"
            @click="patch({ character: item.id })"
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
            :class="{ active: settings.barStyle === style.id }"
            :aria-pressed="settings.barStyle === style.id"
            :disabled="saving"
            @click="patch({ barStyle: style.id })"
          >
            {{ style.name }}
          </button>
        </div>
      </SettingRow>
      <SettingRow title="只保留桌宠" description="隐藏通知横条，只显示桌宠">
        <UiSwitch
          :checked="settings.barHidden"
          label="只保留桌宠"
          :disabled="saving"
          @toggle="patch({ barHidden: !settings.barHidden })"
        />
      </SettingRow>
      <SettingRow title="台词气泡" description="和桌宠互动时冒出一句短台词">
        <UiSwitch
          :checked="settings.bubble"
          label="台词气泡"
          :disabled="saving"
          @toggle="patch({ bubble: !settings.bubble })"
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
          :checked="settings.sources.includes(source.id)"
          :label="source.title"
          :disabled="saving"
          @toggle="toggleSource(source.id)"
        />
      </SettingRow>
      <SettingRow title="预览" description="不最小化主窗口，直接查看桌宠效果">
        <UiButton size="sm" :disabled="saving" @click="preview">预览桌宠</UiButton>
      </SettingRow>
    </SettingGroup>
  </section>
</template>
<style scoped>
.pet-picker {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 6px;
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
</style>
