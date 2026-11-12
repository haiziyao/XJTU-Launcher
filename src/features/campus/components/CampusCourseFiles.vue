<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { FolderOpen, Search } from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import CampusHeaderSlot from './CampusHeaderSlot.vue';
import { bridge } from '../../../shared/bridge';
import { loadLmsOverview } from '../campusOverview';
import { applyCached } from '../../../shared/cache';
import { courseHue, fileSize, lmsTime, updatedInfo } from '../../../shared/domain';
import type { LmsLibraryFile, LmsOverview } from '../../../types';

/**
 * 课程资料 (layout v2): files attached to material activities in the current
 * semester, as a course index on the left, one dense table in the middle and
 * the selected file on the right. The overview has no upload time, only the
 * material activity's end date, so the column says 截止 and never claims a
 * "latest" order. Downloading reuses the same read-only LMS file endpoints.
 */
type Kind = 'all' | 'slide' | 'doc' | 'other';
interface FileGroup {
  title: string;
  items: LmsLibraryFile[];
}
const files = ref<LmsLibraryFile[]>([]);
const semester = ref('');
const failed = ref(0);
const query = ref('');
const courseFilter = ref('');
const kind = ref<Kind>('all');
const loading = ref(true);
const loadedAt = ref<number | null>(null);
const error = ref('');
const notice = ref('');
const busy = ref(0);
const selectedId = ref<number | null>(null);

const ext = (name: string) =>
  name.includes('.') ? name.split('.').pop()!.toUpperCase().slice(0, 4) : '文件';
const kindOf = (name: string): Kind => {
  const suffix = name.includes('.') ? name.split('.').pop()!.toLowerCase() : '';
  if (['ppt', 'pptx', 'key'].includes(suffix)) return 'slide';
  if (['pdf', 'doc', 'docx', 'txt', 'md', 'xls', 'xlsx', 'csv', 'rtf'].includes(suffix))
    return 'doc';
  return 'other';
};
const kindCount = (value: Kind) =>
  value === 'all'
    ? visible.value.length
    : visible.value.filter((item) => kindOf(item.file.name) === value).length;

const visible = computed(() => {
  const keyword = query.value.trim().toLowerCase();
  if (!keyword) return files.value;
  return files.value.filter((item) =>
    `${item.file.name} ${item.title} ${item.course}`.toLowerCase().includes(keyword),
  );
});
const courses = computed(() => {
  const map = new Map<string, number>();
  for (const item of visible.value) {
    const course = item.course || '未标注课程';
    map.set(course, (map.get(course) ?? 0) + 1);
  }
  return [...map.entries()]
    .map(([course, count]) => ({ course, count }))
    .sort((left, right) => left.course.localeCompare(right.course, 'zh-Hans-CN'));
});
const scoped = computed(() => {
  const list = visible.value.filter(
    (item) => !courseFilter.value || item.course === courseFilter.value,
  );
  return kind.value === 'all' ? list : list.filter((item) => kindOf(item.file.name) === kind.value);
});
const groups = computed<FileGroup[]>(() => {
  const map = new Map<string, FileGroup>();
  for (const item of scoped.value) {
    const title = item.title || '未标注资料';
    const group = map.get(title);
    if (group) group.items.push(item);
    else map.set(title, { title, items: [item] });
  }
  const result = [...map.values()];
  for (const group of result) {
    group.items.sort((left, right) => left.file.name.localeCompare(right.file.name, 'zh-Hans-CN'));
  }
  return result.sort((left, right) => left.title.localeCompare(right.title, 'zh-Hans-CN'));
});
const scopedSize = computed(() =>
  scoped.value.reduce((sum, item) => sum + (item.file.size ?? 0), 0),
);
const selected = computed(
  () => scoped.value.find((item) => item.file.id === selectedId.value) ?? null,
);
const meta = computed(() => {
  if (!files.value.length) return '';
  const updated = loadedAt.value ? ` · ${updatedInfo(loadedAt.value)}` : '';
  return (
    [semester.value, `${files.value.length} 个文件`, `${courses.value.length} 门课程`]
      .filter(Boolean)
      .join(' · ') + updated
  );
});
const courseColor = (name: string) => `c${courseHue(name) + 1}`;

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

watch(
  scoped,
  (list) => {
    if (!list.some((item) => item.file.id === selectedId.value)) {
      selectedId.value = list[0]?.file.id ?? null;
    }
  },
  { immediate: true },
);

onMounted(() => void load());
</script>

<template>
  <CampusHeaderSlot slot="meta">
    <span v-if="meta">{{ meta }}</span>
    <span v-if="failed" class="cv-due-soon">· {{ failed }} 个请求失败</span>
  </CampusHeaderSlot>
  <CampusHeaderSlot slot="tools">
    <label class="cv-search">
      <Search :size="13" aria-hidden="true" />
      <input
        v-model="query"
        type="search"
        placeholder="搜索文件"
        aria-label="搜索资料"
        spellcheck="false"
      />
    </label>
    <button type="button" class="cv-btn" :disabled="loading" @click="load(true)">刷新</button>
  </CampusHeaderSlot>

  <div class="cv-cols lcr">
    <div class="cv-panel cv-list">
      <div class="cv-lh">课程 {{ courses.length }}<span>文件</span></div>
      <button
        type="button"
        class="cv-item plain"
        :class="{ on: courseFilter === '' }"
        @click="courseFilter = ''"
      >
        <b>全部课程</b><span class="cv-n">{{ visible.length }}</span>
      </button>
      <button
        v-for="entry in courses"
        :key="entry.course"
        type="button"
        class="cv-item"
        :class="{ on: courseFilter === entry.course }"
        @click="toggleCourse(entry.course)"
      >
        <i class="cv-cs" :class="courseColor(entry.course)" aria-hidden="true"></i>
        <b>{{ entry.course }}</b>
        <span class="cv-n">{{ entry.count }}</span>
      </button>
    </div>

    <div class="cv-center">
      <div v-if="loading && !files.length" class="cv-empty">
        <FolderOpen :size="20" /><b>正在汇总本学期课程资料…</b>
        <span>第一次会扫描本学期全部课程，之后会先使用缓存。</span>
      </div>
      <div v-else-if="!files.length && !error" class="cv-empty">
        <FolderOpen :size="20" /><b>当前学期还没有课程资料</b>
        <span>这里只显示当前学期课程中带有附件的资料活动。</span>
      </div>
      <div v-else-if="!visible.length" class="cv-empty">
        <FolderOpen :size="20" /><b>没有符合搜索条件的文件</b>
      </div>

      <template v-else>
        <div class="cv-ch">
          <b>{{ courseFilter || '全部课程' }}</b>
          <span class="meta">
            {{ scoped.length }} 个文件<template v-if="scopedSize">
              · {{ fileSize(scopedSize) }}</template
            >
          </span>
          <span class="end">
            <div class="cv-seg">
              <button
                v-for="option in [
                  { id: 'all', label: '全部' },
                  { id: 'slide', label: '课件' },
                  { id: 'doc', label: '文档' },
                  { id: 'other', label: '其他' },
                ] as { id: Kind; label: string }[]"
                :key="option.id"
                type="button"
                :class="{ on: kind === option.id }"
                @click="kind = option.id"
              >
                {{ option.label }}<span class="n">{{ kindCount(option.id) }}</span>
              </button>
            </div>
          </span>
        </div>

        <div v-if="!scoped.length" class="cv-empty">
          <FolderOpen :size="20" /><b>这个筛选下没有文件</b>
        </div>
        <div v-else class="cv-panel cv-tbl">
          <div class="cv-thead" style="--cols: 1fr 76px 64px">
            <span>文件</span><span class="cv-r">大小</span><span class="cv-r">截止</span>
          </div>
          <template v-for="group in groups" :key="group.title">
            <div class="cv-grp">
              {{ group.title }}<span class="n">{{ group.items.length }}</span>
            </div>
            <div
              v-for="item in group.items"
              :key="item.file.id"
              class="cv-tr"
              style="--cols: 1fr 76px 64px"
              :class="{ sel: selectedId === item.file.id }"
              @click="selectedId = item.file.id"
            >
              <span class="c">
                <i class="cv-ft">{{ ext(item.file.name) }}</i>
                <span class="cv-ellip">{{ item.file.name }}</span>
              </span>
              <span class="cv-r cv-num cv-muted">
                {{ item.file.size ? fileSize(item.file.size) : '—' }}
              </span>
              <span class="cv-r cv-num cv-muted">{{ day(item.end) || '—' }}</span>
            </div>
          </template>
        </div>
      </template>
    </div>

    <div class="cv-panel cv-detail">
      <template v-if="selected">
        <div class="cv-type">
          <i class="cv-ft">{{ ext(selected.file.name) }}</i
          >课程资料
        </div>
        <h3>{{ selected.file.name }}</h3>
        <dl class="cv-kv">
          <dt>课程</dt>
          <dd>{{ selected.course || '未标注课程' }}</dd>
          <dt>来自</dt>
          <dd>{{ selected.title || '—' }}</dd>
          <dt>大小</dt>
          <dd class="cv-num">{{ selected.file.size ? fileSize(selected.file.size) : '—' }}</dd>
          <dt>截止</dt>
          <dd class="cv-num">{{ selected.end ? lmsTime(selected.end) : '—' }}</dd>
        </dl>
        <div class="cv-blk">
          <p>下载时会询问保存位置，文件不会自动放进工作台。</p>
        </div>
        <div class="cv-foot">
          <button
            type="button"
            class="cv-btn primary"
            :disabled="busy === selected.file.id"
            @click="downloadFile(selected)"
          >
            {{ busy === selected.file.id ? '保存中…' : '下载' }}
          </button>
        </div>
      </template>
      <div v-else class="cv-placeholder">选择左侧的一个文件查看详情。</div>
    </div>
  </div>
</template>
