<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { ask } from '@tauri-apps/plugin-dialog';
import {
  ChevronDown,
  ChevronRight,
  FileText,
  Folder,
  FolderOpen,
  FolderSearch,
  Plus,
  RefreshCw,
  Search,
  Upload,
  X,
} from 'lucide-vue-next';
import { bridge } from '../../shared/bridge';
import { courseHue, displayCourseCode } from '../../shared/domain';
import type {
  LmsSubmitPreview,
  WorkbenchAssignment,
  WorkbenchCourse,
  WorkbenchFile,
  WorkbenchFolder,
  WorkbenchOverview,
} from '../../types';
import InlineAlert from '../../components/ui/InlineAlert.vue';

/**
 * 工作台 shows the user's own folders rather than a private file store: the
 * semester folder is on top, each course folder carries a hidden marker, and
 * every other folder is the user's own. Assignments come from the last
 * 思源学堂 sync and live in app state, not on disk. Submission keeps the
 * preview → explicit confirm → post sequence.
 */

const overview = ref<WorkbenchOverview | null>(null);
const loading = ref(true);
const error = ref('');
const notice = ref('');
const selected = ref('');
const expanded = ref(new Set<string>());
const files = ref<WorkbenchFile[]>([]);
const filesLoading = ref(false);
const search = ref('');
const busy = ref(false);
/** The file shown in the right inspector; separate from the folder selection. */
const pickedFile = ref<WorkbenchFile | null>(null);

const folderModal = ref(false);
const folderName = ref('');

const bindOpen = ref(false);
const bindFile = ref<WorkbenchFile | null>(null);
const bindAssignment = ref('');
const bindNote = ref('');

const submitOpen = ref(false);
const submitAssignment = ref<WorkbenchAssignment | null>(null);
const submitFiles = ref<WorkbenchFile[]>([]);
const submitChecked = ref(new Set<string>());
const submitComment = ref('');
const preview = ref<LmsSubmitPreview | null>(null);
const previewing = ref(false);
const submitting = ref(false);
const submitError = ref('');
const lastSubmit = ref('');

const children = computed(() => {
  const map = new Map<string, WorkbenchFolder[]>();
  for (const folder of overview.value?.folders ?? []) {
    const list = map.get(folder.parent) ?? [];
    list.push(folder);
    map.set(folder.parent, list);
  }
  return map;
});

const terms = computed(() => {
  const list = [...(children.value.get('') ?? [])];
  list.sort((left, right) => {
    if (left.name === overview.value?.currentTerm) return -1;
    if (right.name === overview.value?.currentTerm) return 1;
    return right.name.localeCompare(left.name, 'zh-CN');
  });
  return list;
});

const allFolders = computed(() => new Map((overview.value?.folders ?? []).map((f) => [f.path, f])));
const selectedFolder = computed(() => allFolders.value.get(selected.value) ?? null);

/** Nearest course at or above the selection. */
const selectedCourse = computed<WorkbenchCourse | null>(() => {
  let folder = selectedFolder.value;
  while (folder) {
    const meta = folder.course;
    if (meta) {
      const course = (overview.value?.courses ?? []).find(
        (item) => item.folder === folder!.path || sameCourse(item, meta),
      );
      if (course) return course;
      return {
        id: meta.id,
        code: meta.code,
        subjectCode: meta.subjectCode,
        name: meta.name,
        folder: folder.path,
        assignments: [],
      };
    }
    folder = allFolders.value.get(folder.parent) ?? null;
  }
  return null;
});

/** The semester folder the selection is filed under. */
const selectedTerm = computed(() => {
  let folder = selectedFolder.value;
  while (folder) {
    if (folder.kind === 'term') return folder;
    folder = allFolders.value.get(folder.parent) ?? null;
  }
  return null;
});

/** Root → selection, so a folder nested inside a course keeps its context. */
const breadcrumbs = computed(() => {
  const trail: WorkbenchFolder[] = [];
  let folder = selectedFolder.value;
  while (folder) {
    trail.unshift(folder);
    folder = allFolders.value.get(folder.parent) ?? null;
  }
  return trail;
});

const selectedMeta = computed(() => selectedCourse.value ?? null);

/** The stored term on the nearest course marker, for folders inside a course. */
const selectedCourseTerm = computed(() => {
  let folder = selectedFolder.value;
  while (folder) {
    if (folder.course) return folder.course.term;
    folder = allFolders.value.get(folder.parent) ?? null;
  }
  return '';
});

const visibleFolders = computed(() => {
  const rows: { folder: WorkbenchFolder; depth: number }[] = [];
  const walk = (list: WorkbenchFolder[], depth: number) => {
    for (const folder of list) {
      rows.push({ folder, depth });
      if (expanded.value.has(folder.path)) walk(children.value.get(folder.path) ?? [], depth + 1);
    }
  };
  walk(terms.value, 0);
  return rows;
});

const visibleFiles = computed(() => {
  const needle = search.value.trim().toLowerCase();
  if (!needle) return files.value;
  return files.value.filter((file) => file.name.toLowerCase().includes(needle));
});

/** Folders directly below the selection, listed above its files. */
const childFolders = computed(() => children.value.get(selected.value) ?? []);

/** The assignment a picked file is bound to, when the course list has it. */
const boundAssignment = computed(() => {
  const title = pickedFile.value?.assignment;
  if (!title) return null;
  return courseAssignments.value.find((item) => item.title === title) ?? null;
});

const selectedKind = computed(() => {
  const folder = selectedFolder.value;
  if (!folder) return '';
  if (folder.kind === 'term') return '学期文件夹';
  return folder.course ? '课程文件夹' : '自定义文件夹';
});

function fileExt(name: string) {
  return name.includes('.') ? name.split('.').pop()!.toUpperCase().slice(0, 4) : '文件';
}

function courseColor(folder: WorkbenchFolder) {
  return `c${courseHue(folder.course?.name || folder.name) + 1}`;
}

const termCourses = computed(() => {
  if (!selected.value) return [];
  const list = (children.value.get(selected.value) ?? []).filter((folder) => folder.course);
  const needle = search.value.trim().toLowerCase();
  return needle ? list.filter((folder) => folder.name.toLowerCase().includes(needle)) : list;
});

const totals = computed(() => {
  const folders = overview.value?.folders ?? [];
  return {
    courses: folders.filter((folder) => folder.course).length,
    files: folders.reduce((sum, folder) => sum + folder.files, 0),
    due: (overview.value?.courses ?? []).reduce(
      (sum, course) =>
        sum +
        course.assignments.filter(
          (assignment) => !assignment.closed && (assignment.submitCount ?? 0) === 0,
        ).length,
      0,
    ),
  };
});

const syncedLabel = computed(() => {
  const value = overview.value?.syncedAt ?? 0;
  if (!value) return '尚未同步';
  const date = new Date(value);
  return `作业清单已同步 ${date.getMonth() + 1}-${String(date.getDate()).padStart(2, '0')} ${String(date.getHours()).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
});

const courseAssignments = computed(() => {
  const list = selectedCourse.value?.assignments ?? [];
  const needle = search.value.trim().toLowerCase();
  return needle ? list.filter((item) => item.title.toLowerCase().includes(needle)) : list;
});

function sameCourse(
  course: WorkbenchCourse,
  meta: { id: number; code: string; subjectCode: string },
) {
  if (meta.subjectCode && course.subjectCode) return course.subjectCode === meta.subjectCode;
  return (meta.id && course.id === meta.id) || (meta.code && course.code === meta.code);
}

function courseFor(folder: WorkbenchFolder): WorkbenchCourse | null {
  const meta = folder.course;
  if (!meta) return null;
  return (
    (overview.value?.courses ?? []).find(
      (item) => item.folder === folder.path || sameCourse(item, meta),
    ) ?? null
  );
}

function courseDue(folder: WorkbenchFolder): number {
  return (courseFor(folder)?.assignments ?? []).filter(
    (assignment) => !assignment.closed && (assignment.submitCount ?? 0) === 0,
  ).length;
}

/** Files anywhere below a folder, for the tree's count. */
function folderTotal(folder: WorkbenchFolder): number {
  return (children.value.get(folder.path) ?? []).reduce(
    (sum, child) => sum + folderTotal(child),
    folder.files,
  );
}

function isExpanded(path: string) {
  return expanded.value.has(path);
}

function toggle(path: string) {
  const next = new Set(expanded.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  expanded.value = next;
}

function select(folder: WorkbenchFolder) {
  selected.value = folder.path;
  if (!expanded.value.has(folder.path) && (children.value.get(folder.path)?.length ?? 0) > 0) {
    const next = new Set(expanded.value);
    next.add(folder.path);
    expanded.value = next;
  }
}

function pickDefault() {
  const current = terms.value.find((folder) => folder.name === overview.value?.currentTerm);
  selected.value = current?.path || terms.value[0]?.path || '';
  const next = new Set(expanded.value);
  for (const term of terms.value) next.add(term.path);
  expanded.value = next;
}

async function load() {
  loading.value = true;
  error.value = '';
  try {
    overview.value = await bridge.workbenchOverview();
    const before = selected.value;
    if (!selected.value || !allFolders.value.has(selected.value)) pickDefault();
    // A changed selection loads through the watcher; only refresh in place.
    if (selected.value === before) await loadFiles();
  } catch (failure) {
    error.value = String(failure);
  } finally {
    loading.value = false;
  }
}

async function loadFiles() {
  const folder = selectedFolder.value;
  if (!folder || folder.kind === 'term') {
    files.value = [];
    return;
  }
  filesLoading.value = true;
  try {
    files.value = await bridge.workbenchFolder(folder.path);
  } catch (failure) {
    files.value = [];
    error.value = String(failure);
  } finally {
    filesLoading.value = false;
  }
}

watch(selected, () => {
  lastSubmit.value = '';
  loadFiles();
});

// Keep the inspector on a file that still exists; a new folder picks its first.
watch(
  visibleFiles,
  (list) => {
    const current = pickedFile.value;
    if (!current || !list.some((file) => file.path === current.path)) {
      pickedFile.value = list[0] ?? null;
    }
  },
  { immediate: true },
);

async function sync() {
  busy.value = true;
  error.value = '';
  notice.value = '';
  try {
    const result = await bridge.workbenchSync();
    notice.value = `已同步 ${result.term}：${result.courses} 门课程、${result.assignments} 个作业${
      result.failed ? `，${result.failed} 个请求失败` : ''
    }`;
    await load();
  } catch (failure) {
    error.value = String(failure);
  } finally {
    busy.value = false;
  }
}

function openFolderModal() {
  if (!selectedFolder.value) {
    error.value = '请先在右侧选择一个文件夹';
    return;
  }
  folderName.value = '';
  folderModal.value = true;
}

async function createFolder() {
  const name = folderName.value.trim();
  if (!name || !selected.value) return;
  busy.value = true;
  try {
    const folder = await bridge.workbenchCreateFolder(selected.value, name);
    folderModal.value = false;
    notice.value = `已创建文件夹「${folder.name}」`;
    await load();
    selected.value = folder.path;
  } catch (failure) {
    error.value = String(failure);
  } finally {
    busy.value = false;
  }
}

async function addFiles() {
  if (!selectedFolder.value || selectedFolder.value.kind === 'term') {
    error.value = '请先选择一个课程或文件夹，再添加文件';
    return;
  }
  busy.value = true;
  try {
    const results = await bridge.workbenchAdd(selected.value);
    const added = results.filter((item) => item.file).length;
    const failed = results.filter((item) => item.error);
    if (added) notice.value = `已添加 ${added} 个文件`;
    if (failed.length) error.value = failed.map((item) => item.error).join('；');
    if (added) await load();
  } catch (failure) {
    error.value = String(failure);
  } finally {
    busy.value = false;
  }
}

async function openFile(file: WorkbenchFile) {
  try {
    await bridge.workbenchOpen(file.path);
  } catch (failure) {
    error.value = String(failure);
  }
}

async function reveal(path: string) {
  try {
    await bridge.workbenchReveal(path);
  } catch (failure) {
    error.value = String(failure);
  }
}

async function removeFile(file: WorkbenchFile) {
  const confirmed = await ask(`从工作台删除「${file.name}」？磁盘上的这个文件也会被删除。`, {
    title: '删除文件',
    kind: 'warning',
    okLabel: '删除',
    cancelLabel: '取消',
  });
  if (!confirmed) return;
  try {
    await bridge.workbenchDelete(file.path);
    notice.value = `已删除「${file.name}」`;
    await load();
  } catch (failure) {
    error.value = String(failure);
  }
}

function openBind(file: WorkbenchFile) {
  bindFile.value = file;
  bindAssignment.value = file.assignment;
  bindNote.value = file.note;
  bindOpen.value = true;
}

async function saveBinding() {
  const file = bindFile.value;
  if (!file) return;
  const assignment = courseAssignments.value.find((item) => item.title === bindAssignment.value);
  try {
    await bridge.workbenchUpdate(file.path, {
      assignment: bindAssignment.value.trim(),
      activityId: assignment?.id ?? null,
      tags: file.tags,
      note: bindNote.value.trim(),
    });
    bindOpen.value = false;
    notice.value = '已保存绑定';
    await loadFiles();
  } catch (failure) {
    error.value = String(failure);
  }
}

async function openSubmit(assignment: WorkbenchAssignment) {
  const folder = selectedCourse.value?.folder || selected.value;
  submitAssignment.value = assignment;
  submitComment.value = '';
  preview.value = null;
  submitError.value = '';
  lastSubmit.value = '';
  submitChecked.value = new Set();
  try {
    submitFiles.value = await bridge.workbenchFolder(folder);
  } catch (failure) {
    submitFiles.value = [];
    submitError.value = String(failure);
  }
  submitOpen.value = true;
}

function toggleSubmitFile(path: string) {
  const next = new Set(submitChecked.value);
  if (next.has(path)) next.delete(path);
  else next.add(path);
  submitChecked.value = next;
  preview.value = null;
}

async function previewSubmit() {
  const assignment = submitAssignment.value;
  if (!assignment) return;
  const paths = [...submitChecked.value];
  if (!paths.length && !submitComment.value.trim()) {
    submitError.value = '请选择文件或填写文字说明';
    return;
  }
  previewing.value = true;
  submitError.value = '';
  try {
    preview.value = await bridge.workbenchSubmitPreview(
      assignment.id,
      submitComment.value.trim(),
      paths,
    );
  } catch (failure) {
    preview.value = null;
    submitError.value = String(failure);
  } finally {
    previewing.value = false;
  }
}

async function confirmSubmit() {
  const assignment = submitAssignment.value;
  const checked = preview.value;
  if (!assignment || !checked) return;
  const what = checked.files.length ? `${checked.files.length} 个文件` : '文字说明';
  const ok = await ask(`确认把${what}提交到「${checked.title}」？提交后不能再撤回。`, {
    title: '确认提交作业',
    kind: 'warning',
    okLabel: '确认提交',
    cancelLabel: '取消',
  });
  if (!ok) return;
  submitting.value = true;
  submitError.value = '';
  try {
    const result = await bridge.workbenchSubmit(assignment.id, submitComment.value.trim(), [
      ...submitChecked.value,
    ]);
    submitOpen.value = false;
    lastSubmit.value = result.message || `已提交「${result.title}」`;
    notice.value = lastSubmit.value;
    await load();
    void refreshAfterSubmit();
  } catch (failure) {
    submitError.value = String(failure);
  } finally {
    submitting.value = false;
  }
}

// Pull the assignment list again so 未交 / 已交 matches the school. A failure
// here does not undo the submission, so it only restores the success notice.
async function refreshAfterSubmit() {
  const message = lastSubmit.value;
  try {
    await bridge.workbenchSync();
    await load();
  } catch {
    // The submission itself already succeeded; keep the local list as is.
  }
  notice.value = lastSubmit.value || message;
}

function statusOf(assignment: WorkbenchAssignment) {
  if ((assignment.submitCount ?? 0) > 0) return { text: '已提交', tone: 'ok' };
  if (assignment.closed) return { text: '已截止 · 未交', tone: 'muted' };
  return { text: '未交', tone: 'warn' };
}

function sizeLabel(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

function timeLabel(milliseconds: number) {
  if (!milliseconds) return '';
  const date = new Date(milliseconds);
  return `${date.getMonth() + 1}-${String(date.getDate()).padStart(2, '0')} ${String(
    date.getHours(),
  ).padStart(2, '0')}:${String(date.getMinutes()).padStart(2, '0')}`;
}

function shortDate(value: string) {
  return value ? value.slice(5, 16) : '不限';
}

onMounted(load);
</script>
<template>
  <section class="campus-v2 workbench">
    <header class="cv-top">
      <h1 class="cv-title">
        工作台
        <span class="wb-term">{{ overview?.currentTerm || '尚未同步学期' }}</span>
      </h1>
      <div class="cv-top-right">
        <label class="cv-search">
          <Search :size="13" aria-hidden="true" />
          <input
            v-model="search"
            type="search"
            placeholder="搜索文件或课程"
            aria-label="搜索文件或课程"
            spellcheck="false"
          />
        </label>
      </div>
    </header>

    <div class="cv-sub">
      <div class="cv-head-meta">
        <span
          >{{ totals.courses }} 门课程 · {{ totals.files }} 个文件 ·
          {{ totals.due }} 个作业未交</span
        >
        <span class="cv-vsep" aria-hidden="true"></span>
        <span>{{ syncedLabel }}</span>
      </div>
      <div class="cv-tools">
        <button type="button" class="cv-btn" :disabled="busy" @click="openFolderModal">
          <Plus :size="13" />新建文件夹
        </button>
        <button type="button" class="cv-btn" :disabled="busy" @click="addFiles">
          <Upload :size="13" />添加文件
        </button>
        <button type="button" class="cv-btn primary" :disabled="busy" @click="sync">
          <RefreshCw :size="13" :class="{ spinning: busy }" />同步
        </button>
      </div>
    </div>

    <InlineAlert v-if="error" tone="error" title="操作没有完成" dismissible @dismiss="error = ''">
      {{ error }}
    </InlineAlert>
    <InlineAlert v-if="notice" tone="info" dismissible @dismiss="notice = ''">{{
      notice
    }}</InlineAlert>

    <div class="cv-body">
      <div class="cv-cols lcr wb-cols">
        <aside class="cv-panel cv-list wb-tree" aria-label="文件树">
          <div class="tree-head">
            <span>文件夹</span>
            <button
              type="button"
              class="icon"
              title="新建文件夹"
              aria-label="新建文件夹"
              @click="openFolderModal"
            >
              <Plus :size="13" />
            </button>
          </div>
          <div class="tree-body">
            <div v-for="item in visibleFolders" :key="item.folder.path">
              <div
                class="tree-node"
                :class="{ active: item.folder.path === selected }"
                :style="{ paddingLeft: `${2 + item.depth * 12}px` }"
              >
                <button
                  v-if="item.folder.folders > 0"
                  type="button"
                  class="tw"
                  @click="toggle(item.folder.path)"
                  :aria-label="`${isExpanded(item.folder.path) ? '收起' : '展开'} ${item.folder.name}`"
                >
                  <ChevronDown v-if="isExpanded(item.folder.path)" :size="11" />
                  <ChevronRight v-else :size="11" />
                </button>
                <span v-else class="tw" aria-hidden="true"></span>
                <button type="button" class="tree-main" @click="select(item.folder)">
                  <FolderOpen
                    v-if="isExpanded(item.folder.path) && item.folder.folders > 0"
                    :size="14"
                    class="fi"
                  />
                  <Folder v-else :size="14" class="fi" />
                  <span class="nm">{{ item.folder.name }}</span>
                  <span
                    v-if="
                      item.folder.course?.name &&
                      item.folder.course.name !== item.folder.name &&
                      item.depth > 0
                    "
                    class="real"
                    >{{ item.folder.course.name }}</span
                  >
                  <span class="ct">{{ folderTotal(item.folder) }}</span>
                </button>
              </div>
            </div>
            <p v-if="!visibleFolders.length && !loading" class="wb-tree-empty">还没有文件夹</p>
          </div>
          <button type="button" class="tree-foot" @click="reveal('')">打开文件文件夹</button>
        </aside>

        <div class="cv-center">
          <div v-if="loading" class="cv-empty"><FolderOpen :size="20" /><b>正在读取文件夹…</b></div>
          <template v-else-if="selectedFolder">
            <div class="wb-crumb">
              <template v-for="(crumb, index) in breadcrumbs" :key="crumb.path">
                <ChevronRight v-if="index > 0" :size="11" aria-hidden="true" />
                <span v-if="index === breadcrumbs.length - 1">{{ crumb.name }}</span>
                <button v-else type="button" class="crumb" @click="select(crumb)">
                  {{ crumb.name }}
                </button>
              </template>
            </div>

            <template v-if="selectedFolder.kind === 'term'">
              <div class="cv-ch">
                <b>{{ selectedFolder.name }}</b>
                <span class="meta">
                  {{ termCourses.length }} 门课程 · {{ selectedFolder.files }} 个文件
                </span>
                <span class="end">
                  <button type="button" class="cv-btn sm" @click="reveal(selectedFolder.path)">
                    <FolderSearch :size="12" />在访达中显示
                  </button>
                </span>
              </div>
              <div v-if="termCourses.length" class="cv-panel cv-tbl">
                <div class="cv-thead" style="--cols: 1fr 64px 64px 84px">
                  <span>课程</span><span class="cv-r">文件</span><span class="cv-r">作业</span
                  ><span class="cv-r">未交</span>
                </div>
                <button
                  v-for="folder in termCourses"
                  :key="folder.path"
                  type="button"
                  class="cv-tr"
                  style="--cols: 1fr 64px 64px 84px"
                  @click="select(folder)"
                >
                  <span class="c">
                    <i class="cv-cs" :class="courseColor(folder)" aria-hidden="true"></i>
                    <span class="cv-tt">
                      <span class="cv-ellip">{{ folder.name }}</span>
                      <small>{{
                        displayCourseCode(folder.course?.subjectCode || folder.course?.code) ||
                        '课程文件夹'
                      }}</small>
                    </span>
                  </span>
                  <span class="cv-r cv-num cv-muted">{{ folder.files }}</span>
                  <span class="cv-r cv-num cv-muted">{{
                    courseFor(folder)?.assignments.length ?? 0
                  }}</span>
                  <span class="cv-r">
                    <span v-if="courseDue(folder)" class="cv-badge danger">
                      {{ courseDue(folder) }}
                    </span>
                    <span v-else class="cv-muted">—</span>
                  </span>
                </button>
              </div>
              <div v-else class="cv-empty">
                <Folder :size="20" /><b>这个学期还没有课程文件夹</b>
                <span>点右上角「同步」从思源学堂同步课程与作业清单。</span>
              </div>
            </template>

            <template v-else>
              <div class="cv-ch">
                <b>{{ selectedFolder.name }}</b>
                <span class="meta">
                  <template v-if="selectedMeta">
                    {{
                      displayCourseCode(selectedMeta.subjectCode || selectedMeta.code) ||
                      '未知编号'
                    }}<template v-if="selectedCourseTerm"> · {{ selectedCourseTerm }}</template>
                  </template>
                  <template v-else>
                    自定义文件夹 · {{ selectedFolder.files }} 个文件 ·
                    {{ selectedFolder.folders }} 个子文件夹
                  </template>
                </span>
                <span class="end">
                  <button type="button" class="cv-btn sm" @click="reveal(selectedFolder.path)">
                    <FolderSearch :size="12" />在访达中显示
                  </button>
                </span>
              </div>

              <div class="cv-scroll">
                <template v-if="selectedMeta && courseAssignments.length">
                  <div class="wb-sect">
                    <h2>作业</h2>
                    <span>{{ courseAssignments.length }}</span>
                    <span class="wb-hint">自动同步自习源学堂</span>
                  </div>
                  <div class="cv-panel cv-tbl">
                    <div class="cv-thead" style="--cols: 1fr 96px 76px 64px">
                      <span>作业</span><span class="cv-r">截止</span><span class="cv-r">状态</span
                      ><span class="cv-r">操作</span>
                    </div>
                    <div
                      v-for="assignment in courseAssignments"
                      :key="assignment.id"
                      class="cv-tr"
                      style="--cols: 1fr 96px 76px 64px"
                    >
                      <span class="cv-ellip" :title="assignment.title">{{ assignment.title }}</span>
                      <span class="cv-r cv-num cv-muted">{{ shortDate(assignment.end) }}</span>
                      <span class="cv-r">
                        <span class="cv-badge" :class="statusOf(assignment).tone">
                          {{ statusOf(assignment).text }}
                        </span>
                      </span>
                      <span class="cv-r">
                        <button
                          v-if="!assignment.closed"
                          type="button"
                          class="cv-link"
                          @click="openSubmit(assignment)"
                        >
                          {{ (assignment.submitCount ?? 0) > 0 ? '重新提交' : '提交' }}
                        </button>
                        <span v-else class="cv-muted">—</span>
                      </span>
                    </div>
                  </div>
                </template>

                <div class="wb-sect">
                  <h2>文件</h2>
                  <span>{{ visibleFiles.length }}</span>
                  <span class="wb-hint">文件夹在本地磁盘上，可在访达里直接整理</span>
                </div>

                <div v-if="childFolders.length || visibleFiles.length" class="cv-panel cv-tbl">
                  <div class="cv-thead" style="--cols: 1fr 76px 92px 140px">
                    <span>名称</span><span class="cv-r">大小</span><span class="cv-r">修改时间</span
                    ><span>绑定作业</span>
                  </div>
                  <button
                    v-for="folder in childFolders"
                    :key="folder.path"
                    type="button"
                    class="cv-tr"
                    style="--cols: 1fr 76px 92px 140px"
                    @click="select(folder)"
                  >
                    <span class="c">
                      <Folder :size="13" class="wb-fi" />
                      <span class="cv-ellip">{{ folder.name }}</span>
                    </span>
                    <span class="cv-r cv-muted">{{ folderTotal(folder) }} 个文件</span>
                    <span class="cv-r cv-muted">文件夹</span>
                    <span class="cv-muted">—</span>
                  </button>
                  <button
                    v-for="file in visibleFiles"
                    :key="file.path"
                    type="button"
                    class="cv-tr"
                    style="--cols: 1fr 76px 92px 140px"
                    :class="{ sel: pickedFile?.path === file.path }"
                    @click="pickedFile = file"
                  >
                    <span class="c">
                      <i class="cv-ft">{{ fileExt(file.name) }}</i>
                      <span class="cv-ellip">{{ file.name }}</span>
                    </span>
                    <span class="cv-r cv-num cv-muted">{{ sizeLabel(file.size) }}</span>
                    <span class="cv-r cv-num cv-muted">{{ timeLabel(file.modified) || '—' }}</span>
                    <span class="cv-muted cv-ellip">{{ file.assignment || '未绑定' }}</span>
                  </button>
                </div>
                <div v-else class="cv-empty">
                  <FileText :size="20" /><b>这个文件夹还是空的</b>
                  <span>点「添加文件」把资料放进来，或在访达里直接拖入。</span>
                </div>
              </div>
            </template>
          </template>
          <div v-else class="cv-empty">
            <Folder :size="20" /><b>还没有学期文件夹</b>
            <span>先登录校园账号，然后点右上角「同步」同步本学期课程。</span>
          </div>
        </div>

        <div class="cv-panel cv-detail">
          <template v-if="pickedFile">
            <div class="wb-prev" aria-hidden="true">
              <div class="wb-pg">
                <b>{{ fileExt(pickedFile.name) }}</b>
                <i></i><i></i><i></i><i></i><i></i>
              </div>
            </div>
            <div class="cv-type">
              文件
              <span v-if="pickedFile.assignment" class="cv-badge accent">已绑定</span>
            </div>
            <h3>{{ pickedFile.name }}</h3>
            <dl class="cv-kv">
              <dt>大小</dt>
              <dd class="cv-num">{{ sizeLabel(pickedFile.size) }}</dd>
              <dt>修改时间</dt>
              <dd class="cv-num">{{ timeLabel(pickedFile.modified) || '—' }}</dd>
              <dt>位置</dt>
              <dd class="cv-ellip">~/…/{{ selectedFolder?.name || '—' }}</dd>
              <dt>绑定作业</dt>
              <dd>{{ pickedFile.assignment || '未绑定' }}</dd>
              <template v-if="pickedFile.note">
                <dt>备注</dt>
                <dd>{{ pickedFile.note }}</dd>
              </template>
            </dl>
            <div class="cv-blk">
              <p>文件保存在本地磁盘上，工作台只做索引；打开、显示与删除都会直接作用于这个文件。</p>
            </div>
            <div class="cv-foot">
              <button type="button" class="cv-btn sm primary" @click="openFile(pickedFile)">
                打开
              </button>
              <button type="button" class="cv-btn sm" @click="reveal(pickedFile.path)">
                在访达中显示
              </button>
              <button type="button" class="cv-btn sm" @click="openBind(pickedFile)">
                {{ pickedFile.assignment ? '改绑定' : '绑定作业' }}
              </button>
              <button
                v-if="boundAssignment && !boundAssignment.closed"
                type="button"
                class="cv-btn sm"
                @click="openSubmit(boundAssignment)"
              >
                提交到«{{ boundAssignment.title }}»
              </button>
              <button type="button" class="cv-btn sm danger" @click="removeFile(pickedFile)">
                删除
              </button>
            </div>
          </template>
          <template v-else-if="selectedFolder">
            <div class="cv-type">{{ selectedKind }}</div>
            <h3>{{ selectedFolder.name }}</h3>
            <dl class="cv-kv">
              <dt>文件</dt>
              <dd class="cv-num">{{ selectedFolder.files }}</dd>
              <dt>子文件夹</dt>
              <dd class="cv-num">{{ selectedFolder.folders }}</dd>
              <template v-if="selectedMeta">
                <dt>课程编号</dt>
                <dd>
                  {{ displayCourseCode(selectedMeta.subjectCode || selectedMeta.code) || '未知' }}
                </dd>
                <dt>学期</dt>
                <dd>{{ selectedCourseTerm || '—' }}</dd>
                <dt>作业</dt>
                <dd class="cv-num">{{ selectedCourse?.assignments.length ?? 0 }}</dd>
              </template>
            </dl>
            <div class="cv-blk">
              <p>
                文件夹在本地磁盘上，改名或移动可以直接在访达里完成；工作台按学期和课程整理入口。
              </p>
            </div>
            <div class="cv-foot">
              <button type="button" class="cv-btn sm primary" @click="addFiles">添加文件</button>
              <button type="button" class="cv-btn sm" @click="openFolderModal">新建文件夹</button>
            </div>
          </template>
          <div v-else class="cv-placeholder">选择一个文件夹查看内容。</div>
        </div>
      </div>
    </div>

    <div v-if="folderModal" class="wb-overlay" @click.self="folderModal = false">
      <div class="wb-dialog" role="dialog" aria-modal="true" aria-label="新建文件夹">
        <h3>新建文件夹</h3>
        <p class="muted">会在「{{ selectedFolder?.name }}」下创建。</p>
        <input
          v-model="folderName"
          class="wb-input"
          placeholder="文件夹名称"
          @keyup.enter="createFolder"
        />
        <div class="dialog-actions">
          <button type="button" class="cv-btn" @click="folderModal = false">取消</button>
          <button
            type="button"
            class="cv-btn primary"
            :disabled="busy || !folderName.trim()"
            @click="createFolder"
          >
            创建
          </button>
        </div>
      </div>
    </div>

    <div v-if="bindOpen && bindFile" class="wb-overlay" @click.self="bindOpen = false">
      <div class="wb-dialog" role="dialog" aria-modal="true" aria-label="绑定作业">
        <h3>绑定作业</h3>
        <p class="muted">{{ bindFile.name }}</p>
        <label class="field">
          <span>作业</span>
          <select v-model="bindAssignment" class="wb-input">
            <option value="">不绑定</option>
            <option v-for="item in courseAssignments" :key="item.id" :value="item.title">
              {{ item.title }}
            </option>
          </select>
        </label>
        <label class="field">
          <span>备注</span>
          <input v-model="bindNote" class="wb-input" placeholder="可选" />
        </label>
        <p class="muted small">绑定作业后，提交时可直接选用这个文件。</p>
        <div class="dialog-actions">
          <button type="button" class="cv-btn" @click="bindOpen = false">取消</button>
          <button type="button" class="cv-btn primary" @click="saveBinding">保存</button>
        </div>
      </div>
    </div>

    <div v-if="submitOpen" class="wb-overlay" @click.self="submitOpen = false">
      <div class="wb-dialog wide" role="dialog" aria-modal="true" aria-label="提交作业">
        <div class="dialog-head">
          <h3>提交作业</h3>
          <button type="button" class="icon" aria-label="关闭" @click="submitOpen = false">
            <X :size="14" />
          </button>
        </div>
        <p class="muted">{{ submitAssignment?.title }}</p>

        <div class="submit-files">
          <label v-for="file in submitFiles" :key="file.path" class="check">
            <input
              type="checkbox"
              :checked="submitChecked.has(file.path)"
              @change="toggleSubmitFile(file.path)"
            />
            <span class="ellipsis">{{ file.name }}</span>
            <span class="muted">{{ sizeLabel(file.size) }}</span>
          </label>
          <p v-if="!submitFiles.length" class="muted">这个课程文件夹里还没有文件。</p>
        </div>

        <textarea
          v-model="submitComment"
          class="wb-input"
          rows="3"
          placeholder="提交说明（可选）"
          @input="preview = null"
        ></textarea>

        <InlineAlert v-if="submitError" tone="error" title="提交没有完成">
          {{ submitError }}
        </InlineAlert>

        <div v-if="preview" class="preview">
          <div class="preview-row">
            <span>作业</span><b>{{ preview.title }}</b>
          </div>
          <div class="preview-row">
            <span>截止</span><b>{{ preview.end || '不限' }}</b>
          </div>
          <div class="preview-row">
            <span>状态</span>
            <b>{{ preview.canSubmit ? '可提交' : '已截止 · 不能提交' }}</b>
          </div>
          <div class="preview-row">
            <span>文件</span>
            <b v-if="preview.files.length"
              >{{ preview.files.length }} 个 · {{ sizeLabel(preview.totalSize) }}</b
            >
            <b v-else>不提交文件</b>
          </div>
          <p class="muted small">{{ preview.note }}</p>
        </div>

        <div class="dialog-actions">
          <button type="button" class="cv-btn" @click="submitOpen = false">取消</button>
          <button
            type="button"
            class="cv-btn"
            :disabled="previewing || submitting"
            @click="previewSubmit"
          >
            预览提交
          </button>
          <button
            type="button"
            class="cv-btn primary"
            :disabled="!preview || !preview.canSubmit || submitting"
            @click="confirmSubmit"
          >
            {{ submitting ? '正在提交…' : '确认提交' }}
          </button>
        </div>
      </div>
    </div>
  </section>
</template>
<style scoped>
/* Layout comes from the campus v2 system (.campus-v2 + .cv-*); only the
   workbench-specific tree, section rows and dialogs live here. */
.workbench {
  min-height: 0;
}
.wb-term {
  margin-left: 8px;
  color: var(--text-tertiary);
  font-size: 12px;
  font-weight: 400;
}
.wb-cols {
  min-height: 0;
}

/* ---- folder tree (left column) ---- */
.wb-tree {
  min-height: 0;
}
.tree-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  height: 26px;
  padding: 0 2px 0 8px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-weight: 600;
  flex-shrink: 0;
}
.icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 22px;
  height: 22px;
  border: 0;
  background: none;
  border-radius: 5px;
  color: var(--text-tertiary);
  cursor: pointer;
}
.icon:hover {
  background: var(--surface-hover);
}
.tree-body {
  flex: 1;
  min-height: 0;
  overflow: auto;
  margin-top: 2px;
}
.tree-node {
  display: flex;
  align-items: center;
  width: 100%;
  height: 27px;
  padding-right: 6px;
  border-radius: 6px;
  font-size: 12.5px;
  color: var(--text-secondary);
}
.tree-node:hover {
  background: var(--surface-hover);
}
.tree-node.active {
  background: var(--accent-soft, #eaf1fd);
  color: var(--accent-text, #1a5ec6);
  font-weight: 500;
}
/* An opaque highlight reads as a blue slab over the wallpaper. */
.has-wallpaper .tree-node.active {
  background: var(--accent-tint);
}
.tree-node .tw {
  width: 18px;
  height: 100%;
  flex-shrink: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  border: 0;
  border-radius: 4px;
  background: none;
  color: var(--text-tertiary);
  cursor: pointer;
}
.tree-node .tw:hover {
  background: rgba(15, 23, 42, 0.06);
}
.tree-main {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  gap: 5px;
  height: 100%;
  padding: 0;
  border: 0;
  background: none;
  font: inherit;
  color: inherit;
  text-align: left;
  cursor: pointer;
}
.tree-node .fi {
  flex-shrink: 0;
  color: #9aa1aa;
}
.tree-node.active .fi {
  color: var(--accent-text, #1a5ec6);
}
.tree-node .nm {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tree-node .real {
  font-size: 10.5px;
  color: var(--text-tertiary);
  font-weight: 400;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.tree-node .ct {
  margin-left: auto;
  font-size: 10.5px;
  color: var(--text-tertiary);
  font-variant-numeric: tabular-nums;
}
.wb-tree-empty {
  margin: 0;
  padding: 14px 10px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.tree-foot {
  flex-shrink: 0;
  border: 0;
  border-top: 1px solid var(--divider, rgba(15, 23, 42, 0.05));
  background: none;
  padding: 8px 8px 2px;
  font: inherit;
  font-size: 11.5px;
  color: var(--accent-text, #1a5ec6);
  text-align: left;
  cursor: pointer;
}

/* ---- center column ---- */
.wb-crumb {
  display: flex;
  align-items: center;
  gap: 5px;
  min-height: 14px;
  flex-shrink: 0;
  font-size: 11.5px;
  color: var(--text-tertiary);
}
.crumb {
  border: 0;
  background: none;
  padding: 0;
  font: inherit;
  color: var(--accent-text, #1a5ec6);
  cursor: pointer;
}
.wb-sect {
  display: flex;
  align-items: baseline;
  gap: 8px;
  padding: 2px 2px 0;
  flex-shrink: 0;
}
.wb-sect h2 {
  margin: 0;
  color: var(--text);
  font-size: 12.5px;
  font-weight: 600;
}
.wb-sect > span {
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.wb-sect .wb-hint {
  margin-left: auto;
  font-variant-numeric: normal;
}
.wb-fi {
  flex-shrink: 0;
  color: var(--text-tertiary);
}

/* ---- inspector file preview ---- */
.wb-prev {
  display: flex;
  align-items: center;
  justify-content: center;
  height: 96px;
  border-radius: 8px;
  background: var(--surface-sunken);
}
.wb-pg {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 56px;
  height: 74px;
  padding: 7px 6px;
  border-radius: 3px;
  background: #fff;
  box-shadow: var(--shadow-sm);
}
.wb-pg b {
  color: var(--text-tertiary);
  font-size: 8px;
  font-weight: 600;
  letter-spacing: 0.04em;
}
.wb-pg i {
  display: block;
  height: 3px;
  border-radius: 2px;
  background: #e3e6ea;
}
.wb-pg i:nth-child(2) {
  width: 70%;
}
.wb-pg i:nth-child(5) {
  width: 60%;
}

/* ---- dialogs ---- */
.wb-overlay {
  position: fixed;
  inset: 0;
  background: rgba(15, 23, 42, 0.28);
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 24px;
  z-index: 40;
}
.wb-dialog {
  width: min(420px, 100%);
  background: var(--surface, #fff);
  border-radius: 12px;
  padding: 18px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  box-shadow: 0 18px 48px rgba(15, 23, 42, 0.22);
}
.wb-dialog.wide {
  width: min(560px, 100%);
}
.wb-dialog h3 {
  margin: 0;
  font-size: 15px;
  font-weight: 600;
}
.dialog-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
}
.dialog-actions {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  margin-top: 4px;
}
.wb-input {
  width: 100%;
  padding: 7px 9px;
  border: 1px solid var(--control-border);
  border-radius: 6px;
  font: inherit;
  font-size: 12.5px;
  background: var(--control-bg);
  color: var(--text);
  box-sizing: border-box;
}
.field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 11.5px;
  color: var(--text-secondary);
}
.muted {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.small {
  font-size: 11px;
}
.ellipsis {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.submit-files {
  max-height: 180px;
  overflow: auto;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px 8px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.check {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}
.check .muted {
  margin-left: auto;
}
.preview {
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  background: var(--control-bg-subtle);
}
.preview-row {
  display: flex;
  justify-content: space-between;
  font-size: 12px;
}
.preview-row span {
  color: var(--text-tertiary);
}
.spinning {
  animation: spin 0.9s linear infinite;
}
@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}
</style>
