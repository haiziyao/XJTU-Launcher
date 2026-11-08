<script setup lang="ts">
import { computed, ref } from 'vue';
import { Search } from 'lucide-vue-next';
import type { AppConfig, Plugin, PluginCategory } from '../types';
import { pluginGlyph } from '../pluginGlyph';
import PageHeader from './ui/PageHeader.vue';
import PageTabs from './ui/PageTabs.vue';
import EmptyState from './ui/EmptyState.vue';
import UiSwitch from './ui/UiSwitch.vue';

const props = defineProps<{
  plugins: Plugin[];
  config: AppConfig;
  saving: boolean;
}>();
const emit = defineEmits<{
  toggle: [id: string];
}>();

const tabs = [
  { id: 'campus', title: '校园广场' },
  { id: 'invention', title: '拓展发明' },
  { id: 'lab', title: '创新小港' },
  { id: 'installed', title: '已安装插件' },
];
const tab = ref('campus');
const search = ref('');
const expanded = ref('');

const CATEGORY_NAMES: Record<PluginCategory, string> = {
  campus: '校园广场',
  invention: '拓展发明',
  lab: '创新小港',
};

const matches = (plugin: Plugin) =>
  (plugin.name + plugin.description).toLowerCase().includes(search.value.trim().toLowerCase());
const visible = computed(() => {
  if (tab.value === 'installed') return props.plugins.filter(matches);
  return props.plugins.filter((plugin) => plugin.category === tab.value && matches(plugin));
});
const enabled = (id: string) => !props.config.disabledPlugins.includes(id);

function expand(id: string) {
  expanded.value = expanded.value === id ? '' : id;
}
function onRowKey(event: KeyboardEvent, id: string) {
  if (event.key === 'Enter' || event.key === ' ') {
    event.preventDefault();
    expand(id);
  }
}
</script>
<template>
  <section class="desktop-page plugin-centre">
    <PageHeader title="插件中心">
      <template #actions>
        <label class="field-shell search"
          ><Search :size="14" /><input
            v-model="search"
            placeholder="搜索插件"
            aria-label="搜索插件"
        /></label>
      </template>
    </PageHeader>
    <PageTabs v-model="tab" :tabs="tabs" label="插件分类" />
    <div role="tabpanel">
      <div v-for="plugin in visible" :key="plugin.id" class="plugin-item">
        <div
          class="plugin-row"
          :class="{ expanded: expanded === plugin.id }"
          role="button"
          tabindex="0"
          :aria-expanded="expanded === plugin.id"
          @click="expand(plugin.id)"
          @keydown="onRowKey($event, plugin.id)"
        >
          <span class="plugin-icon" aria-hidden="true">
            <component :is="pluginGlyph(plugin)" :size="19" />
          </span>
          <strong class="plugin-name">{{ plugin.name }}</strong>
          <small class="plugin-description">{{ plugin.description }}</small>
          <span class="plugin-switch" @click.stop>
            <UiSwitch
              :checked="enabled(plugin.id)"
              :label="plugin.name + '启用'"
              :disabled="saving"
              @toggle="emit('toggle', plugin.id)"
            />
          </span>
        </div>
        <Transition name="plugin-expand">
          <div v-if="expanded === plugin.id" class="plugin-details">
            <div class="plugin-chips">
              <span class="plugin-chip" :class="{ accent: enabled(plugin.id) }">
                {{ enabled(plugin.id) ? '已启用' : '已关闭' }}
              </span>
              <span class="plugin-chip">作者 {{ plugin.author }}</span>
              <span class="plugin-chip">版本 {{ plugin.version }}</span>
              <span class="plugin-chip">{{
                CATEGORY_NAMES[plugin.category] ?? plugin.category
              }}</span>
            </div>
            <p>{{ plugin.details || plugin.description }}</p>
          </div>
        </Transition>
      </div>
      <EmptyState
        v-if="!visible.length"
        compact
        :icon="Search"
        :title="search ? '没有匹配的插件' : tab === 'installed' ? '还没有插件' : '这里还没有插件'"
      />
    </div>
  </section>
</template>
<style scoped>
.plugin-item + .plugin-item {
  border-top: 1px solid var(--divider);
}
.plugin-row {
  position: relative;
  display: grid;
  grid-template-columns: auto minmax(0, auto) minmax(0, 1fr) auto;
  align-items: center;
  gap: 14px;
  min-height: 68px;
  padding: 12px 14px;
  border-radius: var(--radius);
  cursor: pointer;
  transition: background var(--duration-fast) ease;
}
.plugin-row:hover {
  background: var(--surface-hover);
}
.plugin-row.expanded {
  background: var(--accent-wash);
}
/* A tinted rail marks the open row without turning every row into a card. */
.plugin-row.expanded::before {
  content: '';
  position: absolute;
  left: 2px;
  top: 12px;
  bottom: 12px;
  width: 3px;
  border-radius: var(--radius-full);
  background: var(--accent);
}
.plugin-row:focus-visible {
  outline: none;
  box-shadow: var(--focus-ring);
}
.plugin-icon {
  display: grid;
  place-items: center;
  width: 38px;
  height: 38px;
  border-radius: 11px;
  background: var(--accent-wash);
  color: var(--accent-text);
  box-shadow: inset 0 0 0 1px var(--accent-tint);
}
.plugin-name {
  font-size: 15px;
  font-weight: 650;
  letter-spacing: 0.01em;
  white-space: nowrap;
}
.plugin-description {
  min-width: 0;
  color: var(--text-tertiary);
  font-size: 12.5px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.plugin-switch {
  display: inline-flex;
}
.plugin-details {
  display: grid;
  gap: 10px;
  margin: 2px 14px 16px;
  padding: 14px 16px;
  border: 1px solid var(--divider);
  border-radius: 10px;
  background: var(--surface-subtle);
}
.plugin-details p {
  margin: 0;
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.75;
}
.plugin-chips {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
}
.plugin-chip {
  padding: 3px 9px;
  border: 1px solid var(--divider);
  border-radius: var(--radius-full);
  background: var(--surface);
  color: var(--text-secondary);
  font-size: 11px;
  line-height: 1.5;
}
.plugin-chip.accent {
  border-color: transparent;
  background: var(--accent-wash);
  color: var(--accent-text);
}
.plugin-expand-enter-active,
.plugin-expand-leave-active {
  transition:
    opacity var(--duration-fast) ease,
    transform var(--duration-fast) var(--ease-out);
}
.plugin-expand-enter-from,
.plugin-expand-leave-to {
  opacity: 0;
  transform: translateY(-3px);
}
</style>
