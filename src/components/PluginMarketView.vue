<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  ArrowDown,
  ArrowUp,
  CalendarDays,
  Compass,
  Download,
  RefreshCw,
  Search,
} from 'lucide-vue-next';
import { bridge } from '../bridge';
import type { AppConfig, Plugin } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile from './ui/IconTile.vue';
import UiSwitch from './ui/UiSwitch.vue';
const props = defineProps<{
  plugins: Plugin[];
  config: AppConfig;
  saving: boolean;
  warnings: string[];
}>();
const emit = defineEmits<{
  toggle: [id: string];
  move: [index: number, delta: number];
  reload: [];
  open: [id: string];
}>();
const search = ref('');
const importing = ref(false);
const error = ref('');
const filtered = computed(() =>
  props.plugins.filter((p) =>
    (p.name + p.description).toLowerCase().includes(search.value.trim().toLowerCase()),
  ),
);
async function importPlugin() {
  importing.value = true;
  error.value = '';
  try {
    if (await bridge.importPlugin()) emit('reload');
  } catch (e) {
    error.value = String(e);
  } finally {
    importing.value = false;
  }
}
</script>
<template>
  <section class="desktop-page">
    <PageHeader title="插件" description="管理已安装插件的启用状态与侧栏顺序">
      <template #actions>
        <label class="field-shell search"
          ><Search :size="14" /><input
            v-model="search"
            placeholder="搜索插件"
            aria-label="搜索插件"
        /></label>
        <button
          class="btn ghost icon"
          title="刷新插件"
          aria-label="刷新插件"
          :disabled="importing || saving"
          @click="emit('reload')"
        >
          <RefreshCw :size="15" />
        </button>
        <button class="btn" :disabled="importing" @click="importPlugin">
          <Download :size="14" />{{ importing ? '正在导入…' : '导入本地插件' }}
        </button>
      </template>
    </PageHeader>
    <Transition name="drop">
      <p v-if="error" class="notice" role="alert">{{ error }}</p>
    </Transition>
    <p v-for="warning in warnings" :key="warning" class="notice warning" role="alert">
      {{ warning }}
    </p>
    <SettingGroup :title="`已安装 · ${plugins.length}`">
      <TransitionGroup name="list">
        <SettingRow
          v-for="plugin in filtered"
          :key="plugin.id"
          :class="{ 'plugin-disabled': config.disabledPlugins.includes(plugin.id) }"
        >
          <template #leading>
            <IconTile :tone="plugin.page.type === 'timetable' ? 'blue' : 'green'" :size="34">
              <CalendarDays v-if="plugin.page.type === 'timetable'" :size="17" /><Compass
                v-else
                :size="17"
              />
            </IconTile>
          </template>
          <template #title>
            {{ plugin.name }}<span class="badge">v{{ plugin.version }}</span>
          </template>
          <template #description>{{ plugin.description }}</template>
          <button
            class="btn ghost small"
            :disabled="config.disabledPlugins.includes(plugin.id)"
            @click="emit('open', plugin.id)"
          >
            打开
          </button>
          <span class="row-actions reorder">
            <button
              class="btn ghost icon small"
              title="上移"
              aria-label="上移"
              :disabled="saving || plugins.indexOf(plugin) === 0 || !!search.trim()"
              @click="emit('move', plugins.indexOf(plugin), -1)"
            >
              <ArrowUp :size="14" />
            </button>
            <button
              class="btn ghost icon small"
              title="下移"
              aria-label="下移"
              :disabled="
                saving || plugins.indexOf(plugin) === plugins.length - 1 || !!search.trim()
              "
              @click="emit('move', plugins.indexOf(plugin), 1)"
            >
              <ArrowDown :size="14" />
            </button>
          </span>
          <UiSwitch
            :checked="!config.disabledPlugins.includes(plugin.id)"
            :label="plugin.name + '启用'"
            :disabled="saving"
            @toggle="emit('toggle', plugin.id)"
          />
        </SettingRow>
      </TransitionGroup>
      <EmptyState
        v-if="!filtered.length"
        compact
        :icon="search ? Search : Compass"
        :title="search ? '没有匹配的插件' : '暂无已安装插件'"
      />
    </SettingGroup>
  </section>
</template>
