<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { RotateCcw } from 'lucide-vue-next';
import { bridge } from '../bridge';
import {
  DEFAULT_FONT_SETTINGS,
  FONT_FAMILIES,
  FONT_PLUGIN_ID,
  FONT_REGIONS,
  normalizeFontSettings,
} from '../fonts';
import type { FontPluginSettings, FontRegion, FontRegionId } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import SettingsDialog from './ui/SettingsDialog.vue';
import UiButton from './ui/UiButton.vue';

const props = defineProps<{ settings: FontPluginSettings }>();
const emit = defineEmits<{ update: [settings: FontPluginSettings] }>();

const settings = ref(normalizeFontSettings(props.settings));
const open = ref<FontRegionId | null>(null);
const error = ref('');
const saving = ref(false);
let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;

const WEIGHTS = [300, 400, 500, 600, 700, 800];
const current = computed(() => (open.value ? settings.value[open.value] : null));
const currentName = computed(
  () => FONT_REGIONS.find((region) => region.id === open.value)?.name ?? '',
);

function familyName(id: string) {
  return FONT_FAMILIES.find((family) => family.id === id)?.name ?? id;
}

function patch(region: FontRegionId, change: Partial<FontRegion>) {
  settings.value = {
    ...settings.value,
    [region]: { ...settings.value[region], ...change },
  };
  emit('update', settings.value);
  scheduleSave();
}

function scheduleSave() {
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => void save(), 320);
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
    const stored = await bridge.savePluginSettings<FontPluginSettings>(
      FONT_PLUGIN_ID,
      settings.value,
    );
    if (disposed) return;
    const next = normalizeFontSettings(stored);
    settings.value = next;
    emit('update', next);
  } catch (e) {
    if (!disposed) error.value = e instanceof Error ? e.message : String(e);
  } finally {
    saving.value = false;
  }
}

function resetRegion(region: FontRegionId) {
  patch(region, DEFAULT_FONT_SETTINGS[region]);
}

onMounted(() => {
  void bridge
    .pluginSettings<FontPluginSettings>(FONT_PLUGIN_ID)
    .then((stored) => {
      if (disposed) return;
      const next = normalizeFontSettings(stored);
      settings.value = next;
      emit('update', next);
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
  <section class="desktop-page font-page">
    <PageHeader title="字体美化" />
    <p v-if="error" class="notice" role="alert">{{ error }}</p>
    <SettingGroup title="字体区域">
      <SettingRow
        v-for="region in FONT_REGIONS"
        :key="region.id"
        clickable
        :title="region.name"
        :description="region.description"
        @select="open = region.id"
      >
        <span class="setting-value">{{ familyName(settings[region.id].family) }}</span>
      </SettingRow>
    </SettingGroup>
    <SettingsDialog :open="open !== null" :title="`${currentName}字体`" @close="open = null">
      <template v-if="open && current">
        <label class="font-field">
          <span>字体</span>
          <input
            :value="current.family"
            list="plugin-font-families"
            spellcheck="false"
            @change="patch(open, { family: ($event.target as HTMLInputElement).value.trim() })"
          />
          <datalist id="plugin-font-families">
            <option v-for="family in FONT_FAMILIES" :key="family.id" :value="family.id">
              {{ family.name }}
            </option>
          </datalist>
        </label>
        <label class="font-field">
          <span>字号</span>
          <span class="font-field-row">
            <input
              type="range"
              min="8"
              max="48"
              step="0.5"
              :value="current.size"
              @input="patch(open, { size: Number(($event.target as HTMLInputElement).value) })"
            />
            <span class="font-value">{{ current.size }} px</span>
          </span>
        </label>
        <label class="font-field">
          <span>字重</span>
          <select
            :value="current.weight"
            @change="patch(open, { weight: Number(($event.target as HTMLSelectElement).value) })"
          >
            <option v-for="weight in WEIGHTS" :key="weight" :value="weight">{{ weight }}</option>
          </select>
        </label>
        <label class="font-field">
          <span>行高</span>
          <span class="font-field-row">
            <input
              type="range"
              min="1"
              max="2.5"
              step="0.05"
              :value="current.lineHeight"
              @input="
                patch(open, { lineHeight: Number(($event.target as HTMLInputElement).value) })
              "
            />
            <span class="font-value">{{ current.lineHeight }}</span>
          </span>
        </label>
        <label class="font-field">
          <span>字距</span>
          <span class="font-field-row">
            <input
              type="range"
              min="-1"
              max="5"
              step="0.1"
              :value="current.letterSpacing"
              @input="
                patch(open, { letterSpacing: Number(($event.target as HTMLInputElement).value) })
              "
            />
            <span class="font-value">{{ current.letterSpacing }} px</span>
          </span>
        </label>
      </template>
      <template #footer>
        <UiButton variant="ghost" size="sm" @click="open && resetRegion(open)">
          <RotateCcw :size="14" />恢复默认
        </UiButton>
        <UiButton size="sm" @click="open = null">完成</UiButton>
      </template>
    </SettingsDialog>
  </section>
</template>
<style scoped>
.font-field {
  display: grid;
  gap: 6px;
  color: var(--text-secondary);
  font-size: 12px;
}
.font-field > input,
.font-field select {
  height: 30px;
  padding: 0 9px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text);
  font: inherit;
}
.font-field-row {
  display: flex;
  align-items: center;
  gap: 10px;
}
.font-field-row input[type='range'] {
  flex: 1;
  accent-color: var(--accent);
}
.font-value {
  min-width: 62px;
  text-align: right;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}
</style>
