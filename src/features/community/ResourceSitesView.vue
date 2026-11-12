<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import { ArrowLeft, Copy, Globe, LoaderCircle, TriangleAlert } from 'lucide-vue-next';
import { bridge } from '../../shared/bridge';
import { copyText } from '../../shared/clipboard';
import { resourceTabs, sitesForTab } from './community';
import type { ResourceCatalog, ResourceSite } from '../../types';
import PageHeader from '../../components/ui/PageHeader.vue';
import SegmentedTabs from '../../components/ui/SegmentedTabs.vue';
import EmptyState from '../../components/ui/EmptyState.vue';
import UiButton from '../../components/ui/UiButton.vue';

const props = defineProps<{
  catalog: ResourceCatalog | null;
  loading: boolean;
  error: string;
}>();
const emit = defineEmits<{ back: [] }>();

const activeTab = ref('recommended');
const copiedId = ref('');
const copyError = ref('');
let copyTimer = 0;

const tabs = computed(() => resourceTabs(props.catalog));
const sites = computed(() => sitesForTab(props.catalog, activeTab.value));

watch(
  tabs,
  (value) => {
    if (!value.some((tab) => tab.id === activeTab.value)) activeTab.value = 'recommended';
  },
  { immediate: true },
);

function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url;
  }
}

async function copySite(site: ResourceSite) {
  if (!(await copyText(site.url))) {
    copyError.value = '复制失败，请手动复制地址';
    return;
  }
  copyError.value = '';
  copiedId.value = site.id;
  window.clearTimeout(copyTimer);
  copyTimer = window.setTimeout(() => (copiedId.value = ''), 1600);
}

function openSite(site: ResourceSite) {
  void bridge.openUrl(site.url);
}

onBeforeUnmount(() => window.clearTimeout(copyTimer));
</script>

<template>
  <section class="desktop-page community-page resource-page">
    <UiButton variant="ghost" class="back-button" @click="emit('back')">
      <ArrowLeft :size="15" />返回社区广场
    </UiButton>
    <PageHeader title="资源站点" description="校园服务与常用网站入口" />

    <EmptyState
      v-if="loading"
      :icon="LoaderCircle"
      busy
      title="正在读取资源站点"
      compact
      class="resource-state"
    />
    <EmptyState
      v-else-if="error"
      :icon="TriangleAlert"
      tone="danger"
      title="无法读取资源站点"
      :description="error"
      compact
      class="resource-state"
    />
    <template v-else-if="catalog">
      <SegmentedTabs v-model="activeTab" :tabs="tabs" label="资源分类" compact />

      <div v-if="sites.length" class="resource-grid">
        <article v-for="site in sites" :key="site.id" class="resource-card">
          <button type="button" class="resource-main" @click="openSite(site)">
            <span class="resource-icon" aria-hidden="true">
              <img v-if="site.icon" :src="site.icon" alt="" />
              <Globe v-else :size="21" :stroke-width="1.6" />
            </span>
            <span class="resource-copy">
              <strong>{{ site.title }}</strong>
              <span class="resource-description">{{ site.description }}</span>
            </span>
          </button>
          <div v-if="site.tags.length" class="resource-tags">
            <span v-for="tag in site.tags" :key="tag">{{ tag }}</span>
          </div>
          <div class="resource-footer">
            <span class="resource-host" :title="site.url">{{ hostOf(site.url) }}</span>
            <button type="button" class="resource-copy-button" @click="copySite(site)">
              <Copy :size="13" />
              {{ copiedId === site.id ? '已复制' : '复制地址' }}
            </button>
          </div>
        </article>
      </div>
      <EmptyState
        v-else
        :icon="Globe"
        title="这个分类还没有站点"
        description="换一个分类看看。"
        compact
        class="resource-state"
      />
      <p v-if="copyError" class="resource-error" role="alert">{{ copyError }}</p>
    </template>
  </section>
</template>

<style scoped>
.resource-page {
  padding-bottom: 8px;
}
.resource-state {
  margin-top: 28px;
}

.resource-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 14px;
  margin-top: 16px;
}

.resource-card {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 15px 16px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--panel, var(--surface));
  box-shadow: var(--shadow-xs);
  transition:
    border-color var(--duration-fast) ease,
    box-shadow var(--duration-fast) ease;
}
.resource-card:hover {
  border-color: var(--border-strong);
  box-shadow: var(--shadow-sm);
}

.resource-main {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 0;
  border: 0;
  background: transparent;
  color: var(--text);
  text-align: left;
}
.resource-icon {
  display: grid;
  place-items: center;
  flex-shrink: 0;
  width: 40px;
  height: 40px;
  overflow: hidden;
  border-radius: var(--radius);
  background: var(--surface-subtle);
  color: var(--text-tertiary);
}
.resource-icon img {
  display: block;
  width: 40px;
  height: 40px;
}
.resource-copy {
  display: flex;
  flex-direction: column;
  gap: 3px;
  min-width: 0;
  padding-top: 1px;
}
.resource-copy strong {
  font-size: 14px;
  font-weight: 600;
}
.resource-description {
  color: var(--text-secondary);
  font-size: 12.5px;
  line-height: 1.55;
}

.resource-tags {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  padding-left: 52px;
}
.resource-tags span {
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--surface-subtle);
  color: var(--text-tertiary);
  font-size: 11.5px;
}

.resource-footer {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--divider);
}
.resource-host {
  min-width: 0;
  overflow: hidden;
  color: var(--text-tertiary);
  font-size: 11.5px;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.resource-copy-button {
  display: inline-flex;
  flex-shrink: 0;
  align-items: center;
  gap: 5px;
  padding: 4px 9px;
  border: 1px solid var(--control-border);
  border-radius: var(--radius-sm);
  background: var(--control-bg);
  color: var(--text-secondary);
  font-size: 12px;
  transition:
    border-color var(--duration-fast) ease,
    color var(--duration-fast) ease;
}
.resource-copy-button:hover {
  border-color: var(--border-strong);
  color: var(--text);
}
.resource-copy-button:active {
  transform: scale(0.97);
}
.resource-error {
  margin-top: 10px;
  color: var(--danger);
  font-size: 12px;
}

@media (max-width: 860px) {
  .resource-grid {
    grid-template-columns: minmax(0, 1fr);
  }
}
@media (prefers-reduced-motion: reduce) {
  .resource-copy-button:active {
    transform: none;
  }
}
</style>
