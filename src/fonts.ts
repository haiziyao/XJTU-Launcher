import type { FontPluginSettings, FontRegion, FontRegionId } from './types';

export const FONT_PLUGIN_ID = 'cn.xjtu.font-beautify';

/** Same values the Rust side writes when a region has never been set. */
export const DEFAULT_FONT_SETTINGS: FontPluginSettings = {
  ui: { family: 'system', size: 13, weight: 400, lineHeight: 1.5, letterSpacing: 0 },
  title: { family: 'system', size: 20, weight: 600, lineHeight: 1.35, letterSpacing: 0 },
  body: { family: 'system', size: 12.5, weight: 400, lineHeight: 1.6, letterSpacing: 0 },
  timetable: { family: 'system', size: 13, weight: 600, lineHeight: 1.35, letterSpacing: 0 },
  code: { family: 'mono', size: 12, weight: 400, lineHeight: 1.5, letterSpacing: 0 },
};

export const FONT_REGIONS: { id: FontRegionId; name: string; description: string }[] = [
  { id: 'ui', name: '界面', description: '按钮、导航、标签与输入框' },
  { id: 'title', name: '标题', description: '页面标题与分组标题' },
  { id: 'body', name: '正文', description: '列表与说明文字' },
  { id: 'timetable', name: '课表数字', description: '课表格子里的节次与课程' },
  { id: 'code', name: '代码', description: '等宽显示的路径、命令与代码' },
];

/** Built-in choices in the family picker; a custom family can be typed instead. */
export const FONT_FAMILIES = [
  { id: 'system', name: '系统默认' },
  { id: 'mono', name: '系统等宽' },
  { id: 'PingFang SC', name: '苹方' },
  { id: 'Hiragino Sans GB', name: '冬青黑体' },
  { id: 'Songti SC', name: '宋体' },
  { id: 'Kaiti SC', name: '楷体' },
  { id: 'Georgia', name: 'Georgia' },
  { id: 'Times New Roman', name: 'Times New Roman' },
  { id: 'Menlo', name: 'Menlo' },
];

function familyStack(family: string, fallback: 'sans' | 'mono'): string {
  if (family === 'system') return 'var(--font-sans)';
  if (family === 'mono') return 'var(--font-mono)';
  const quote = /^[A-Za-z0-9 -]+$/.test(family);
  return `${quote ? `"${family}"` : family}, var(--font-${fallback})`;
}

function setRegion(
  root: HTMLElement,
  id: FontRegionId,
  region: FontRegion,
  fallback: 'sans' | 'mono',
) {
  const prefix = `--plugin-${id}`;
  root.style.setProperty(`${prefix}-family`, familyStack(region.family, fallback));
  root.style.setProperty(`${prefix}-size`, `${region.size}px`);
  root.style.setProperty(`${prefix}-weight`, String(region.weight));
  root.style.setProperty(`${prefix}-leading`, String(region.lineHeight));
  root.style.setProperty(`${prefix}-tracking`, `${region.letterSpacing}px`);
}

/**
 * Writes the font regions to CSS custom properties and switches the scoped
 * rules in `fonts.css` on. Disabling the plugin clears both, so the app returns
 * to the bundled typography without a reload.
 */
export function applyFonts(settings: FontPluginSettings | null, enabled: boolean) {
  const root = document.documentElement;
  if (!enabled || !settings) {
    root.classList.remove('font-styled');
    return;
  }
  for (const region of FONT_REGIONS) {
    setRegion(root, region.id, settings[region.id], region.id === 'code' ? 'mono' : 'sans');
  }
  root.classList.add('font-styled');
}

export function normalizeFontSettings(value: unknown): FontPluginSettings {
  const result = { ...DEFAULT_FONT_SETTINGS };
  if (!value || typeof value !== 'object') return result;
  for (const region of FONT_REGIONS) {
    const entry = (value as Record<string, unknown>)[region.id];
    if (entry && typeof entry === 'object') {
      result[region.id] = { ...DEFAULT_FONT_SETTINGS[region.id], ...(entry as FontRegion) };
    }
  }
  return result;
}
