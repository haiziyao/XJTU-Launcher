<script setup lang="ts">
import { computed, onMounted, onBeforeUnmount, ref, watch } from 'vue';
import { marketBusy as busy, marketProgress as progress, marketError as error } from '../market';
import { ask } from '@tauri-apps/plugin-dialog';
import {
  ArrowDown,
  ArrowUp,
  CalendarDays,
  Compass,
  Download,
  MoreHorizontal,
  Package,
  RefreshCw,
  Route,
  Search,
  Settings2,
} from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { AppConfig, Plugin, MarketCatalog, MarketEntry } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import UiSwitch from './ui/UiSwitch.vue';
import SegmentedTabs from './ui/SegmentedTabs.vue';
const props = defineProps<{
  plugins: Plugin[];
  config: AppConfig;
  saving: boolean;
  warnings: string[];
  sources: Record<string, string>;
}>();
const emit = defineEmits<{
  toggle: [id: string];
  move: [index: number, delta: number];
  reload: [];
  open: [id: string];
  settings: [];
}>();
const tabs = [
  { id: 'campus', title: '校园功能' },
  { id: 'remote', title: '远端市场' },
  { id: 'installed', title: '已安装' },
];
const tab = ref('campus');
const search = ref('');
const loading = ref(false);
const loaded = ref(false);
const catalog = ref<MarketCatalog>({ source: null, defaultSource: null, plugins: [], updates: [] });
const menu = ref('');
let disposed = false;
const matches = (p: { name: string; description?: string }) =>
  (p.name + (p.description || '')).toLowerCase().includes(search.value.trim().toLowerCase());
const filtered = computed(() => props.plugins.filter(matches));
const builtins = computed(() =>
  props.plugins.filter((p) => ['cn.xjtu.timetable', 'cn.xjtu.campus'].includes(p.id) && matches(p)),
);
const remote = computed(() => catalog.value.plugins.filter(matches));
const installed = (id: string) => props.plugins.find((p) => p.id === id);
const upgrade = (id: string) =>
  catalog.value.plugins.find(
    (p) =>
      p.id === id && catalog.value.updates.includes(id) && installed(id)?.version !== p.version,
  );
async function refresh() {
  if (loading.value) return;
  loading.value = true;
  error.value = '';
  try {
    const result = await bridge.pluginCatalog();
    if (!disposed) {
      catalog.value = result;
      loaded.value = true;
    }
  } catch (e) {
    if (!disposed) error.value = String(e);
  } finally {
    loading.value = false;
  }
}
async function importPlugin() {
  busy.value = 'import';
  error.value = '';
  try {
    if (await bridge.importPlugin()) emit('reload');
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
async function install(entry: MarketEntry) {
  busy.value = entry.id;
  progress.value = null;
  error.value = '';
  try {
    await bridge.installPlugin(entry.id, entry.version);
    emit('reload');
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
    progress.value = null;
  }
}
async function remove(plugin: Plugin) {
  menu.value = '';
  if (
    !(await ask(`移除“${plugin.name}”？之后可重新安装。`, {
      title: '移除插件',
      kind: 'warning',
      okLabel: '移除',
      cancelLabel: '取消',
    }))
  )
    return;
  busy.value = plugin.id;
  error.value = '';
  try {
    await bridge.removePlugin(plugin.id);
    emit('reload');
  } catch (e) {
    error.value = String(e);
  } finally {
    busy.value = '';
  }
}
/** Double-clicking a row opens it, unless the click landed on a control. */
function rowDblClick(event: MouseEvent, action: () => void) {
  const target = event.target as HTMLElement | null;
  if (target?.closest('button, a, input, [role="switch"]')) return;
  action();
}
/** Rows only open an enabled plugin; enabling stays a deliberate click. */
function openPlugin(plugin: Plugin) {
  if (!props.config.disabledPlugins.includes(plugin.id)) emit('open', plugin.id);
}
watch(tab, (value) => {
  menu.value = '';
  search.value = '';
  if (value === 'remote' && !loaded.value) void refresh();
});
onBeforeUnmount(() => {
  disposed = true;
});
</script>
<template>
  <section class="desktop-page market-page">
    <PageHeader title="插件市场">
      <template #actions>
        <label class="field-shell search"
          ><Search :size="14" /><input
            v-model="search"
            placeholder="搜索插件"
            aria-label="搜索插件"
        /></label>
        <button v-if="tab === 'installed'" class="btn" :disabled="!!busy" @click="importPlugin">
          <Download :size="14" />导入本地插件
        </button>
        <button
          v-if="tab !== 'campus'"
          class="btn ghost icon"
          title="刷新"
          aria-label="刷新插件"
          :disabled="loading || !!busy"
          @click="
            emit('reload');
            refresh();
          "
        >
          <RefreshCw :size="15" :class="{ spin: loading }" />
        </button>
      </template>
    </PageHeader>
    <SegmentedTabs v-model="tab" :tabs="tabs" label="插件分类" />
    <p v-if="error" class="notice" role="alert">{{ error }}</p>
    <p v-for="warning in warnings" :key="warning" class="notice warning" role="alert">
      {{ warning }}
    </p>
    <div v-if="tab === 'campus'" role="tabpanel">
      <EmptyState v-if="!builtins.length" compact :icon="Search" title="没有匹配的插件" />
      <SettingGroup v-if="builtins.length" title="常用功能">
        <SettingRow
          v-for="plugin in builtins"
          :key="plugin.id"
          class="actionable"
          :title="plugin.name"
          :description="plugin.description"
          @dblclick="rowDblClick($event, () => openPlugin(plugin))"
        >
          <template #leading
            ><IconTile :size="34"
              ><CalendarDays v-if="plugin.page.type === 'timetable'" :size="19" /><Route
                v-else-if="plugin.page.type === 'campus'"
                :size="19" /><Compass v-else :size="19" /></IconTile
          ></template>
          <button
            class="btn small"
            :class="{ tinted: !config.disabledPlugins.includes(plugin.id) }"
            :disabled="saving"
            @click="
              config.disabledPlugins.includes(plugin.id)
                ? emit('toggle', plugin.id)
                : emit('open', plugin.id)
            "
          >
            {{ config.disabledPlugins.includes(plugin.id) ? '启用' : '打开' }}
          </button>
        </SettingRow>
      </SettingGroup>
    </div>
    <div v-else-if="tab === 'remote'" role="tabpanel">
      <EmptyState v-if="loading && !loaded" :icon="RefreshCw" busy title="正在获取插件…" />
      <EmptyState
        v-else-if="!catalog.source && !error"
        :icon="Package"
        title="尚未配置插件服务"
        description="在设置中添加插件服务器后，即可浏览和安装。"
        ><button class="btn" @click="emit('settings')">插件服务设置</button></EmptyState
      >
      <SettingGroup v-else>
        <SettingRow
          v-for="entry in remote"
          :key="entry.id"
          :title="entry.name"
          :description="entry.description"
        >
          <template #leading
            ><IconTile :size="34"><Package :size="19" /></IconTile
          ></template>
          <span class="setting-value">{{ entry.version }}</span>
          <button
            class="btn small"
            :disabled="!!busy || (!!installed(entry.id) && !upgrade(entry.id))"
            @click="install(entry)"
          >
            {{
              busy === entry.id
                ? progress === null
                  ? '安装中…'
                  : `${progress}%`
                : installed(entry.id)
                  ? upgrade(entry.id)
                    ? '更新'
                    : '已安装'
                  : '安装'
            }}
          </button>
        </SettingRow>
        <EmptyState
          v-if="!remote.length && !error"
          compact
          :icon="Search"
          :title="search ? '没有匹配的插件' : '服务端暂无插件'"
        />
      </SettingGroup>
    </div>
    <div v-else role="tabpanel">
      <div class="installed-heading"><span>插件</span><span>启用</span><span></span></div>
      <div v-for="plugin in filtered" :key="plugin.id" class="installed-item">
        <div class="installed-row" @dblclick="rowDblClick($event, () => openPlugin(plugin))">
          <div class="installed-name">
            <IconTile :size="34"
              ><CalendarDays v-if="plugin.page.type === 'timetable'" :size="19" /><Route
                v-else-if="plugin.page.type === 'campus'"
                :size="19" /><Settings2
                v-else-if="plugin.page.type === 'agentSettings'"
                :size="19" /><Compass v-else :size="19"
            /></IconTile>
            <div>
              <strong>{{ plugin.name }}</strong
              ><small
                >{{
                  sources[plugin.id]
                    ? '远端插件'
                    : plugin.page.type === 'agentSettings'
                      ? 'Agent 配置'
                      : plugin.id.startsWith('cn.xjtu.')
                        ? '校园功能'
                        : '本地插件'
                }}
                · {{ plugin.version }}</small
              >
            </div>
          </div>
          <UiSwitch
            :checked="!config.disabledPlugins.includes(plugin.id)"
            :label="plugin.name + '启用'"
            :disabled="saving || !!busy"
            @toggle="emit('toggle', plugin.id)"
          />
          <span class="installed-move">
            <button
              class="btn ghost icon small"
              aria-label="上移"
              :disabled="saving || !!busy || plugins.indexOf(plugin) === 0 || !!search"
              @click="emit('move', plugins.indexOf(plugin), -1)"
            >
              <ArrowUp :size="14" />
            </button>
            <button
              class="btn ghost icon small"
              aria-label="下移"
              :disabled="
                saving || !!busy || plugins.indexOf(plugin) === plugins.length - 1 || !!search
              "
              @click="emit('move', plugins.indexOf(plugin), 1)"
            >
              <ArrowDown :size="14" />
            </button>
          </span>
          <button
            class="btn ghost icon"
            :aria-label="plugin.name + '操作'"
            :aria-expanded="menu === plugin.id"
            @click="menu = menu === plugin.id ? '' : plugin.id"
          >
            <MoreHorizontal :size="18" />
          </button>
        </div>
        <div v-if="menu === plugin.id" class="installed-actions">
          <span class="plugin-origin" :title="sources[plugin.id]">{{
            sources[plugin.id] || plugin.description
          }}</span>
          <button
            class="btn small tinted"
            :disabled="config.disabledPlugins.includes(plugin.id)"
            @click="emit('open', plugin.id)"
          >
            打开
          </button>
          <button
            v-if="upgrade(plugin.id)"
            class="btn small"
            :disabled="!!busy"
            @click="install(upgrade(plugin.id)!)"
          >
            {{ busy === plugin.id ? '更新中…' : '更新' }}
          </button>
          <button
            v-if="!['cn.xjtu.timetable', 'cn.xjtu.campus'].includes(plugin.id)"
            class="btn ghost small danger"
            :disabled="!!busy"
            @click="remove(plugin)"
          >
            移除
          </button>
        </div>
      </div>
      <EmptyState v-if="!filtered.length" compact :icon="Search" title="没有匹配的插件" />
    </div>
  </section>
</template>
<style scoped>
.market-page :deep(.setting-card) {
  border: 0;
  box-shadow: none;
  border-radius: 0;
  background: transparent;
}
.market-page :deep(.setting-row) {
  min-height: 70px;
  padding: 14px 12px;
}
.market-page :deep(.setting-group) {
  margin-bottom: 30px;
}
.installed-heading,
.installed-row {
  display: grid;
  grid-template-columns: minmax(0, 1fr) 72px auto 36px;
  gap: 12px;
  align-items: center;
  padding: 16px 12px;
}
.installed-heading {
  color: var(--text-tertiary);
  font-size: 12px;
}
.installed-row {
  min-height: 86px;
  transition: background var(--duration-fast) ease;
}
.installed-row:hover {
  background: var(--accent-wash);
}
.installed-item {
  border-bottom: 1px solid var(--border);
}
.installed-name {
  display: flex;
  align-items: center;
  gap: 16px;
  min-width: 0;
}
.installed-name strong {
  font-weight: 600;
}
.installed-name small {
  display: block;
  color: var(--text-tertiary);
  margin-top: 5px;
}
.installed-move {
  display: inline-flex;
  justify-content: flex-end;
  gap: 2px;
}
.installed-actions {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 0 12px 14px 62px;
}
.plugin-origin {
  flex: 1;
  color: var(--text-tertiary);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  font-size: 12px;
}
</style>
