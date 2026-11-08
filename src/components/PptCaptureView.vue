<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, ref, watch } from 'vue';
import {
  ChevronLeft,
  ChevronRight,
  Download,
  Presentation,
  RotateCcw,
  Scissors,
  SquareDashed,
  Trash2,
  X,
} from 'lucide-vue-next';
import { save } from '@tauri-apps/plugin-dialog';
import SectionHeader from './ui/SectionHeader.vue';
import EmptyState from './ui/EmptyState.vue';
import InlineAlert from './ui/InlineAlert.vue';
import UiButton from './ui/UiButton.vue';
import MenuSelect, { type MenuOption } from './ui/MenuSelect.vue';
import { bridge } from '../bridge';
import { applyCached, cacheName } from '../cache';
import { siteQuery } from '../campusSession';
import { fileSize } from '../domain';
import { buildNotesPdf, buildPdf, buildZip } from '../capture/export';
import {
  ANALYSIS_HEIGHT,
  ANALYSIS_WIDTH,
  MAX_FRAMES,
  isNearDuplicate,
  isSimilar,
  signature as signatureOf,
  type Signature,
} from '../capture/frames';
import { exportBaseName, slideName } from '../capture/naming';
import {
  SCAN_MODES,
  mergeSuspects,
  scan,
  settingsFor,
  type ScanCapture,
  type ScanMode,
  type ScanSample,
  type SuspectRange,
} from '../capture/scanner';
import { SeekTimeout, frameReady, seekTo, waitEvent } from '../capture/seek';
import { defaultVideoIndex } from '../capture/source';
import {
  appendPage,
  clearSession,
  deletePages,
  loadSession,
  saveSession,
  updateOrder,
  type StoredSession,
} from '../capture/store';
import type { CaptureTarget } from '../capture/target';
import type { LmsActivities, LmsActivity, LmsCourse, LmsCourses, LmsReplayVideo } from '../types';

/**
 * 回放截取 (built-in plugin `cn.xjtu.ppt-capture`), entered from a lesson in
 * 思源学堂 → 课程回放. It seeks through one replay, keeps slides that differ at
 * the chosen mode, records unstable ranges for review, lets the user curate
 * the pages, then exports a PDF, an A4 note handout or a JPEG archive.
 *
 * The presigned video address comes from `lms_replay_videos`, expires, and is
 * never cached, logged or shown as text. Course content stays on this machine
 * and is written only where the user points the save dialog.
 */
interface Frame {
  id: number;
  /** Row in IndexedDB; null when persistence is unavailable. */
  pageId: number | null;
  url: string;
  blob: Blob;
  width: number;
  height: number;
  time: number;
  signature: Signature;
  selected: boolean;
}
interface Roi {
  x: number;
  y: number;
  w: number;
  h: number;
}

/** Set by 思源学堂 → 课程回放 → 课时详情; previews that lesson on mount. */
const props = defineProps<{
  target?: CaptureTarget | null;
  /** Student identity; a change means another account took over the page. */
  scope?: string;
}>();

const courses = ref<LmsCourse[]>([]);
const lessons = ref<LmsActivity[]>([]);
const courseId = ref('');
const activityId = ref('');
const videos = ref<LmsReplayVideo[]>([]);
const videoIndex = ref(0);
const loading = ref(true);
const lessonsLoading = ref(false);
const loadingVideo = ref(false);
const scanning = ref(false);
const exporting = ref(false);
const savingPage = ref(false);
/** A second click is needed before an existing run is discarded. */
const pendingRestart = ref(false);
const error = ref('');
const notice = ref('');
const loadedAt = ref<number | null>(null);
const duration = ref(0);
const playKey = ref(0);
const mode = ref<ScanMode>('balanced');
const threshold = ref('1.2');
const dedupe = ref(true);
const frames = ref<Frame[]>([]);
const suspects = ref<SuspectRange[]>([]);
const progress = ref({ done: 0, total: 0 });
const roiMode = ref(false);
const roi = ref<Roi | null>(null);
const dragging = ref<Roi | null>(null);
const playerBox = ref<HTMLElement | null>(null);
const videoEl = ref<HTMLVideoElement | null>(null);
const preview = ref<number | null>(null);
const undo = ref<Frame[] | null>(null);
const dragIndex = ref<number | null>(null);
/** Seconds already scanned; a restored lesson can continue from here. */
const resumeAt = ref(0);
/** Settings the stored session was scanned with; changing any starts over. */
const restoredSettings = ref('');
let cancelRequested = false;
let frameId = 0;
let dragStart: { x: number; y: number } | null = null;
let activeVideo: HTMLVideoElement | null = null;
let recoveries = 0;
let canvases: {
  canvas: HTMLCanvasElement;
  context: CanvasRenderingContext2D;
  analysis: HTMLCanvasElement;
  analysisContext: CanvasRenderingContext2D;
} | null = null;

const current = computed(() => videos.value[videoIndex.value] ?? null);
const course = computed(() => courses.value.find((item) => String(item.id) === courseId.value));
const lesson = computed(() => lessons.value.find((item) => String(item.id) === activityId.value));
const selectedCount = computed(() => frames.value.filter((frame) => frame.selected).length);
const subtitle = computed(() => {
  if (!course.value) return '';
  if (!lesson.value) return course.value.name;
  return `${course.value.name} · ${lesson.value.title}`;
});
const courseOptions = computed<MenuOption[]>(() =>
  [...courses.value]
    .sort((a, b) => b.year.localeCompare(a.year) || a.name.localeCompare(b.name, 'zh-Hans-CN'))
    .map((item) => ({ value: String(item.id), label: item.name })),
);
const lessonOptions = computed<MenuOption[]>(() =>
  lessons.value.map((item) => ({
    value: String(item.id),
    label: `${item.title} · ${item.end.slice(5, 10)}`,
  })),
);
const modeOptions: MenuOption[] = (Object.keys(SCAN_MODES) as ScanMode[]).map((value) => ({
  value,
  label: `${SCAN_MODES[value].title} · ${SCAN_MODES[value].step} 秒`,
}));
const thresholdOptions: MenuOption[] = ['0.5', '0.8', '1.2', '2', '3'].map((value) => ({
  value,
  label: `${value}%`,
}));
const videoOptions = computed<MenuOption[]>(() =>
  videos.value.map((video, index) => ({
    value: String(index),
    label: `${video.label} · ${fileSize(video.size)}${video.bestAudio ? ' · 最佳音轨' : ''}`,
  })),
);
const roiBox = computed(() => dragging.value ?? roi.value);
const roiStyle = computed(() => {
  const box = roiBox.value;
  if (!box) return {};
  return {
    left: `${box.x * 100}%`,
    top: `${box.y * 100}%`,
    width: `${box.w * 100}%`,
    height: `${box.h * 100}%`,
  };
});
const progressText = computed(() => {
  if (!scanning.value && !progress.value.total) return '';
  const percent = progress.value.total
    ? Math.round((progress.value.done / progress.value.total) * 100)
    : 0;
  return `${progress.value.done} / ${progress.value.total} · ${percent}% · 保留 ${frames.value.length} 页`;
});
const previewFrame = computed(() =>
  preview.value == null ? null : (frames.value[preview.value] ?? null),
);
const settingsKey = computed(() =>
  JSON.stringify([mode.value, threshold.value, dedupe.value, roi.value]),
);
const resumable = computed(
  () =>
    resumeAt.value > 0 &&
    resumeAt.value < duration.value - 0.05 &&
    settingsKey.value === restoredSettings.value,
);

function isLesson(item: LmsActivity): boolean {
  return item.type === 'lesson';
}

function clock(seconds: number): string {
  const total = Math.max(0, Math.floor(seconds));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const rest = String(total % 60).padStart(2, '0');
  return hours ? `${hours}:${String(minutes).padStart(2, '0')}:${rest}` : `${minutes}:${rest}`;
}

function base64(bytes: Uint8Array): string {
  const chunks: string[] = [];
  const size = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += size) {
    chunks.push(String.fromCharCode(...bytes.subarray(offset, offset + size)));
  }
  return btoa(chunks.join(''));
}

function discardUndo() {
  if (!undo.value) return;
  for (const frame of undo.value) {
    if (!frames.value.some((item) => item.id === frame.id)) URL.revokeObjectURL(frame.url);
  }
  undo.value = null;
}

function releaseFrames() {
  discardUndo();
  for (const frame of frames.value) URL.revokeObjectURL(frame.url);
  frames.value = [];
  preview.value = null;
}

function beginUndo() {
  discardUndo();
  undo.value = [...frames.value];
}

function forgetPages(removed: Frame[]) {
  const ids = removed
    .map((frame) => frame.pageId)
    .filter((id): id is number => typeof id === 'number');
  if (!ids.length || !activityId.value) return;
  deletePages(Number(activityId.value), ids).catch((e) =>
    console.warn('[capture] delete pages', e),
  );
}

async function undoDelete() {
  if (!undo.value) return;
  const restored = undo.value;
  const present = new Set(frames.value.map((frame) => frame.id));
  undo.value = null;
  for (const [index, frame] of restored.entries()) {
    if (present.has(frame.id)) continue;
    try {
      frame.pageId = await appendPage(
        Number(activityId.value),
        index,
        {
          time: frame.time,
          width: frame.width,
          height: frame.height,
          signature: frame.signature,
        },
        frame.blob,
      );
    } catch (e) {
      console.warn('[capture] restore page', e);
    }
  }
  frames.value = restored;
  await updateOrder(
    Number(activityId.value),
    restored
      .map((frame, index) => ({ id: frame.pageId ?? 0, order: index }))
      .filter((entry) => entry.id > 0),
  );
  notice.value = `已恢复 ${restored.length - present.size} 页。`;
}

function addFrame(capture: ScanCapture, pageId: number | null = null): Frame {
  if (frames.value.length >= MAX_FRAMES) {
    throw new Error(`已达到 ${MAX_FRAMES} 页上限，请先导出或删除一部分再继续。`);
  }
  frameId += 1;
  const frame: Frame = {
    id: frameId,
    pageId,
    url: URL.createObjectURL(capture.blob),
    blob: capture.blob,
    width: capture.width,
    height: capture.height,
    time: capture.time,
    signature: capture.signature,
    selected: false,
  };
  frames.value.push(frame);
  return frame;
}

/** Add a page and persist it; IndexedDB failure only loses resume support. */
async function addCaptured(capture: ScanCapture): Promise<Frame> {
  if (frames.value.length >= MAX_FRAMES) {
    throw new Error(`已达到 ${MAX_FRAMES} 页上限，请先导出或删除一部分再继续。`);
  }
  let pageId: number | null = null;
  try {
    pageId = await appendPage(
      Number(activityId.value),
      frames.value.length,
      {
        time: capture.time,
        width: capture.width,
        height: capture.height,
        signature: capture.signature,
      },
      capture.blob,
    );
  } catch (e) {
    console.warn('[capture] local store', e);
  }
  return addFrame(capture, pageId);
}

function sessionRecord(overrides: Partial<StoredSession> = {}): StoredSession {
  return {
    id: String(activityId.value),
    activity: Number(activityId.value) || 0,
    course: course.value?.name ?? '',
    title: lesson.value?.title ?? '',
    start: lesson.value?.start ?? '',
    mode: mode.value,
    threshold: Number(threshold.value),
    dedupe: dedupe.value,
    roi: roi.value,
    duration: duration.value,
    checkpoint: resumeAt.value,
    total: Math.ceil(duration.value / SCAN_MODES[mode.value].step),
    suspects: suspects.value,
    updated: Date.now(),
    ...overrides,
  };
}

async function persistSession(overrides: Partial<StoredSession> = {}) {
  const activity = Number(activityId.value);
  if (!activity) return;
  try {
    await saveSession(activity, sessionRecord(overrides));
  } catch (e) {
    console.warn('[capture] save session', e);
  }
}

async function restoreSession() {
  const activity = Number(activityId.value);
  if (!activity) return;
  resumeAt.value = 0;
  restoredSettings.value = '';
  let stored;
  try {
    stored = await loadSession(activity);
  } catch (e) {
    console.warn('[capture] load session', e);
    return;
  }
  // Lesson switched while reading; the restored pages belong elsewhere.
  if (activity !== Number(activityId.value)) return;
  if (!stored) return;
  const { session } = stored;
  mode.value = session.mode;
  threshold.value = String(session.threshold);
  dedupe.value = session.dedupe;
  roi.value = session.roi;
  suspects.value = session.suspects ?? [];
  duration.value = session.duration || duration.value;
  resumeAt.value = session.checkpoint ?? 0;
  restoredSettings.value = JSON.stringify([
    session.mode,
    String(session.threshold),
    session.dedupe,
    session.roi,
  ]);
  if (!stored.pages.length) return;
  releaseFrames();
  for (const { page, blob } of stored.pages) {
    frameId += 1;
    frames.value.push({
      id: frameId,
      pageId: page.id,
      url: URL.createObjectURL(blob),
      blob,
      width: page.width,
      height: page.height,
      time: page.time,
      signature: page.signature,
      selected: false,
    });
  }
  notice.value = `已恢复上次的 ${stored.pages.length} 页${
    resumable.value ? `，可从 ${clock(resumeAt.value)} 继续截取` : ''
  }。`;
}

async function loadCourses(force = false) {
  loading.value = true;
  error.value = '';
  try {
    await applyCached<LmsCourses>('lmsCourses', undefined, (value) => {
      courses.value = value.courses;
    });
    const result = await siteQuery<LmsCourses>(
      { type: 'lmsCourses' },
      'lms',
      () => bridge.refreshLmsSession(),
      'lmsCourses',
      force,
    );
    courses.value = result.courses;
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[capture] lms courses', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    loading.value = false;
  }
}

async function loadLessons(force = false) {
  const id = Number(courseId.value);
  if (!id) {
    lessons.value = [];
    activityId.value = '';
    videos.value = [];
    return;
  }
  lessonsLoading.value = true;
  error.value = '';
  const params = { course: id };
  try {
    await applyCached<LmsActivities>('lmsActivities', params, (value) => {
      lessons.value = value.activities.filter(isLesson);
    });
    const result = await siteQuery<LmsActivities>(
      { type: 'lmsActivities', course: id },
      'lms',
      () => bridge.refreshLmsSession(),
      cacheName('lmsActivities', params),
      force,
    );
    lessons.value = result.activities.filter(isLesson);
    if (!lessons.value.some((item) => String(item.id) === activityId.value)) {
      activityId.value = '';
      videos.value = [];
    }
    loadedAt.value = Date.now();
  } catch (e) {
    console.error('[capture] lms activities', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    lessonsLoading.value = false;
  }
}

async function loadVideo() {
  if (!activityId.value) {
    videos.value = [];
    return;
  }
  loadingVideo.value = true;
  error.value = '';
  notice.value = '';
  try {
    videos.value = (await bridge.lmsReplayVideos(Number(activityId.value))).videos;
    videoIndex.value = defaultVideoIndex(videos.value);
    playKey.value += 1;
    duration.value = 0;
    if (!videos.value.length) error.value = '这节课没有可播放的回放视频';
  } catch (e) {
    console.error('[capture] replay videos', e);
    error.value = e instanceof Error ? e.message : String(e);
    videos.value = [];
  } finally {
    loadingVideo.value = false;
  }
}

function onLoadedMetadata() {
  const video = videoEl.value;
  if (!video) return;
  duration.value = Number.isFinite(video.duration) ? video.duration : 0;
  video.currentTime = 0;
  activeVideo = video;
}

function onVideoError() {
  const code = videoEl.value?.error?.code;
  error.value =
    code === 4
      ? '浏览器拒绝了这段视频，回放地址可能已过期，请重新解析'
      : '视频加载失败，请重新解析回放地址';
}

function changeVideo(value: string) {
  videoIndex.value = Number(value);
  playKey.value += 1;
  duration.value = 0;
  activeVideo = null;
}

function point(event: PointerEvent): { x: number; y: number } {
  const rect = playerBox.value?.getBoundingClientRect();
  if (!rect) return { x: 0, y: 0 };
  const clamp = (value: number) => Math.min(1, Math.max(0, value));
  return {
    x: clamp((event.clientX - rect.left) / rect.width),
    y: clamp((event.clientY - rect.top) / rect.height),
  };
}

function startRoi(event: PointerEvent) {
  if (!roiMode.value || scanning.value) return;
  event.preventDefault();
  (event.currentTarget as HTMLElement).setPointerCapture(event.pointerId);
  dragStart = point(event);
  dragging.value = { x: dragStart.x, y: dragStart.y, w: 0, h: 0 };
}

function moveRoi(event: PointerEvent) {
  if (!dragStart) return;
  const now = point(event);
  dragging.value = {
    x: Math.min(dragStart.x, now.x),
    y: Math.min(dragStart.y, now.y),
    w: Math.abs(now.x - dragStart.x),
    h: Math.abs(now.y - dragStart.y),
  };
}

function endRoi(event: PointerEvent) {
  if (!dragStart) return;
  (event.currentTarget as HTMLElement)?.releasePointerCapture?.(event.pointerId);
  const box = dragging.value;
  dragStart = null;
  dragging.value = null;
  roiMode.value = false;
  if (box && box.w > 0.02 && box.h > 0.02) roi.value = box;
}

function clearRoi() {
  roi.value = null;
  roiMode.value = false;
}

function workCanvases() {
  if (canvases) return canvases;
  const canvas = document.createElement('canvas');
  const analysis = document.createElement('canvas');
  analysis.width = ANALYSIS_WIDTH;
  analysis.height = ANALYSIS_HEIGHT;
  const context = canvas.getContext('2d', { willReadFrequently: true });
  const analysisContext = analysis.getContext('2d', { willReadFrequently: true });
  if (!context || !analysisContext) throw new Error('无法创建画布');
  canvases = { canvas, context, analysis, analysisContext };
  return canvases;
}

function crop(video: HTMLVideoElement): { x: number; y: number; w: number; h: number } {
  const width = video.videoWidth || 1280;
  const height = video.videoHeight || 720;
  if (!roi.value) return { x: 0, y: 0, w: width, h: height };
  return {
    x: Math.round(roi.value.x * width),
    y: Math.round(roi.value.y * height),
    w: Math.max(2, Math.round(roi.value.w * width)),
    h: Math.max(2, Math.round(roi.value.h * height)),
  };
}

function draw(video: HTMLVideoElement): CanvasRenderingContext2D {
  const { canvas, context } = workCanvases();
  const box = crop(video);
  canvas.width = box.w;
  canvas.height = box.h;
  context.drawImage(video, box.x, box.y, box.w, box.h, 0, 0, box.w, box.h);
  return context;
}

function toBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error('无法生成截图'))),
      'image/jpeg',
      0.92,
    );
  });
}

async function sampleAt(video: HTMLVideoElement, time: number): Promise<ScanSample> {
  const reached = await seekTo(video, time);
  await frameReady(video);
  draw(video);
  const { canvas, analysisContext } = workCanvases();
  analysisContext.drawImage(canvas, 0, 0, ANALYSIS_WIDTH, ANALYSIS_HEIGHT);
  const pixels = analysisContext.getImageData(0, 0, ANALYSIS_WIDTH, ANALYSIS_HEIGHT).data;
  return { time: reached, signature: signatureOf(pixels) };
}

async function captureAt(video: HTMLVideoElement, time: number): Promise<ScanCapture> {
  const sample = await sampleAt(video, time);
  const { canvas } = workCanvases();
  const blob = await toBlob(canvas);
  return { ...sample, blob, width: canvas.width, height: canvas.height };
}

/** Re-resolve the presigned address after a stalled seek and remount the player. */
async function recoverVideo(): Promise<HTMLVideoElement> {
  if (recoveries >= 2) throw new Error('多次跳转超时，回放地址可能已过期，请重新解析。');
  recoveries += 1;
  notice.value = `视频跳转超时，正在重新解析回放地址（${recoveries}/2）…`;
  await loadVideo();
  await nextTick();
  const video = videoEl.value;
  if (!video) throw new Error('视频重新加载失败，请重新解析回放地址');
  if (video.readyState < 1) {
    await waitEvent(video, 'loadedmetadata', '视频信息读取失败', 30000);
  }
  activeVideo = video;
  return video;
}

async function guarded<T>(
  video: HTMLVideoElement,
  action: (element: HTMLVideoElement) => Promise<T>,
): Promise<T> {
  try {
    const value = await action(video);
    recoveries = 0;
    return value;
  } catch (e) {
    if (!(e instanceof SeekTimeout)) throw e;
    const nextVideo = await recoverVideo();
    const value = await action(nextVideo);
    recoveries = 0;
    return value;
  }
}

function stopScan() {
  cancelRequested = true;
}

function requestScan() {
  if (scanning.value) return;
  // Discarding a finished or incompatible run takes an explicit second click.
  if (!resumable.value && frames.value.length && !pendingRestart.value) {
    pendingRestart.value = true;
    return;
  }
  pendingRestart.value = false;
  void startScan(true);
}

async function startScan(continueRun = true) {
  if (scanning.value) return;
  pendingRestart.value = false;
  const video = videoEl.value;
  if (!video || !current.value) return;
  if (!duration.value && video.readyState >= 1 && Number.isFinite(video.duration)) {
    duration.value = video.duration;
  }
  error.value = '';
  notice.value = '';
  const startTime =
    continueRun && resumable.value
      ? Math.min(resumeAt.value, Math.max(0, duration.value - 0.05))
      : 0;
  if (!startTime) {
    releaseFrames();
    suspects.value = [];
    resumeAt.value = 0;
    // A failed clear only costs resume support; the run itself can go on.
    await clearSession(Number(activityId.value)).catch((e) =>
      console.warn('[capture] clear session', e),
    );
  }
  progress.value = { done: 0, total: 0 };
  scanning.value = true;
  cancelRequested = false;
  recoveries = 0;
  activeVideo = video;
  try {
    if (!duration.value) {
      if (video.readyState >= 1) duration.value = video.duration;
      else await waitEvent(video, 'loadedmetadata', '视频信息读取失败');
    }
    await persistSession({ checkpoint: startTime, duration: duration.value });
    const known = frames.value.map((frame) => frame.signature);
    const result = await scan(
      duration.value,
      settingsFor(mode.value, Number(threshold.value), dedupe.value),
      {
        known,
        baseline: known.at(-1) ?? null,
        sample: (time) => guarded(activeVideo!, (element) => sampleAt(element, time)),
        capture: (time) => guarded(activeVideo!, (element) => captureAt(element, time)),
        save: async (capture) => {
          await addCaptured(capture);
        },
        suspect: (range) => {
          suspects.value = mergeSuspects([...suspects.value, range]);
          void persistSession({ suspects: suspects.value });
        },
        progress: (done, total) => {
          progress.value = { done, total };
        },
        checkpoint: async (done, total, time) => {
          resumeAt.value = time;
          await persistSession({ checkpoint: time, total, suspects: suspects.value });
        },
        cancelled: () => cancelRequested,
      },
      startTime,
    );
    resumeAt.value = result.cancelled ? resumeAt.value : duration.value;
    await persistSession({
      checkpoint: resumeAt.value,
      suspects: result.suspects,
      duration: duration.value,
    });
    const suffix = result.suspects.length
      ? `；${result.suspects.length} 段画面不稳定，可在下方回看补查`
      : '';
    notice.value = result.cancelled
      ? `已停止，保留 ${frames.value.length} 页${suffix}。`
      : `截取完成，保留 ${frames.value.length} 页${suffix}。`;
  } catch (e) {
    await persistSession({ checkpoint: resumeAt.value, suspects: suspects.value });
    console.error('[capture] frames', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    scanning.value = false;
  }
}

async function rescanSuspect(range: SuspectRange) {
  const video = activeVideo ?? videoEl.value;
  if (!video || scanning.value) return;
  scanning.value = true;
  cancelRequested = false;
  recoveries = 0;
  activeVideo = video;
  error.value = '';
  try {
    const step = 0.5;
    const limit = Number(threshold.value);
    let added = 0;
    for (let time = range.start + step; time < range.end - 0.02; time += step) {
      if (cancelRequested) break;
      const sample = await guarded(video, (element) => sampleAt(element, time));
      if (frames.value.some((frame) => isSimilar(frame.signature, sample.signature, limit)))
        continue;
      const confirm = await guarded(video, (element) =>
        sampleAt(element, Math.min(time + 0.25, range.end)),
      );
      const changed = isSimilar(sample.signature, confirm.signature, limit);
      if (!changed) continue;
      await addCaptured(await guarded(video, (element) => captureAt(element, time)));
      added += 1;
    }
    suspects.value = suspects.value.filter(
      (item) => item.start > range.end || item.end < range.start,
    );
    notice.value = added ? `已在这一段补回 ${added} 页。` : '这一段没有稳定的新画面。';
  } catch (e) {
    console.error('[capture] rescan', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    scanning.value = false;
  }
}

async function saveCurrent() {
  const video = activeVideo ?? videoEl.value;
  if (!video || savingPage.value || scanning.value) return;
  savingPage.value = true;
  error.value = '';
  try {
    const capture = await guarded(video, (element) => captureAt(element, element.currentTime));
    await addCaptured(capture);
    notice.value = `已保存 ${clock(capture.time)} 的画面。`;
  } catch (e) {
    console.error('[capture] manual page', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    savingPage.value = false;
  }
}

async function jumpTo(frame: Frame) {
  const video = videoEl.value;
  if (!video) return;
  video.pause();
  try {
    await seekTo(video, frame.time);
  } catch (e) {
    error.value = e instanceof Error ? e.message : String(e);
  }
}

function toggleAll(selected: boolean) {
  for (const frame of frames.value) frame.selected = selected;
}

function deleteSelected() {
  if (!selectedCount.value) return;
  const removed = frames.value.filter((frame) => frame.selected);
  beginUndo();
  frames.value = frames.value.filter((frame) => !frame.selected);
  forgetPages(removed);
  notice.value = `已删除 ${removed.length} 页，可撤销。`;
}

function removeFrame(frame: Frame) {
  beginUndo();
  frames.value = frames.value.filter((item) => item.id !== frame.id);
  forgetPages([frame]);
  if (preview.value != null && preview.value >= frames.value.length) preview.value = null;
  notice.value = '已删除 1 页，可撤销。';
}

function cleanSimilar() {
  if (frames.value.length < 2) return;
  const limit = Number(threshold.value);
  const kept = [...frames.value];
  const removed = new Set<number>();
  for (let index = kept.length - 1; index >= 1; index -= 1) {
    const previous = kept[index - 1];
    const currentFrame = kept[index];
    if (
      previous.width === currentFrame.width &&
      previous.height === currentFrame.height &&
      isNearDuplicate(previous.signature, currentFrame.signature, limit)
    ) {
      removed.add(previous.id);
    }
  }
  if (!removed.size) {
    notice.value = '没有找到相似的相邻页面。';
    return;
  }
  const gone = frames.value.filter((frame) => removed.has(frame.id));
  beginUndo();
  frames.value = frames.value.filter((frame) => !removed.has(frame.id));
  forgetPages(gone);
  notice.value = `已合并 ${removed.size} 个过渡页，保留每组最后一张，可撤销。`;
}

function onDragStart(index: number) {
  dragIndex.value = index;
}

function onDrop(index: number) {
  const from = dragIndex.value;
  dragIndex.value = null;
  if (from == null || from === index) return;
  const [moved] = frames.value.splice(from, 1);
  frames.value.splice(index, 0, moved);
  if (activityId.value) {
    void updateOrder(
      Number(activityId.value),
      frames.value
        .map((frame, order) => ({ id: frame.pageId ?? 0, order }))
        .filter((entry) => entry.id > 0),
    ).catch((e) => console.warn('[capture] reorder pages', e));
  }
  notice.value = '已调整页序。';
}

function openPreview(index: number) {
  preview.value = index;
}

function closePreview() {
  preview.value = null;
}

function movePreview(delta: number) {
  if (preview.value == null) return;
  const next = preview.value + delta;
  if (next >= 0 && next < frames.value.length) preview.value = next;
}

function removePreview() {
  if (preview.value == null) return;
  const frame = frames.value[preview.value];
  removeFrame(frame);
}

function onKeydown(event: KeyboardEvent) {
  if (preview.value == null) return;
  if (event.key === 'ArrowLeft') movePreview(-1);
  else if (event.key === 'ArrowRight') movePreview(1);
  else if (event.key === 'Delete' || event.key === 'Backspace') removePreview();
  else if (event.key === 'Escape') closePreview();
  else return;
  event.preventDefault();
}

function pageData(): Promise<{ data: Uint8Array; width: number; height: number }[]> {
  return Promise.all(
    frames.value.map(async (frame) => ({
      data: new Uint8Array(await frame.blob.arrayBuffer()),
      width: frame.width,
      height: frame.height,
    })),
  );
}

async function chooseTarget(name: string): Promise<string | null> {
  const target = await save({ defaultPath: await bridge.saveTarget(name) });
  return target ?? null;
}

function exportName(): string {
  const title = course.value?.name ?? lesson.value?.title ?? '回放截取';
  return exportBaseName(title, lesson.value?.start ?? '');
}

async function exportPdf(notes = false) {
  if (!frames.value.length) {
    notice.value = '先截取页面再导出。';
    return;
  }
  exporting.value = true;
  error.value = '';
  try {
    const pages = await pageData();
    const target = await chooseTarget(`${exportName()}${notes ? '-讲义' : ''}.pdf`);
    if (!target) return;
    const written = await bridge.saveExport(
      target,
      base64(notes ? buildNotesPdf(pages) : buildPdf(pages)),
    );
    notice.value = `已保存到 ${await bridge.displayPath(target)}（${fileSize(written)}）`;
  } catch (e) {
    console.error('[capture] export pdf', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    exporting.value = false;
  }
}

async function exportZip() {
  if (!frames.value.length) {
    notice.value = '先截取页面再导出。';
    return;
  }
  exporting.value = true;
  error.value = '';
  try {
    const entries = await Promise.all(
      frames.value.map(async (frame, index) => ({
        name: slideName(index, frame.time),
        data: new Uint8Array(await frame.blob.arrayBuffer()),
      })),
    );
    const target = await chooseTarget(`${exportName()}-slides.zip`);
    if (!target) return;
    const written = await bridge.saveExport(target, base64(buildZip(entries)));
    notice.value = `已保存到 ${await bridge.displayPath(target)}（${fileSize(written)}）`;
  } catch (e) {
    console.error('[capture] export zip', e);
    error.value = e instanceof Error ? e.message : String(e);
  } finally {
    exporting.value = false;
  }
}

watch(courseId, () => {
  void loadLessons();
});
watch(activityId, async () => {
  releaseFrames();
  suspects.value = [];
  roi.value = null;
  duration.value = 0;
  resumeAt.value = 0;
  restoredSettings.value = '';
  pendingRestart.value = false;
  activeVideo = null;
  try {
    await loadVideo();
  } finally {
    await restoreSession();
  }
});
watch(current, () => {
  duration.value = 0;
});
watch(
  () => props.scope,
  async () => {
    // The backend keys capture state by the active account, so frames from the
    // previous account must not stay on screen while the new one loads.
    cancelRequested = true;
    releaseFrames();
    suspects.value = [];
    resumeAt.value = 0;
    restoredSettings.value = '';
    pendingRestart.value = false;
    await restoreSession();
  },
);

onMounted(async () => {
  window.addEventListener('keydown', onKeydown);
  await loadCourses();
  const target = props.target;
  if (!target) return;
  if (target.course) courseId.value = String(target.course);
  if (target.activity) activityId.value = String(target.activity);
});
onBeforeUnmount(() => {
  window.removeEventListener('keydown', onKeydown);
  cancelRequested = true;
  releaseFrames();
  videos.value = [];
});
</script>

<template>
  <section class="capture-page">
    <SectionHeader
      title="回放截取"
      :subtitle="subtitle"
      :updated-at="loadedAt"
      :refreshing="loading || lessonsLoading || loadingVideo"
      @refresh="loadCourses(true)"
    />

    <InlineAlert v-if="error" tone="error" :title="error" />
    <InlineAlert v-if="notice" tone="info" :title="notice" dismissible @dismiss="notice = ''" />
    <InlineAlert tone="partial" title="仅供本人课程学习使用">
      截取结果只保存到你选择的位置，本应用不会上传课程画面；请勿传播课程内容。
    </InlineAlert>

    <div class="capture-pickers">
      <MenuSelect
        v-model="courseId"
        :options="courseOptions"
        label="课程"
        placeholder="选择课程"
        :disabled="loading || !courseOptions.length"
      />
      <MenuSelect
        v-model="activityId"
        :options="lessonOptions"
        label="回放"
        placeholder="选择回放"
        :disabled="lessonsLoading || !lessonOptions.length"
      />
      <UiButton v-if="activityId" variant="ghost" size="sm" @click="loadVideo">重新解析</UiButton>
    </div>

    <EmptyState
      v-if="loading && !courses.length"
      :icon="Presentation"
      compact
      busy
      title="正在读取思源学堂课程…"
      description="第一次会读取本学期课程，之后会先使用缓存。"
    />
    <EmptyState
      v-else-if="!courses.length && !error"
      :icon="Presentation"
      title="没有可用的课程"
      description="请先在校园账号中完成思源学堂登录，再回到这里。"
    />
    <EmptyState
      v-else-if="lessonsLoading && !lessons.length"
      :icon="Presentation"
      compact
      busy
      title="正在读取这门课的回放…"
    />
    <EmptyState
      v-else-if="courseId && !lessons.length && !lessonsLoading && !error"
      :icon="Presentation"
      compact
      title="这门课程没有课程回放"
      description="只显示思源学堂中“课程回放”类型的活动。"
    />
    <EmptyState
      v-else-if="!activityId"
      :icon="Presentation"
      compact
      title="从课时详情进入"
      description="在思源学堂的课程回放里打开一节课，点“截取讲义”。"
    />
    <EmptyState
      v-else-if="!current && !loadingVideo"
      :icon="Presentation"
      compact
      title="暂时没有可用的回放视频"
      description="按上面的提示处理后，可以点“重新解析”再试。"
    />

    <template v-if="current">
      <div class="capture-stage" ref="playerBox">
        <video
          ref="videoEl"
          :key="playKey"
          class="capture-video"
          controls
          playsinline
          preload="metadata"
          crossorigin="anonymous"
          @loadedmetadata="onLoadedMetadata"
          @error="onVideoError"
        >
          <source :src="current.downloadUrl" type="video/mp4" />
        </video>
        <div
          class="capture-roi-layer"
          :class="{ armed: roiMode }"
          @pointerdown="startRoi"
          @pointermove="moveRoi"
          @pointerup="endRoi"
          @pointercancel="endRoi"
        >
          <div v-if="roiBox" class="capture-roi" :style="roiStyle"></div>
        </div>
      </div>

      <div class="capture-bar">
        <div class="capture-bar-main">
          <MenuSelect
            v-if="videos.length > 1"
            :model-value="String(videoIndex)"
            :options="videoOptions"
            label="视频源"
            @update:model-value="changeVideo"
          />
          <MenuSelect v-model="mode" :options="modeOptions" label="扫描方式" :disabled="scanning" />
          <MenuSelect
            v-model="threshold"
            :options="thresholdOptions"
            label="新页差异"
            :disabled="scanning"
          />
          <label class="capture-check">
            <input v-model="dedupe" type="checkbox" :disabled="scanning" />
            去除重复页
          </label>
          <UiButton
            variant="ghost"
            size="sm"
            :disabled="scanning"
            :class="{ active: roiMode }"
            @click="roiMode = !roiMode"
          >
            <SquareDashed :size="14" />
            {{ roiMode ? '在画面上拖动框选' : '框选区域' }}
          </UiButton>
          <UiButton v-if="roi" variant="ghost" size="sm" :disabled="scanning" @click="clearRoi">
            使用全画面
          </UiButton>
          <span class="capture-meta">
            {{ current.label }} · {{ fileSize(current.size) }}
            <template v-if="duration"> · {{ Math.round(duration / 60) }} 分钟</template>
          </span>
        </div>
        <div class="capture-bar-actions">
          <UiButton
            v-if="!scanning"
            :variant="pendingRestart ? 'ghost' : 'primary'"
            :danger="pendingRestart"
            size="sm"
            @click="requestScan"
          >
            <Scissors :size="14" />{{
              pendingRestart
                ? `确认清空 ${frames.length} 页并重扫`
                : resumable
                  ? `继续截取（${clock(resumeAt)}）`
                  : frames.length
                    ? '重新截取'
                    : '开始截取'
            }}
          </UiButton>
          <UiButton
            v-if="!scanning && resumable"
            variant="ghost"
            size="sm"
            @click="startScan(false)"
          >
            从头扫描
          </UiButton>
          <UiButton
            v-if="!scanning && pendingRestart"
            variant="ghost"
            size="sm"
            @click="pendingRestart = false"
          >
            取消
          </UiButton>
          <UiButton v-if="scanning" variant="ghost" danger size="sm" @click="stopScan">
            停止
          </UiButton>
          <UiButton
            variant="ghost"
            size="sm"
            :disabled="savingPage || scanning"
            @click="saveCurrent"
          >
            保存当前页
          </UiButton>
        </div>
      </div>

      <div class="capture-progress" role="status">
        <span v-if="progressText">{{ progressText }}</span>
        <span v-else class="muted">
          均衡每 2 秒扫一次，发现换页后按 0.5 秒补查；重复画面会被丢弃。
        </span>
      </div>

      <div v-if="suspects.length" class="capture-suspects">
        <div class="capture-suspects-head">
          <strong>{{ suspects.length }} 段画面不稳定</strong>
          <span class="muted">过渡动画或翻页过程没有保存，可回看补查</span>
        </div>
        <div class="capture-suspect-list">
          <div
            v-for="range in suspects"
            :key="`${range.start}-${range.end}`"
            class="capture-suspect"
          >
            <span>{{ clock(range.start) }} – {{ clock(range.end) }}</span>
            <UiButton variant="ghost" size="sm" :disabled="scanning" @click="rescanSuspect(range)">
              细查此段
            </UiButton>
          </div>
        </div>
      </div>

      <EmptyState
        v-if="!frames.length && !scanning"
        :icon="Scissors"
        compact
        title="还没有截取"
        description="选好扫描方式后点“开始截取”；也可以把视频拖到想要的位置，点“保存当前页”。"
      />

      <div v-if="frames.length" class="capture-results">
        <div class="capture-results-head">
          <strong>{{ frames.length }} 页</strong>
          <span class="muted">已选 {{ selectedCount }} 页（用于删除）</span>
          <div class="capture-results-actions">
            <UiButton variant="ghost" size="sm" @click="toggleAll(true)">全选</UiButton>
            <UiButton variant="ghost" size="sm" @click="toggleAll(false)">全不选</UiButton>
            <UiButton variant="ghost" size="sm" :disabled="!selectedCount" @click="deleteSelected">
              <Trash2 :size="14" />删除所选
            </UiButton>
            <UiButton variant="ghost" size="sm" :disabled="!undo" @click="undoDelete">
              <RotateCcw :size="14" />撤销删除
            </UiButton>
            <UiButton
              variant="ghost"
              size="sm"
              :disabled="frames.length < 2 || scanning"
              @click="cleanSimilar"
            >
              清理相似页
            </UiButton>
          </div>
        </div>
        <div class="capture-grid">
          <figure
            v-for="(frame, index) in frames"
            :key="frame.id"
            class="capture-thumb"
            :class="{ dragging: dragIndex === index }"
            draggable="true"
            @dragstart="onDragStart(index)"
            @dragover.prevent
            @drop="onDrop(index)"
          >
            <label>
              <button
                type="button"
                class="capture-thumb-preview"
                :aria-label="`预览第 ${index + 1} 页`"
                @click.prevent="openPreview(index)"
              >
                <img :src="frame.url" :alt="`第 ${index + 1} 页`" loading="lazy" />
              </button>
              <input v-model="frame.selected" type="checkbox" />
              <button
                type="button"
                class="capture-thumb-time"
                :title="`跳到 ${clock(frame.time)}`"
                @click.prevent="jumpTo(frame)"
              >
                {{ clock(frame.time) }}
              </button>
            </label>
            <button type="button" class="capture-thumb-remove" @click="removeFrame(frame)">
              移除
            </button>
          </figure>
        </div>
      </div>

      <div class="capture-export">
        <UiButton
          variant="primary"
          size="sm"
          :disabled="!frames.length || exporting"
          @click="exportPdf(false)"
        >
          <Download :size="14" />原始页面 PDF
        </UiButton>
        <UiButton
          variant="ghost"
          size="sm"
          :disabled="!frames.length || exporting"
          @click="exportPdf(true)"
        >
          <Download :size="14" />笔记讲义 PDF
        </UiButton>
        <UiButton
          variant="ghost"
          size="sm"
          :disabled="!frames.length || exporting"
          @click="exportZip"
        >
          <Download :size="14" />图片包
        </UiButton>
        <span class="capture-meta"
          >导出全部 {{ frames.length }} 页；文件名带上课程与上课时间。</span
        >
      </div>
    </template>

    <div v-if="previewFrame" class="capture-preview" role="dialog" aria-label="页面预览">
      <div class="capture-preview-bar">
        <span
          >第 {{ (preview ?? 0) + 1 }} / {{ frames.length }} 页 ·
          {{ clock(previewFrame.time) }}</span
        >
        <div class="capture-preview-actions">
          <UiButton variant="ghost" size="sm" @click="movePreview(-1)">上一页</UiButton>
          <UiButton variant="ghost" size="sm" @click="movePreview(1)">下一页</UiButton>
          <UiButton variant="ghost" size="sm" @click="removePreview">删除此页</UiButton>
          <UiButton variant="ghost" size="sm" @click="closePreview"><X :size="14" />关闭</UiButton>
        </div>
      </div>
      <img :src="previewFrame.url" :alt="`第 ${(preview ?? 0) + 1} 页`" />
      <button
        type="button"
        class="capture-preview-nav prev"
        aria-label="上一页"
        @click="movePreview(-1)"
      >
        <ChevronLeft :size="20" />
      </button>
      <button
        type="button"
        class="capture-preview-nav next"
        aria-label="下一页"
        @click="movePreview(1)"
      >
        <ChevronRight :size="20" />
      </button>
    </div>
  </section>
</template>

<style scoped>
.capture-page {
  padding-bottom: 24px;
}
.capture-pickers {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  margin: 12px 0 14px;
}
.capture-pickers :deep(.menu-field) {
  min-width: 220px;
}
.capture-stage {
  position: relative;
  overflow: hidden;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: #000;
}
.capture-video {
  display: block;
  width: 100%;
  max-height: 56vh;
  background: #000;
}
.capture-roi-layer {
  position: absolute;
  inset: 0;
  pointer-events: none;
}
.capture-roi-layer.armed {
  pointer-events: auto;
  cursor: crosshair;
}
.capture-roi {
  position: absolute;
  border: 1.5px solid var(--accent);
  background: rgba(31, 111, 229, 0.12);
  box-shadow: 0 0 0 9999px rgba(15, 23, 42, 0.35);
}
.capture-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 10px;
  margin-top: 12px;
}
.capture-bar-main,
.capture-bar-actions {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
}
.capture-bar :deep(.menu-field) {
  min-width: 148px;
}
.capture-check {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  color: var(--text-secondary);
  font-size: 12px;
  white-space: nowrap;
}
.capture-check input {
  margin: 0;
}
.capture-meta {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.capture-progress {
  min-height: 18px;
  margin: 8px 0 4px;
  color: var(--text-secondary);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}
.capture-progress .muted {
  color: var(--text-tertiary);
}
.capture-suspects {
  margin: 8px 0 4px;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface);
}
.capture-suspects-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 8px;
  font-size: 12.5px;
}
.capture-suspects-head .muted {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.capture-suspect-list {
  display: flex;
  flex-wrap: wrap;
  gap: 6px;
  margin-top: 8px;
}
.capture-suspect {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 2px 4px 2px 8px;
  border: 1px solid var(--divider);
  border-radius: 999px;
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.capture-results {
  margin-top: 12px;
}
.capture-results-head {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
  font-size: 12.5px;
}
.capture-results-head .muted {
  color: var(--text-tertiary);
}
.capture-results-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
  margin-left: auto;
}
.capture-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  gap: 10px;
}
.capture-thumb {
  margin: 0;
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  background: var(--surface);
  overflow: hidden;
}
.capture-thumb.dragging {
  opacity: 0.5;
}
.capture-thumb label {
  position: relative;
  display: block;
}
.capture-thumb-preview {
  display: block;
  width: 100%;
  padding: 0;
  border: 0;
  background: #111;
  cursor: zoom-in;
}
.capture-thumb img {
  display: block;
  width: 100%;
  aspect-ratio: 16 / 9;
  object-fit: contain;
  background: #111;
}
.capture-thumb input {
  position: absolute;
  top: 6px;
  left: 6px;
  margin: 0;
}
.capture-thumb-time {
  position: absolute;
  right: 6px;
  bottom: 6px;
  padding: 1px 5px;
  border: 0;
  border-radius: 999px;
  background: rgba(15, 23, 42, 0.72);
  color: #fff;
  font: inherit;
  font-size: 10.5px;
  font-variant-numeric: tabular-nums;
  cursor: pointer;
}
.capture-thumb-time:hover {
  background: rgba(15, 23, 42, 0.9);
}
.capture-thumb-remove {
  display: block;
  width: 100%;
  padding: 5px;
  border: 0;
  border-top: 1px solid var(--divider);
  background: transparent;
  color: var(--text-secondary);
  font: inherit;
  font-size: 11.5px;
  cursor: pointer;
}
.capture-thumb-remove:hover {
  background: var(--surface-hover);
  color: var(--text);
}
.capture-export {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}
.capture-preview {
  position: fixed;
  inset: 0;
  z-index: 60;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 24px 72px;
  background: rgba(8, 12, 20, 0.86);
}
.capture-preview-bar {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  color: #e5e7eb;
  font-size: 12.5px;
}
.capture-preview-actions {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}
.capture-preview img {
  max-width: 100%;
  max-height: calc(100vh - 120px);
  border-radius: var(--radius-md);
  background: #111;
}
.capture-preview-nav {
  position: absolute;
  top: 50%;
  display: grid;
  place-items: center;
  width: 36px;
  height: 64px;
  border: 0;
  border-radius: var(--radius-md);
  background: rgba(15, 23, 42, 0.6);
  color: #fff;
  cursor: pointer;
}
.capture-preview-nav.prev {
  left: 18px;
}
.capture-preview-nav.next {
  right: 18px;
}
</style>
