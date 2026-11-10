<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import { Download, RotateCcw, Trash2 } from 'lucide-vue-next';
import { bridge } from '../../shared/bridge';
import { fileSize } from '../../shared/domain';
import {
  DEFAULT_FONT_SETTINGS,
  FONT_CATEGORIES,
  FONT_FAMILIES,
  FONT_PLUGIN_ID,
  FONT_REGIONS,
  applyFontToRegions,
  ensureFontsLoaded,
  fontFamilyStack,
  normalizeFontSettings,
  setFontLibrary,
} from './fonts';
import type { FontCategory, FontCategoryId } from './fonts';
import type {
  FontCatalogEntry,
  FontDownloadProgress,
  FontLibraryState,
  FontPluginSettings,
  FontRegion,
  InstalledFont,
} from '../../types';
import wenkaiPreview from '../../assets/font-previews/wenkai.png';
import neoXiheiPreview from '../../assets/font-previews/neo-xihei.png';
import notoSerifPreview from '../../assets/font-previews/noto-serif.png';
import sarasaPreview from '../../assets/font-previews/sarasa-gothic.png';
import MenuSelect from '../../components/ui/MenuSelect.vue';
import MoreMenu, { type MoreItem } from '../../components/ui/MoreMenu.vue';
import PageHeader from '../../components/ui/PageHeader.vue';
import SettingGroup from '../../components/ui/SettingGroup.vue';
import SettingRow from '../../components/ui/SettingRow.vue';
import SettingsDialog from '../../components/ui/SettingsDialog.vue';
import UiButton from '../../components/ui/UiButton.vue';

const props = defineProps<{ settings: FontPluginSettings }>();
const emit = defineEmits<{ update: [settings: FontPluginSettings] }>();

const PREVIEWS: Record<string, string> = {
  wenkai: wenkaiPreview,
  'neo-xihei': neoXiheiPreview,
  'noto-serif': notoSerifPreview,
  'sarasa-gothic': sarasaPreview,
};

interface LibraryRow {
  id: string;
  name: string;
  family: string;
  style: string;
  license: string;
  size: number;
  catalog: FontCatalogEntry | null;
  installed: InstalledFont | null;
  local: boolean;
  preview?: string;
}

const settings = ref(normalizeFontSettings(props.settings));
const open = ref<FontCategoryId | null>(null);
const error = ref('');
const saving = ref(false);

const library = ref<FontLibraryState | null>(null);
const downloads = ref<Record<string, FontDownloadProgress | undefined>>({});
const cancelling = ref<Record<string, boolean>>({});
const adding = ref(false);
const deleting = ref<LibraryRow | null>(null);

const rowMenu: MoreItem[] = [{ id: 'delete', label: '删除字体文件', icon: Trash2, danger: true }];

let timer: ReturnType<typeof setTimeout> | undefined;
let disposed = false;
let unlisten: (() => void) | undefined;

const rows = computed<LibraryRow[]>(() => {
  const installed = library.value?.installed ?? [];
  const catalogRows = (library.value?.catalog ?? []).map((entry) => ({
    id: entry.id,
    name: entry.name,
    family: entry.family,
    style: entry.style,
    license: entry.license,
    size: entry.size,
    catalog: entry,
    installed: installed.find((font) => font.id === entry.id) ?? null,
    local: false,
    preview: PREVIEWS[entry.id],
  }));
  const catalogIds = new Set((library.value?.catalog ?? []).map((entry) => entry.id));
  const localRows = installed
    .filter((font) => !catalogIds.has(font.id))
    .map((font) => ({
      id: font.id,
      name: font.name,
      family: font.family,
      style: '本地字体',
      license: '',
      size: font.size,
      catalog: null,
      installed: font,
      local: true,
      preview: undefined,
    }));
  return [...catalogRows, ...localRows];
});

const familyOptions = computed(() => {
  const options: { id: string; name: string }[] = [...FONT_FAMILIES];
  const seen = new Set(options.map((option) => option.id));
  for (const font of library.value?.installed ?? []) {
    if (!seen.has(font.family)) {
      options.push({ id: font.family, name: font.name });
      seen.add(font.family);
    }
  }
  return options;
});

function familyName(family: string) {
  const installed = library.value?.installed.find((font) => font.family === family);
  if (installed) return installed.name;
  return FONT_FAMILIES.find((option) => option.id === family)?.name ?? family;
}

const familySelectOptions = computed(() =>
  familyOptions.value.map((option) => ({ value: option.id, label: option.name })),
);

function toggle(category: FontCategoryId) {
  open.value = open.value === category ? null : category;
}

const SAMPLES: Record<FontCategoryId, string> = {
  global: '设置 · 按钮 · 导航 · 第 3 节 · 高等数学',
  title: '永和九年，岁在癸丑',
  body: '永和九年，岁在癸丑。暮春之初，会于会稽山阴之兰亭。',
  code: 'const answer = 42; // 永和九年',
};

function sampleText(category: FontCategory) {
  return SAMPLES[category.id];
}

/** Mirrors the category's real CSS values so the sample updates while dragging. */
function sampleStyle(category: FontCategory) {
  const value = settings.value[category.regions[0]];
  return {
    fontFamily: fontFamilyStack(value.family, category.id === 'code' ? 'mono' : 'sans'),
    fontSize: `${value.size}px`,
    fontWeight: String(value.weight),
    lineHeight: String(value.lineHeight),
    letterSpacing: `${value.letterSpacing}px`,
    color: value.color || undefined,
  };
}

function rangeFill(value: number, min: number, max: number): Record<string, string> {
  const percent = Math.max(0, Math.min(100, ((value - min) / (max - min)) * 100));
  return { '--font-range-fill': `${percent}%` };
}

function messageOf(value: unknown) {
  return value instanceof Error ? value.message : String(value);
}

function patchCategory(category: FontCategory, change: Partial<FontRegion>) {
  const next = { ...settings.value };
  for (const id of category.regions) {
    next[id] = { ...next[id], ...change };
  }
  settings.value = next;
  emit('update', next);
  scheduleSave();
}

function commit(next: FontPluginSettings) {
  settings.value = next;
  emit('update', next);
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
    if (!disposed) error.value = messageOf(e);
  } finally {
    saving.value = false;
  }
}

function resetCategory(category: FontCategory) {
  const next = { ...settings.value };
  for (const id of category.regions) {
    next[id] = { ...DEFAULT_FONT_SETTINGS[id] };
  }
  commit(next);
}

async function loadLibrary() {
  if (disposed) return;
  try {
    const state = await bridge.fontsState();
    if (disposed) return;
    library.value = state;
    setFontLibrary(state);
    // Local fonts have no bundled preview image; load them so the compact
    // thumbnail can render in the actual face.
    void ensureFontsLoaded(
      state.installed.filter((font) => font.source === 'local').map((font) => font.family),
    );
  } catch (e) {
    if (!disposed) error.value = messageOf(e);
  } finally {
  }
}

function progressOf(row: LibraryRow) {
  return downloads.value[row.id];
}

function badgeText(row: LibraryRow) {
  const progress = progressOf(row);
  if (progress) return progress.phase === 'extract' ? '正在解压' : '下载中';
  if (row.installed) return isUsed(row) ? '使用中' : row.local ? '本地' : '已安装';
  return '未安装';
}

function progressLabel(row: LibraryRow) {
  const progress = progressOf(row);
  if (!progress) return '';
  if (progress.phase === 'extract') return '正在解压…';
  const percent =
    progress.total > 0 ? Math.min(100, Math.round((progress.received / progress.total) * 100)) : 0;
  return `正在下载 ${percent}%`;
}

function rowMeta(row: LibraryRow) {
  const parts = [row.style, fileSize(row.installed?.size ?? row.size)];
  if (row.local) {
    if (row.installed?.format) parts.push(row.installed.format.toUpperCase());
  } else if (row.license) {
    parts.push(row.license);
  }
  return parts.filter(Boolean).join(' · ');
}

function isUsed(row: LibraryRow) {
  return (
    !!row.installed &&
    FONT_REGIONS.some((region) => settings.value[region.id].family === row.family)
  );
}

function previewFamily(family: string) {
  const clean = family.replace(/["\\\n\r]/g, '').trim();
  return clean ? `"${clean}", var(--font-sans)` : 'var(--font-sans)';
}

function setDownload(id: string, progress: FontDownloadProgress | null) {
  const next = { ...downloads.value };
  if (progress) next[id] = progress;
  else delete next[id];
  downloads.value = next;
}

function onProgress(progress: FontDownloadProgress) {
  if (!downloads.value[progress.id]) return;
  setDownload(progress.id, progress);
}

async function download(row: LibraryRow) {
  if (!row.catalog || downloads.value[row.id]) return;
  error.value = '';
  setDownload(row.id, { id: row.id, phase: 'download', received: 0, total: row.size });
  try {
    await bridge.fontDownload(row.id);
    await loadLibrary();
  } catch (e) {
    const message = messageOf(e);
    if (!message.includes('已取消')) error.value = message;
  } finally {
    setDownload(row.id, null);
    const next = { ...cancelling.value };
    delete next[row.id];
    cancelling.value = next;
  }
}

async function cancel(row: LibraryRow) {
  if (cancelling.value[row.id]) return;
  cancelling.value = { ...cancelling.value, [row.id]: true };
  try {
    await bridge.fontDownloadCancel(row.id);
  } catch (e) {
    error.value = messageOf(e);
  }
}

function apply(row: LibraryRow) {
  if (!row.installed) return;
  commit(applyFontToRegions(settings.value, row.family));
}

/** Stops using the font but keeps the downloaded file, so it can be applied again. */
function unapply(row: LibraryRow) {
  const next = { ...settings.value };
  let changed = false;
  for (const region of FONT_REGIONS) {
    if (next[region.id].family === row.family) {
      next[region.id] = { ...next[region.id], family: 'system' };
      changed = true;
    }
  }
  if (changed) commit(next);
}

function askDelete(row: LibraryRow) {
  deleting.value = row;
}

async function confirmDelete() {
  const row = deleting.value;
  if (!row) return;
  deleting.value = null;
  await remove(row);
}

async function remove(row: LibraryRow) {
  if (!row.installed) return;
  error.value = '';
  try {
    await bridge.fontRemove(row.id);
  } catch (e) {
    error.value = messageOf(e);
  } finally {
    await loadLibrary();
    const stillInstalled = (library.value?.installed ?? []).some(
      (font) => font.family === row.family,
    );
    if (!stillInstalled) {
      const next = { ...settings.value };
      let changed = false;
      for (const region of FONT_REGIONS) {
        if (next[region.id].family === row.family) {
          next[region.id] = { ...next[region.id], family: 'system' };
          changed = true;
        }
      }
      if (changed) commit(next);
    }
  }
}

async function addLocal() {
  if (adding.value) return;
  adding.value = true;
  error.value = '';
  try {
    const font = await bridge.fontAddLocal();
    if (font) await loadLibrary();
  } catch (e) {
    error.value = messageOf(e);
  } finally {
    adding.value = false;
  }
}

function openSource(row: LibraryRow) {
  if (row.catalog) void bridge.openUrl(row.catalog.sourceUrl);
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
      if (!disposed) error.value = messageOf(e);
    });
  void loadLibrary();
  void bridge
    .onFontDownload(onProgress)
    .then((stop) => {
      if (disposed) stop();
      else unlisten = stop;
    })
    .catch(() => {
      // Browser-only frontend preview has no download events.
    });
});

onBeforeUnmount(() => {
  disposed = true;
  unlisten?.();
  if (timer) void save();
});
</script>
<template>
  <section class="desktop-page font-page">
    <PageHeader title="字体美化" />
    <p v-if="error" class="notice" role="alert">{{ error }}</p>
    <SettingGroup title="字体区域">
      <div
        v-for="category in FONT_CATEGORIES"
        :key="category.id"
        class="font-region"
        :class="{ open: open === category.id }"
      >
        <SettingRow
          clickable
          :title="category.name"
          :description="category.description"
          @select="toggle(category.id)"
        >
          <span class="setting-value">
            {{ familyName(settings[category.regions[0]].family) }}
          </span>
        </SettingRow>
        <Transition name="font-expand">
          <div v-if="open === category.id" class="font-editor">
            <div class="font-preview">
              <div class="font-preview-head">
                <span class="font-preview-label">预览 · {{ category.name }}</span>
                <span class="font-preview-scope">{{ category.description }}</span>
              </div>
              <p class="font-preview-text" :style="sampleStyle(category)">
                {{ sampleText(category) }}
              </p>
            </div>
            <SettingRow title="字体">
              <MenuSelect
                class="font-family-select"
                :model-value="settings[category.regions[0]].family"
                :options="familySelectOptions"
                label="字体"
                @update:model-value="patchCategory(category, { family: $event })"
              />
            </SettingRow>
            <SettingRow title="颜色">
              <div class="font-color">
                <input
                  class="font-color-input"
                  type="color"
                  :value="settings[category.regions[0]].color || '#1f2329'"
                  :aria-label="`${category.name}文字颜色`"
                  @input="
                    patchCategory(category, {
                      color: ($event.target as HTMLInputElement).value,
                    })
                  "
                />
                <span class="font-color-value">
                  {{ settings[category.regions[0]].color || '跟随主题' }}
                </span>
                <UiButton
                  v-if="settings[category.regions[0]].color"
                  size="sm"
                  variant="ghost"
                  @click="patchCategory(category, { color: '' })"
                >
                  清除
                </UiButton>
              </div>
            </SettingRow>
            <SettingRow title="字号">
              <div class="font-range">
                <input
                  type="range"
                  min="8"
                  max="48"
                  step="0.5"
                  :value="settings[category.regions[0]].size"
                  :style="rangeFill(settings[category.regions[0]].size, 8, 48)"
                  :aria-label="`${category.name}字号`"
                  @input="
                    patchCategory(category, {
                      size: Number(($event.target as HTMLInputElement).value),
                    })
                  "
                />
                <span class="font-range-value">{{ settings[category.regions[0]].size }} px</span>
              </div>
            </SettingRow>
            <SettingRow title="字重">
              <div class="font-range">
                <input
                  type="range"
                  min="100"
                  max="900"
                  step="100"
                  :value="settings[category.regions[0]].weight"
                  :style="rangeFill(settings[category.regions[0]].weight, 100, 900)"
                  :aria-label="`${category.name}字重`"
                  @input="
                    patchCategory(category, {
                      weight: Number(($event.target as HTMLInputElement).value),
                    })
                  "
                />
                <span class="font-range-value">{{ settings[category.regions[0]].weight }}</span>
              </div>
            </SettingRow>
            <SettingRow title="行高">
              <div class="font-range">
                <input
                  type="range"
                  min="1"
                  max="2.5"
                  step="0.05"
                  :value="settings[category.regions[0]].lineHeight"
                  :style="rangeFill(settings[category.regions[0]].lineHeight, 1, 2.5)"
                  :aria-label="`${category.name}行高`"
                  @input="
                    patchCategory(category, {
                      lineHeight: Number(($event.target as HTMLInputElement).value),
                    })
                  "
                />
                <span class="font-range-value">
                  {{ settings[category.regions[0]].lineHeight }}
                </span>
              </div>
            </SettingRow>
            <SettingRow title="字距">
              <div class="font-range">
                <input
                  type="range"
                  min="-1"
                  max="5"
                  step="0.1"
                  :value="settings[category.regions[0]].letterSpacing"
                  :style="rangeFill(settings[category.regions[0]].letterSpacing, -1, 5)"
                  :aria-label="`${category.name}字距`"
                  @input="
                    patchCategory(category, {
                      letterSpacing: Number(($event.target as HTMLInputElement).value),
                    })
                  "
                />
                <span class="font-range-value">
                  {{ settings[category.regions[0]].letterSpacing }} px
                </span>
              </div>
            </SettingRow>
            <div class="font-editor-footer">
              <UiButton size="sm" variant="ghost" @click="resetCategory(category)">
                <RotateCcw :size="14" />恢复默认
              </UiButton>
            </div>
          </div>
        </Transition>
      </div>
    </SettingGroup>
    <SettingGroup title="字体库">
      <template #actions>
        <UiButton size="sm" variant="ghost" :disabled="adding" @click="addLocal">
          {{ adding ? '添加中…' : '+ 添加字体…' }}
        </UiButton>
      </template>
      <div v-for="row in rows" :key="row.id" class="setting-row font-row">
        <span class="font-thumb" aria-hidden="true">
          <img v-if="row.preview" :src="row.preview" alt="" />
          <span v-else class="font-thumb-text" :style="{ fontFamily: previewFamily(row.family) }">
            永和九年
          </span>
        </span>
        <span class="setting-row-copy font-copy">
          <strong>
            {{ row.name }}
            <span
              class="font-badge"
              :class="{
                installed: row.installed && !progressOf(row),
                running: !!progressOf(row),
              }"
            >
              {{ badgeText(row) }}
            </span>
          </strong>
          <small v-if="progressOf(row)" class="font-progress">
            <progress
              v-if="progressOf(row)?.phase === 'download'"
              :value="progressOf(row)?.received ?? 0"
              :max="progressOf(row)?.total || 1"
              aria-label="字体下载进度"
            ></progress>
            <progress v-else aria-label="字体解压进度"></progress>
            <span role="status">{{ progressLabel(row) }}</span>
          </small>
          <small v-else class="font-meta">{{ rowMeta(row) }}</small>
        </span>
        <span class="font-actions">
          <UiButton
            v-if="row.catalog"
            class="font-source"
            size="sm"
            variant="ghost"
            @click="openSource(row)"
          >
            来源
          </UiButton>
          <UiButton
            v-if="progressOf(row)"
            class="font-toggle"
            size="sm"
            variant="ghost"
            :disabled="progressOf(row)?.phase === 'extract' || cancelling[row.id]"
            @click="cancel(row)"
          >
            取消
          </UiButton>
          <template v-else-if="row.installed">
            <UiButton
              v-if="isUsed(row)"
              class="font-toggle"
              size="sm"
              variant="secondary"
              @click="unapply(row)"
            >
              取消应用
            </UiButton>
            <UiButton
              v-else
              class="font-apply font-toggle"
              size="sm"
              variant="secondary"
              @click="apply(row)"
            >
              应用
            </UiButton>
            <MoreMenu class="font-more" :items="rowMenu" @select="askDelete(row)" />
          </template>
          <UiButton
            v-else
            class="font-download"
            size="sm"
            variant="secondary"
            @click="download(row)"
          >
            <Download :size="13" :stroke-width="2" />
            下载
          </UiButton>
        </span>
      </div>
    </SettingGroup>
    <SettingsDialog :open="deleting !== null" title="删除字体" @close="deleting = null">
      <p v-if="deleting" class="font-delete-copy">
        将删除「{{ deleting.name }}」的字体文件；正在使用它的区域会恢复系统字体，之后需要重新下载。
      </p>
      <template #footer>
        <UiButton variant="ghost" size="sm" @click="deleting = null">取消</UiButton>
        <UiButton variant="primary" size="sm" danger @click="confirmDelete">删除</UiButton>
      </template>
    </SettingsDialog>
  </section>
</template>
<style scoped>
.font-region :deep(.setting-row-chevron) {
  transition: transform var(--duration-fast) var(--ease-out);
}
.font-region.open :deep(.setting-row-chevron) {
  transform: rotate(90deg);
}
.font-region.open :deep(.setting-row.clickable:hover .setting-row-chevron) {
  transform: rotate(90deg);
}
.font-editor {
  padding: 12px 16px 10px;
  background: var(--surface-sunken);
  box-shadow: inset 0 1px 0 var(--divider);
}
.font-preview {
  margin-bottom: 6px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
  box-shadow: var(--shadow-xs);
}
.font-preview-head {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.font-preview-label {
  padding: 0 6px;
  border-radius: var(--radius-full);
  background: var(--accent-tint);
  color: var(--accent-text);
  font-size: 10.5px;
  font-weight: 500;
  line-height: 1.7;
}
.font-preview-scope {
  color: var(--text-tertiary);
  font-size: 11.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.font-preview-text {
  margin: 0;
  color: var(--text);
  overflow-wrap: anywhere;
}
.font-editor :deep(.setting-row) {
  min-height: 44px;
  padding: 0;
}
.font-editor :deep(.setting-row + .setting-row) {
  border-top: 1px solid var(--divider);
}
.font-range {
  display: flex;
  align-items: center;
  gap: 10px;
  width: 220px;
  max-width: 100%;
}
.font-range input[type='range'] {
  flex: 1;
  min-width: 120px;
  height: 18px;
  margin: 0;
  border: 0;
  background: transparent;
  -webkit-appearance: none;
  appearance: none;
}
.font-range input[type='range']::-webkit-slider-runnable-track {
  height: 4px;
  border-radius: var(--radius-full);
  background: linear-gradient(
    to right,
    var(--accent) 0 var(--font-range-fill, 50%),
    var(--control-sunken) var(--font-range-fill, 50%) 100%
  );
}
.font-range input[type='range']::-webkit-slider-thumb {
  width: 14px;
  height: 14px;
  margin-top: -5px;
  border: 1px solid var(--control-border);
  border-radius: 50%;
  background: #fff;
  box-shadow: 0 1px 3px rgba(15, 23, 42, 0.16);
  -webkit-appearance: none;
  appearance: none;
  transition: box-shadow var(--duration-fast) ease;
}
.font-range input[type='range']:hover::-webkit-slider-thumb {
  box-shadow: 0 0 0 3px var(--accent-wash);
}
.font-range input[type='range']:focus-visible {
  outline: none;
}
.font-range input[type='range']:focus-visible::-webkit-slider-thumb {
  box-shadow: 0 0 0 3px var(--accent-tint);
}
.font-range-value {
  min-width: 54px;
  color: var(--text-tertiary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
  text-align: right;
  white-space: nowrap;
}
.font-family-select {
  width: 190px;
}
.font-color {
  display: flex;
  align-items: center;
  gap: 8px;
}
.font-color-input {
  width: 30px;
  height: 24px;
  padding: 2px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  cursor: pointer;
}
.font-color-input::-webkit-color-swatch-wrapper {
  padding: 2px;
}
.font-color-input::-webkit-color-swatch {
  border: 0;
  border-radius: 3px;
}
.font-color-value {
  min-width: 62px;
  color: var(--text-tertiary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.font-editor-footer {
  display: flex;
  justify-content: flex-end;
  padding-top: 8px;
}
.font-expand-enter-active,
.font-expand-leave-active {
  transition:
    opacity 120ms ease,
    transform 120ms ease;
}
.font-expand-enter-from,
.font-expand-leave-to {
  opacity: 0;
  transform: translateY(-3px);
}
.font-row {
  gap: 12px;
  min-height: 58px;
  padding-top: 8px;
  padding-bottom: 8px;
}
.font-thumb {
  display: grid;
  flex-shrink: 0;
  place-items: center;
  width: 96px;
  height: 37px;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: #fff;
}
.font-thumb img {
  display: block;
  width: 100%;
  height: 100%;
  object-fit: cover;
}
.font-thumb-text {
  color: #1f2329;
  font-size: 15px;
  line-height: 1;
  white-space: nowrap;
}
.font-copy {
  display: flex;
  flex-direction: column;
  justify-content: center;
}
.font-badge {
  padding: 1px 6px;
  border-radius: 999px;
  background: var(--surface-sunken);
  color: var(--text-tertiary);
  font-size: 10.5px;
  font-weight: 400;
  line-height: 1.5;
}
.font-badge.installed {
  background: var(--success-soft);
  color: var(--success-text);
}
.font-badge.running {
  background: var(--accent-tint);
  color: var(--accent-text);
}
.font-meta {
  margin-top: 2px;
}
.font-progress {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-top: 4px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  line-height: 1.2;
  white-space: nowrap;
}
.font-progress progress {
  flex: 1;
  min-width: 60px;
  max-width: 150px;
  height: 4px;
  accent-color: var(--accent);
}
.font-actions {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 4px;
  flex-shrink: 0;
}
/* Keep the action columns aligned across rows: the same fixed widths are
   reserved whether the row shows 应用, 取消应用 or 取消. */
.font-source {
  min-width: 46px;
}
.font-toggle,
.font-download {
  min-width: 68px;
}
.font-more :deep(.ui-icon-button) {
  width: 26px;
  height: 26px;
}
/* Crisp outline instead of a translucent colour block: the row keeps the
   white, bordered look of the settings rows around it. */
.font-download,
.font-apply {
  border-color: var(--accent-outline);
  color: var(--accent-text);
}
.font-download svg {
  color: var(--accent);
}
.font-download:hover:not(:disabled),
.font-apply:hover:not(:disabled) {
  border-color: var(--accent);
  background: var(--accent-wash);
}
.font-delete-copy {
  margin: 0;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.6;
}
</style>
