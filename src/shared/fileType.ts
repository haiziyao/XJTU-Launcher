import type { Component } from 'vue';
import { File, FileArchive, FileImage, FileText, FileVideo } from 'lucide-vue-next';
import type { Tone } from '../components/ui/IconTile.vue';

const IMAGE = ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'svg', 'heic'];
const VIDEO = ['mp4', 'mov', 'mkv', 'avi', 'webm'];
const ARCHIVE = ['zip', 'rar', '7z', 'tar', 'gz'];
const TEXT = ['pdf', 'doc', 'docx', 'ppt', 'pptx', 'xls', 'xlsx', 'txt', 'md', 'csv'];
const LABEL: Record<string, string> = {
  pdf: 'PDF',
  doc: 'Word',
  docx: 'Word',
  ppt: 'PPT',
  pptx: 'PPT',
  xls: '表格',
  xlsx: '表格',
  csv: 'CSV',
  txt: '文本',
  md: '文本',
  zip: '压缩包',
  rar: '压缩包',
  '7z': '压缩包',
  mp4: '视频',
  mov: '视频',
  mkv: '视频',
  png: '图片',
  jpg: '图片',
  jpeg: '图片',
  gif: '图片',
  svg: '图片',
};

export interface FileKind {
  icon: Component;
  tone: Tone;
  label: string;
}

/** Shared file icon, colour and type label for rows and local-library lists. */
export function fileKind(name: string): FileKind {
  const extension = name.split('.').pop()?.toLowerCase() ?? '';
  if (IMAGE.includes(extension)) {
    return { icon: FileImage, tone: 'violet', label: LABEL[extension] || '图片' };
  }
  if (VIDEO.includes(extension)) return { icon: FileVideo, tone: 'red', label: '视频' };
  if (ARCHIVE.includes(extension)) return { icon: FileArchive, tone: 'orange', label: '压缩包' };
  if (TEXT.includes(extension)) {
    return {
      icon: FileText,
      tone: 'blue',
      label: LABEL[extension] || extension.toUpperCase(),
    };
  }
  return { icon: File, tone: 'gray', label: extension.toUpperCase() || '文件' };
}
