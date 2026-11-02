<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { FolderOpen } from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import SectionHeader from '../ui/SectionHeader.vue';
import FilterBar from '../ui/FilterBar.vue';
import FileRow from './FileRow.vue';
import EmptyState from '../ui/EmptyState.vue';
import InlineAlert from '../ui/InlineAlert.vue';
import UiButton from '../ui/UiButton.vue';
import { bridge } from '../../bridge';
import { loadLmsOverview } from '../../campusOverview';
import { applyCached } from '../../cache';
import { fileSize } from '../../domain';
import type { LmsLibraryFile, LmsOverview } from '../../types';

/**
 * 课程资料 (LAYOUT §9.3): files attached to material activities in the
 * current semester, grouped by course. This is the school's course material,
 * not the user's local file library. Opening and downloading are user-initiated and
 * reuse the same read-only LMS file endpoints as the course page.
 */
interface FileGroup {
  course: string;
  items: LmsLibraryFile[];
}
const files = ref<LmsLibraryFile[]>([]);
const semester = ref('');
const failed = ref(0);
const query = ref('');
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
const notice = ref('');
const busy = ref(0);

const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return files.value;
  return files.value.filter((item) =>
    `${item.file.name} ${item.title} ${item.course}`.toLowerCase().includes(keyword),
  );
});
const groups = computed<FileGroup[]>(() => {
  const map = new Map<string, FileGroup>();
  for (const item of visible.value) {
    const course = item.course || '未标注课程';
    const group = map.get(course);
    if (group) group.items.push(item);
    else map.set(course, { course, items: [item] });
  }
  return [...map.values()].sort((left, right) =>
    left.course.localeCompare(right.course, 'zh-Hans-CN'),
  );
});
const subtitle = computed(() => {
  if (!files.value.length) return '';
  const courses = new Set(files.value.map((item) => item.course)).size;
  return [semester.value, `${files.value.length} 个文件`, `${courses} 门课程`]
    .filter(Boolean)
    .join(' · ');
});

async function load(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<LmsOverview>('lmsOverview', undefined, (value) => {
      if (!files.value.length) files.value = value.files;
      if (!semester.value) semester.value = value.semester;
      failed.value = value.failed ?? 0;
    });
    const fresh = await loadLmsOverview(force);
    files.value = fresh.files;
    semester.value = fresh.semester;
    failed.value = fresh.failed;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[campus] lms library', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function openFile(item: LmsLibraryFile) {
  error.value = '';
  notice.value = '';
  busy.value = item.file.id;
  try {
    await bridge.lmsOpenFile(item.file.id);
  } catch (e) {
    console.error('[campus] lms open file', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = 0;
  }
}

async function downloadFile(item: LmsLibraryFile) {
  error.value = '';
  notice.value = '';
  const target = await save({
    defaultPath: await bridge.saveTarget(item.file.name || '附件'),
  });
  if (!target) return;
  busy.value = item.file.id;
  try {
    const bytes = await bridge.lmsSaveFile(item.file.id, target);
    notice.value = `已保存到 ${await bridge.displayPath(target)}${bytes ? `（${fileSize(bytes)}）` : ''}`;
  } catch (e) {
    console.error('[campus] lms save file', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    busy.value = 0;
  }
}

onMounted(() => void load());
</script>

<template>
  <section class="campus-section">
    <SectionHeader
      title="课程资料"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading"
      @refresh="load(true)"
    />

    <FilterBar
      :query="query"
      :active="Boolean(query)"
      placeholder="搜索文件名、资料名或课程"
      label="搜索资料"
      @update:query="query = $event"
      @clear="query = ''"
    />

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="failed" tone="partial" title="部分内容未能读取">
      {{ failed }} 个课程或作业请求失败，列表可能不完整；刷新可重试。
    </InlineAlert>
    <InlineAlert v-if="notice" tone="info" :title="notice" dismissible @dismiss="notice = ''" />
    <p v-if="files.length" class="library-source">
      这是本学期课程中老师发布的资料附件；打开或下载由你手动触发。
    </p>

    <EmptyState
      v-if="loading && !files.length"
      :icon="FolderOpen"
      compact
      busy
      title="正在汇总本学期课程资料…"
      description="第一次会扫描本学期全部课程，之后会先使用缓存。"
    />
    <EmptyState
      v-else-if="!files.length && !error"
      :icon="FolderOpen"
      title="当前学期还没有课程资料"
      description="这里只显示当前学期课程中带有附件的资料活动。"
    />
    <EmptyState
      v-else-if="!visible.length"
      :icon="FolderOpen"
      compact
      title="没有符合搜索条件的文件"
      description="换一个文件名或课程名试试。"
    />

    <section v-for="group in groups" :key="group.course" class="library-group">
      <h3 class="library-group-title">
        {{ group.course }}
        <span>{{ group.items.length }}</span>
      </h3>
      <div class="library-list">
        <FileRow
          v-for="item in group.items"
          :key="item.file.id"
          :name="item.file.name"
          :size="item.file.size"
          :detail="[item.title, item.semester].filter(Boolean).join(' · ')"
          :previewable="false"
          :downloadable="item.file.allowDownload"
          :busy="busy === item.file.id"
          @download="downloadFile(item)"
        >
          <template #actions>
            <UiButton
              v-if="item.file.allowDownload"
              size="sm"
              variant="ghost"
              :disabled="busy === item.file.id"
              @click="openFile(item)"
            >
              打开
            </UiButton>
            <span v-else class="library-locked">不可下载</span>
          </template>
        </FileRow>
      </div>
    </section>
  </section>
</template>

<style scoped>
.library-source {
  margin: 0 0 14px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.library-group + .library-group {
  margin-top: 18px;
}
.library-group-title {
  display: flex;
  align-items: center;
  gap: 6px;
  margin: 0 0 6px;
  padding: 0 4px;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
}
.library-group-title span {
  color: var(--text-tertiary);
  font-weight: 500;
  font-variant-numeric: tabular-nums;
}
.library-list {
  padding: 4px;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.library-list > :deep(.file-row + .file-row) {
  border-top: 1px solid var(--border);
}
.library-locked {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
</style>
