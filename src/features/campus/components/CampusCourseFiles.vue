<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { FolderOpen } from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import SectionHeader from '../../../components/ui/SectionHeader.vue';
import EmptyState from '../../../components/ui/EmptyState.vue';
import InlineAlert from '../../../components/ui/InlineAlert.vue';
import UiButton from '../../../components/ui/UiButton.vue';
import { bridge } from '../../../shared/bridge';
import { loadLmsOverview } from '../campusOverview';
import { applyCached } from '../../../shared/cache';
import { fileSize, lmsTime } from '../../../shared/domain';
import type { LmsLibraryFile, LmsOverview } from '../../../types';

/**
 * 课程资料 (LAYOUT §9.3, version B): files attached to material activities in
 * the current semester, shown as one card per course plus a cross-course
 * table. Clicking a card narrows the table to that course.
 * The overview has no upload time, only the material activity's end date, so
 * the table labels that column 活动截止 and never claims a "latest" order.
 * Opening and downloading are user-initiated and reuse the same read-only LMS
 * file endpoints as the course page.
 */
interface FileGroup {
  course: string;
  items: LmsLibraryFile[];
  size: number;
  types: string[];
}
const files = ref<LmsLibraryFile[]>([]);
const semester = ref('');
const failed = ref(0);
const query = ref('');
const courseFilter = ref('');
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
    if (group) {
      group.items.push(item);
      group.size += item.file.size ?? 0;
    } else {
      map.set(course, {
        course,
        items: [item],
        size: item.file.size ?? 0,
        types: [],
      });
    }
  }
  const result = [...map.values()];
  for (const group of result) {
    group.items.sort((left, right) => left.file.name.localeCompare(right.file.name, 'zh-Hans-CN'));
    group.types = [
      ...new Set(
        group.items
          .map((item) =>
            item.file.name.includes('.') ? item.file.name.split('.').pop()!.toUpperCase() : '',
          )
          .filter(Boolean),
      ),
    ].slice(0, 3);
  }
  return result.sort((left, right) => left.course.localeCompare(right.course, 'zh-Hans-CN'));
});
const tableFiles = computed(() =>
  [...visible.value]
    .filter((item) => !courseFilter.value || item.course === courseFilter.value)
    .sort(
      (left, right) =>
        left.course.localeCompare(right.course, 'zh-Hans-CN') ||
        left.file.name.localeCompare(right.file.name, 'zh-Hans-CN'),
    )
    .slice(0, 8),
);
const filteredCount = computed(
  () => visible.value.filter((item) => item.course === courseFilter.value).length,
);
const subtitle = computed(() => {
  if (!files.value.length) return '';
  const courses = new Set(files.value.map((item) => item.course)).size;
  return [semester.value, `${files.value.length} 个文件`, `${courses} 门课程`]
    .filter(Boolean)
    .join(' · ');
});

function day(value: string): string {
  const text = lmsTime(value);
  return text.length >= 10 ? text.slice(5, 10) : text;
}

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

function toggleCourse(course: string) {
  courseFilter.value = courseFilter.value === course ? '' : course;
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
    >
      <template #filters>
        <label class="field-shell search files-search">
          <FolderOpen :size="13" aria-hidden="true" />
          <input
            v-model="query"
            type="search"
            placeholder="搜索文件名、资料名或课程"
            aria-label="搜索资料"
            spellcheck="false"
          />
        </label>
        <UiButton
          v-if="courseFilter || query"
          variant="ghost"
          size="sm"
          @click="
            courseFilter = '';
            query = '';
          "
        >
          清除筛选
        </UiButton>
      </template>
    </SectionHeader>

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="failed" tone="partial" title="部分内容未能读取">
      {{ failed }} 个课程或作业请求失败，列表可能不完整；刷新可重试。
    </InlineAlert>
    <InlineAlert v-if="notice" tone="info" :title="notice" dismissible @dismiss="notice = ''" />

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

    <template v-if="visible.length">
      <div class="campus-cards">
        <article
          v-for="group in groups"
          :key="group.course"
          class="campus-course-card"
          :class="{ active: courseFilter === group.course }"
          role="button"
          tabindex="0"
          :aria-pressed="courseFilter === group.course"
          @click="toggleCourse(group.course)"
          @keydown.enter.prevent="toggleCourse(group.course)"
          @keydown.space.prevent="toggleCourse(group.course)"
        >
          <span class="card-title">{{ group.course }}</span>
          <span class="card-meta">
            <span>{{ group.items.length }} 个文件</span>
            <span v-if="group.size">{{ fileSize(group.size) }}</span>
          </span>
          <ul class="card-files">
            <li v-for="item in group.items.slice(0, 3)" :key="item.file.id">
              {{ item.file.name }}
            </li>
          </ul>
          <span class="card-foot">
            <span
              >{{ group.types.join(' · ') || '文件'
              }}<template v-if="group.size"> · {{ fileSize(group.size) }}</template></span
            >
            <span class="card-open">{{ courseFilter === group.course ? '已筛选' : '筛选 ›' }}</span>
          </span>
        </article>
      </div>

      <div class="files-subhead">
        <div>
          <h3>全部文件</h3>
          <span>{{
            courseFilter
              ? `${courseFilter} · ${filteredCount} 个文件`
              : `跨课程 · 共 ${visible.length} 个文件 · 按课程排列`
          }}</span>
        </div>
        <UiButton v-if="courseFilter" variant="ghost" size="sm" @click="courseFilter = ''">
          显示全部课程
        </UiButton>
      </div>

      <div class="campus-table">
        <div class="campus-table-head files-head">
          <span>文件</span>
          <span>课程</span>
          <span class="num">大小</span>
          <span class="num">活动截止</span>
          <span class="num">操作</span>
        </div>
        <div v-for="item in tableFiles" :key="item.file.id" class="campus-row files-row">
          <span class="name" :title="item.file.name">{{ item.file.name }}</span>
          <span class="course" :title="item.course">{{ item.course || '未标注课程' }}</span>
          <span class="num muted">{{ item.file.size ? fileSize(item.file.size) : '—' }}</span>
          <span class="num muted" :title="item.end ? lmsTime(item.end) : ''">{{
            day(item.end) || '—'
          }}</span>
          <button
            v-if="item.file.allowDownload"
            type="button"
            class="row-action"
            :disabled="busy === item.file.id"
            @click="downloadFile(item)"
          >
            {{ busy === item.file.id ? '保存中' : '下载' }}
          </button>
          <button v-else type="button" class="row-action" @click="openFile(item)">打开</button>
        </div>
      </div>
    </template>
  </section>
</template>

<style scoped>
.files-search {
  width: 240px;
}
.files-subhead {
  display: flex;
  align-items: flex-end;
  justify-content: space-between;
  gap: 12px;
  margin: 4px 0 10px;
}
.files-subhead h3 {
  margin: 0;
  font-size: 13px;
  font-weight: 600;
}
.files-subhead span {
  margin-left: 8px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.files-head,
.files-row {
  grid-template-columns: minmax(0, 2fr) minmax(0, 1.4fr) 76px 64px 56px;
}
.files-row .name {
  font-weight: 400;
}
</style>
