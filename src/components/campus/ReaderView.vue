<script setup lang="ts">
import { computed, onBeforeUnmount, ref, watch } from 'vue';
import DOMPurify from 'dompurify';
import { Download, X } from 'lucide-vue-next';
import EmptyState from '../ui/EmptyState.vue';
import Skeleton from '../ui/Skeleton.vue';
import UiButton from '../ui/UiButton.vue';
import UiIconButton from '../ui/UiIconButton.vue';
import { fileSize } from '../../domain';

/**
 * Notice reader (LAYOUT §9.10, SPEC `NoticeDetail`). The HTML is cleaned here
 * again even though the backend already filters it. Links never navigate the
 * app window: they ask the page to open the campus site in the web window.
 */
const props = withDefaults(
  defineProps<{
    html: string;
    title?: string;
    source?: string;
    date?: string;
    attachments?: { name: string; url: string; size: number | null }[];
    truncated?: boolean;
    loading?: boolean;
    error?: string;
  }>(),
  {
    title: '',
    source: '',
    date: '',
    attachments: () => [],
    truncated: false,
    loading: false,
    error: '',
  },
);
const emit = defineEmits<{
  openLink: [href: string];
  download: [attachment: { name: string; url: string; size: number | null }];
}>();

const clean = computed(() =>
  props.html
    ? DOMPurify.sanitize(props.html, {
        FORBID_TAGS: ['style', 'form', 'input', 'textarea', 'select', 'button', 'iframe'],
        FORBID_ATTR: ['style'],
      })
    : '',
);
const meta = computed(() => [props.source, props.date].filter(Boolean).join(' · '));

const zoomed = ref('');

function onClick(event: MouseEvent) {
  const target = event.target as Element | null;
  const link = target?.closest('a');
  if (link instanceof HTMLAnchorElement) {
    event.preventDefault();
    emit('openLink', link.getAttribute('href') ?? '');
    return;
  }
  const image = target?.closest('img');
  if (image instanceof HTMLImageElement) {
    event.preventDefault();
    zoomed.value = image.currentSrc || image.src;
  }
}
function onKey(event: KeyboardEvent) {
  if (event.key === 'Escape') zoomed.value = '';
}
watch(zoomed, (value) => {
  if (value) document.addEventListener('keydown', onKey, true);
  else document.removeEventListener('keydown', onKey, true);
});
onBeforeUnmount(() => document.removeEventListener('keydown', onKey, true));
</script>

<template>
  <article class="reader" :aria-label="title || '通知正文'">
    <header v-if="title" class="reader-head">
      <h1>{{ title }}</h1>
      <p v-if="meta" class="reader-meta">{{ meta }}</p>
      <slot name="actions" />
    </header>

    <Skeleton v-if="loading" :lines="8" height="15px" label="正在读取通知正文" />
    <EmptyState
      v-else-if="error"
      compact
      tone="danger"
      title="无法读取通知正文"
      :description="error"
    />
    <template v-else>
      <div class="reader-body" @click="onClick" v-html="clean"></div>
      <p v-if="truncated" class="reader-truncated">
        正文较长，只显示了一部分；完整内容可在校园网页中查看。
      </p>
      <section v-if="attachments.length" class="reader-attachments">
        <h2>附件</h2>
        <ul>
          <li v-for="attachment in attachments" :key="attachment.url">
            <span class="reader-file-name">{{ attachment.name }}</span>
            <span class="reader-file-size">{{ fileSize(attachment.size) }}</span>
            <UiButton size="sm" @click="emit('download', attachment)">
              <Download :size="13" /><span>下载</span>
            </UiButton>
          </li>
        </ul>
      </section>
    </template>

    <Teleport to="body">
      <div v-if="zoomed" class="reader-zoom" @click="zoomed = ''">
        <img :src="zoomed" alt="放大的图片" />
        <UiIconButton
          label="关闭图片"
          variant="neutral"
          shape="circle"
          size="md"
          class="reader-zoom-close"
          @click.stop="zoomed = ''"
        >
          <X :size="16" />
        </UiIconButton>
      </div>
    </Teleport>
  </article>
</template>

<style scoped>
.reader {
  max-width: 680px;
  min-width: 0;
}
.reader-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: 4px 12px;
  margin-bottom: 14px;
}
.reader-head h1 {
  width: 100%;
  font-size: 17px;
  font-weight: 600;
  line-height: 1.4;
}
.reader-meta {
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.reader-body {
  color: var(--text);
  font-size: 13.5px;
  line-height: 1.75;
  overflow-wrap: anywhere;
}
/* Content arrives from the school site; keep its structure but fit our column. */
.reader-body :deep(p) {
  margin: 0 0 12px;
}
.reader-body :deep(h1),
.reader-body :deep(h2),
.reader-body :deep(h3) {
  margin: 20px 0 8px;
  font-size: 14px;
  font-weight: 600;
  line-height: 1.5;
}
.reader-body :deep(a) {
  color: var(--accent-text);
  text-decoration: underline;
  text-underline-offset: 2px;
}
.reader-body :deep(img) {
  display: block;
  max-width: 100%;
  height: auto;
  margin: 12px 0;
  border-radius: var(--radius-sm);
  cursor: zoom-in;
}
.reader-body :deep(table) {
  width: 100%;
  border-collapse: collapse;
  margin: 12px 0;
  font-size: 12.5px;
}
.reader-body :deep(th),
.reader-body :deep(td) {
  padding: 6px 8px;
  border: 1px solid var(--border);
  text-align: left;
}
.reader-body :deep(blockquote) {
  margin: 12px 0;
  padding: 8px 12px;
  border-left: 3px solid var(--border-strong);
  color: var(--text-secondary);
}
.reader-body :deep(pre) {
  padding: 10px 12px;
  border-radius: var(--radius);
  background: var(--surface-sunken);
  overflow: auto;
  font-family: var(--font-mono);
  font-size: 12px;
}
.reader-truncated {
  margin-top: 14px;
  color: var(--text-tertiary);
  font-size: 11.5px;
}
.reader-attachments {
  margin-top: 22px;
  padding-top: 14px;
  border-top: 1px solid var(--border);
}
.reader-attachments h2 {
  margin-bottom: 8px;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
}
.reader-attachments ul {
  margin: 0;
  padding: 0;
  list-style: none;
  border: 1px solid var(--border);
  border-radius: var(--radius);
  overflow: hidden;
}
.reader-attachments li {
  display: flex;
  align-items: center;
  gap: 10px;
  min-height: 38px;
  padding: 4px 8px 4px 12px;
  font-size: 12.5px;
}
.reader-attachments li + li {
  border-top: 1px solid var(--divider);
}
.reader-file-name {
  flex: 1;
  min-width: 0;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.reader-file-size {
  color: var(--text-tertiary);
  font-size: 11.5px;
  font-variant-numeric: tabular-nums;
}
.reader-zoom {
  position: fixed;
  z-index: 140;
  inset: 0;
  display: grid;
  place-items: center;
  padding: 32px;
  background: rgba(15, 23, 42, 0.8);
  cursor: zoom-out;
}
.reader-zoom img {
  max-width: 100%;
  max-height: 88vh;
  border-radius: var(--radius-sm);
  box-shadow: var(--shadow-lg);
}
.reader-zoom-close {
  position: absolute;
  top: 16px;
  right: 16px;
  background: rgba(255, 255, 255, 0.9);
}
</style>
