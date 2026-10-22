<script setup lang="ts">
import { computed, ref } from 'vue';
import {
  BookMarked,
  BookOpen,
  Building2,
  ExternalLink,
  GraduationCap,
  Link2,
  Search,
} from 'lucide-vue-next';
import type { Link } from '../types';
import PageHeader from './ui/PageHeader.vue';
import SettingGroup from './ui/SettingGroup.vue';
import SettingRow from './ui/SettingRow.vue';
import EmptyState from './ui/EmptyState.vue';
import IconTile, { type Tone } from './ui/IconTile.vue';
const props = defineProps<{ name: string; links: Link[] }>();
const emit = defineEmits<{ open: [url: string] }>();
const query = ref('');
const matches = computed(() =>
  props.links.filter((link) =>
    (link.title + link.description).toLowerCase().includes(query.value.trim().toLowerCase()),
  ),
);
function style(title: string): { icon: typeof Link2; tone: Tone } {
  if (title.includes('一网通办')) return { icon: Building2, tone: 'blue' };
  if (title.includes('教务')) return { icon: GraduationCap, tone: 'violet' };
  if (title.includes('图书')) return { icon: BookMarked, tone: 'green' };
  if (title.includes('学堂')) return { icon: BookOpen, tone: 'teal' };
  return { icon: Link2, tone: 'blue' };
}
</script>
<template>
  <section class="desktop-page narrow-page">
    <PageHeader :title="name" description="使用当前校园账号打开校园网站">
      <template #actions>
        <label class="field-shell search"
          ><Search :size="14" /><input
            v-model="query"
            placeholder="搜索校园网站"
            aria-label="搜索校园网站"
        /></label>
      </template>
    </PageHeader>
    <SettingGroup>
      <SettingRow
        v-for="link in matches"
        :key="link.url"
        clickable
        :title="link.title"
        :description="link.description"
        @select="emit('open', link.url)"
      >
        <template #leading>
          <IconTile :tone="style(link.title).tone" :size="32">
            <component :is="style(link.title).icon" :size="16" />
          </IconTile>
        </template>
        <ExternalLink :size="14" class="muted" />
      </SettingRow>
      <EmptyState v-if="!matches.length" compact :icon="Search" title="没有匹配的网站" />
    </SettingGroup>
  </section>
</template>
