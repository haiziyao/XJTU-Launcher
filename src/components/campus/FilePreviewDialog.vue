<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue';
import { Download, FileQuestion, X } from 'lucide-vue-next';
import * as pdfjs from 'pdfjs-dist/legacy/build/pdf.mjs';
import workerUrl from 'pdfjs-dist/legacy/build/pdf.worker.min.mjs?url';
import Skeleton from '../ui/Skeleton.vue';
import UiButton from '../ui/UiButton.vue';
import UiIconButton from '../ui/UiIconButton.vue';
import { fileSize } from '../../domain';
import type { FilePreview } from '../../types';

/**
 * In-app preview for course and notice attachments (LAYOUT §4.2, SPEC §4.4).
 * Nothing in the file is executed: images are shown as images, HTML arrives as
 * text, and PDFs are parsed by pdf.js. The PDF bytes come from the caller
 * through `pdfSource` — the backend only hands out a preview id.
 */
const props = withDefaults(
  defineProps<{
    open: boolean;
    preview: FilePreview | null;
    loading?: boolean;
    error?: string;
    downloadable?: boolean;
    /** Resolves the PDF bytes for `kind: 'pdf'`. */
    pdfSource?: (() => Promise<ArrayBuffer | Uint8Array>) | null;
  }>(),
  { loading: false, error: '', downloadable: true, pdfSource: null },
);
const emit = defineEmits<{ close: []; download: [] }>();

// The worker URL must be served from our own origin; CSP needs worker-src 'self'.
pdfjs.GlobalWorkerOptions.workerSrc = workerUrl;

const pages = ref<HTMLElement | null>(null);
const pdfError = ref('');
const rendering = ref(false);
let task: pdfjs.PDFDocumentLoadingTask | null = null;
let run = 0;

const title = computed(() => props.preview?.name || '文件预览');
const meta = computed(() => {
  const value = props.preview;
  if (!value) return '';
  if (value.kind === 'unsupported') return fileSize(value.size);
  return [value.kind === 'pdf' ? 'PDF' : value.mime, fileSize(value.size)]
    .filter(Boolean)
    .join(' · ');
});
const busy = computed(() => props.loading || rendering.value);

function cleanup() {
  run++;
  rendering.value = false;
  if (task) {
    void task.destroy();
    task = null;
  }
  if (pages.value) pages.value.replaceChildren();
}

async function renderPdf(source: NonNullable<typeof props.pdfSource>) {
  cleanup();
  const current = ++run;
  pdfError.value = '';
  rendering.value = true;
  try {
    const bytes = await source();
    if (current !== run) return;
    const documentTask = pdfjs.getDocument({ data: new Uint8Array(bytes) });
    task = documentTask;
    const pdf = await documentTask.promise;
    if (current !== run) return;
    await nextTick();
    const host = pages.value;
    if (!host) return;
    const width = Math.min(host.clientWidth || 720, 820);
    const ratio = window.devicePixelRatio || 1;
    for (let number = 1; number <= pdf.numPages; number++) {
      if (current !== run) return;
      const page = await pdf.getPage(number);
      const base = page.getViewport({ scale: 1 });
      const viewport = page.getViewport({ scale: (width / base.width) * ratio });
      const canvas = document.createElement('canvas');
      canvas.width = Math.floor(viewport.width);
      canvas.height = Math.floor(viewport.height);
      canvas.style.width = `${Math.floor(viewport.width / ratio)}px`;
      canvas.style.height = `${Math.floor(viewport.height / ratio)}px`;
      canvas.className = 'pdf-page';
      host.appendChild(canvas);
      await page.render({ canvas, viewport }).promise;
    }
  } catch (error) {
    if (current === run) pdfError.value = error instanceof Error ? error.message : String(error);
  } finally {
    if (current === run) rendering.value = false;
  }
}

function load() {
  cleanup();
  pdfError.value = '';
  if (!props.open || props.preview?.kind !== 'pdf') return;
  if (!props.pdfSource) {
    pdfError.value = '这个 PDF 暂时无法在应用内预览，请下载后查看。';
    return;
  }
  void renderPdf(props.pdfSource);
}

watch(() => [props.open, props.preview], load, { immediate: true });
watch(
  () => props.open,
  (value) => {
    if (value) document.addEventListener('keydown', onKey, true);
    else document.removeEventListener('keydown', onKey, true);
  },
  { immediate: true },
);
function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') emit('close');
}
onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKey, true);
  cleanup();
});
</script>

<template>
  <Teleport to="body">
    <div v-if="open" class="preview-backdrop" @click.self="emit('close')">
      <section
        class="preview-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="preview-title"
      >
        <header class="preview-head">
          <div class="preview-heading">
            <h2 id="preview-title">{{ title }}</h2>
            <p v-if="meta">{{ meta }}</p>
          </div>
          <div class="preview-actions">
            <UiButton
              v-if="downloadable && preview && preview.kind !== 'unsupported'"
              size="sm"
              @click="emit('download')"
            >
              <Download :size="14" /><span>下载</span>
            </UiButton>
            <UiIconButton label="关闭预览" @click="emit('close')"><X :size="16" /></UiIconButton>
          </div>
        </header>

        <div class="preview-body">
          <Skeleton v-if="loading" :lines="6" height="16px" label="正在读取文件" />
          <p v-else-if="error" class="preview-message error" role="alert">{{ error }}</p>
          <p v-else-if="pdfError" class="preview-message error" role="alert">{{ pdfError }}</p>
          <template v-else-if="preview">
            <div v-if="preview.kind === 'image'" class="preview-image">
              <img :src="preview.dataUrl" :alt="preview.name" />
            </div>
            <div v-else-if="preview.kind === 'text'" class="preview-text-wrap">
              <pre class="preview-text">{{ preview.text }}</pre>
              <p v-if="preview.truncated" class="preview-message">文件较大，只显示前 1 MB 内容。</p>
            </div>
            <div v-else-if="preview.kind === 'pdf'" ref="pages" class="preview-pdf">
              <Skeleton v-if="rendering" :lines="4" height="18px" label="正在渲染 PDF" />
            </div>
            <div v-else class="preview-unsupported">
              <FileQuestion :size="26" aria-hidden="true" />
              <p>这种文件不能在应用内预览，请下载后打开。</p>
              <UiButton v-if="downloadable" size="sm" @click="emit('download')">下载文件</UiButton>
            </div>
          </template>
          <p v-else class="preview-message">没有可预览的内容。</p>
          <p v-if="busy && preview?.kind !== 'pdf'" class="visually-hidden" role="status">
            正在载入
          </p>
        </div>
      </section>
    </div>
  </Teleport>
</template>

<style scoped>
.preview-backdrop {
  position: fixed;
  z-index: 120;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 24px;
  background: rgba(15, 23, 42, 0.32);
}
.preview-dialog {
  display: flex;
  flex-direction: column;
  width: min(860px, 100%);
  max-height: min(86vh, 900px);
  border-radius: var(--radius-lg);
  background: var(--surface);
  box-shadow: var(--shadow-lg);
  overflow: hidden;
}
.preview-head {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
  padding: 14px 16px 12px 20px;
  border-bottom: 1px solid var(--border);
}
.preview-heading {
  min-width: 0;
}
.preview-heading h2 {
  overflow: hidden;
  font-size: 14px;
  font-weight: 600;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.preview-heading p {
  margin-top: 2px;
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.preview-actions {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-shrink: 0;
}
.preview-body {
  flex: 1;
  min-height: 0;
  padding: 18px 20px 22px;
  overflow: auto;
  background: var(--surface-subtle);
}
.preview-image {
  display: grid;
  place-items: center;
}
.preview-image img {
  max-width: 100%;
  max-height: 68vh;
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-sm);
}
.preview-text-wrap {
  border: 1px solid var(--border);
  border-radius: var(--radius);
  background: var(--surface);
}
.preview-text {
  margin: 0;
  padding: 14px 16px;
  color: var(--text);
  font-family: var(--font-mono);
  font-size: 12px;
  line-height: 1.65;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
}
.preview-pdf {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 12px;
}
.preview-pdf :deep(canvas) {
  max-width: 100%;
  height: auto !important;
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-sm);
  background: #fff;
}
.preview-unsupported {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 10px;
  padding: 40px 20px;
  color: var(--text-secondary);
  text-align: center;
  font-size: 12.5px;
}
.preview-message {
  color: var(--text-tertiary);
  font-size: 12.5px;
  text-align: center;
}
.preview-message.error {
  color: var(--danger);
}
.preview-text-wrap .preview-message {
  padding: 0 16px 12px;
  text-align: left;
}
.visually-hidden {
  position: absolute;
  width: 1px;
  height: 1px;
  margin: -1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
  white-space: nowrap;
  border: 0;
}
</style>
