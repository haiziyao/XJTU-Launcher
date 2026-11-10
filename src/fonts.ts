import { bridge } from './bridge';
import type { FontLibraryState, FontPluginSettings, FontRegion, FontRegionId } from './types';

export const FONT_PLUGIN_ID = 'cn.xjtu.font-beautify';

/** Same values the Rust side writes when a region has never been set. */
export const DEFAULT_FONT_SETTINGS: FontPluginSettings = {
  ui: { family: 'system', size: 13, weight: 400, lineHeight: 1.5, letterSpacing: 0, color: '' },
  title: {
    family: 'system',
    size: 20,
    weight: 600,
    lineHeight: 1.35,
    letterSpacing: 0,
    color: '',
  },
  body: {
    family: 'system',
    size: 12.5,
    weight: 400,
    lineHeight: 1.6,
    letterSpacing: 0,
    color: '',
  },
  timetable: {
    family: 'system',
    size: 13,
    weight: 600,
    lineHeight: 1.35,
    letterSpacing: 0,
    color: '',
  },
  code: { family: 'mono', size: 12, weight: 400, lineHeight: 1.5, letterSpacing: 0, color: '' },
};

export const FONT_REGIONS: { id: FontRegionId; name: string; description: string }[] = [
  { id: 'ui', name: '全局', description: '按钮、导航与输入框' },
  { id: 'title', name: '标题', description: '页面标题与分组标题' },
  { id: 'body', name: '正文', description: '列表与说明文字' },
  { id: 'timetable', name: '课表', description: '课表格子里的节次与课程' },
  { id: 'code', name: '代码', description: '等宽显示的路径、命令与代码' },
];

export type FontCategoryId = 'global' | 'title' | 'body' | 'code';

export interface FontCategory {
  id: FontCategoryId;
  name: string;
  description: string;
  /** Regions this category writes; the first one is also the preview sample. */
  regions: FontRegionId[];
}

/**
 * The page groups the five stored regions into four user-facing categories:
 * the timetable follows the global font, so it has no row of its own.
 */
export const FONT_CATEGORIES: FontCategory[] = [
  {
    id: 'global',
    name: '全局',
    description: '按钮、导航、课表与未单独设置的地方',
    regions: ['ui', 'timetable'],
  },
  { id: 'title', name: '标题', description: '页面标题与分组标题', regions: ['title'] },
  { id: 'body', name: '正文', description: '列表、段落与说明文字', regions: ['body'] },
  { id: 'code', name: '代码', description: '路径、命令与代码块', regions: ['code'] },
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

/**
 * Downloaded fonts are not installed into the OS; each installed file is
 * registered as a `FontFace` under its family name. The settings keep storing
 * plain family names, so existing plugin files and the family input keep
 * working without a translation layer.
 */
let libraryState: FontLibraryState | null = null;
let libraryRequest: Promise<FontLibraryState | null> | null = null;
const loadedFaces = new Map<string, FontFace>();
const loadingFaces = new Map<string, Promise<void>>();
let applyGeneration = 0;

export function fontFamilyStack(family: string, fallback: 'sans' | 'mono'): string {
  if (family === 'system') return 'var(--font-sans)';
  if (family === 'mono') return 'var(--font-mono)';
  const clean = family.replace(/["\\\n\r]/g, '').trim();
  return clean ? `"${clean}", var(--font-${fallback})` : `var(--font-${fallback})`;
}

/** Applies one installed family to every region, leaving the fine tuning alone. */
export function applyFontToRegions(
  settings: FontPluginSettings,
  family: string,
): FontPluginSettings {
  const next = { ...settings };
  for (const region of FONT_REGIONS) {
    next[region.id] = { ...next[region.id], family };
  }
  return next;
}

export function setFontLibrary(state: FontLibraryState | null) {
  libraryState = state;
  if (!state || typeof document === 'undefined' || !document.fonts) return;
  const families = new Set(state.installed.map((font) => font.family));
  for (const [family, face] of loadedFaces) {
    if (!families.has(family)) {
      document.fonts.delete(face);
      loadedFaces.delete(family);
    }
  }
}

export async function refreshFontLibrary(): Promise<FontLibraryState | null> {
  try {
    const state = await bridge.fontsState();
    setFontLibrary(state);
    return state;
  } catch {
    return null;
  }
}

async function currentLibrary(): Promise<FontLibraryState | null> {
  if (libraryState) return libraryState;
  if (!libraryRequest) {
    libraryRequest = refreshFontLibrary().finally(() => {
      libraryRequest = null;
    });
  }
  return libraryRequest;
}

/** Loads installed families without changing the page's CSS. */
export async function ensureFontsLoaded(families: Iterable<string>): Promise<void> {
  await Promise.all([...families].map(ensureFamilyLoaded));
}

async function ensureFamilyLoaded(family: string): Promise<void> {
  if (family === 'system' || family === 'mono') return;
  if (typeof document === 'undefined' || !document.fonts || typeof FontFace === 'undefined') {
    return;
  }
  if (loadedFaces.has(family)) return;
  const pending = loadingFaces.get(family);
  if (pending) return pending;

  const task = (async () => {
    const state = await currentLibrary();
    const installed = state?.installed.find((font) => font.family === family);
    if (!installed) return;
    try {
      const bytes = await bridge.fontData(installed.id);
      // The font may have been removed while its bytes were in flight.
      if (!libraryState?.installed.some((font) => font.id === installed.id)) return;
      const face = new FontFace(family, bytes);
      await face.load();
      if (!libraryState?.installed.some((font) => font.id === installed.id)) return;
      document.fonts.add(face);
      loadedFaces.set(family, face);
    } catch {
      // Keep the fallback stack; a broken file must not block the app.
    }
  })();
  loadingFaces.set(family, task);
  try {
    await task;
  } finally {
    loadingFaces.delete(family);
  }
}

/**
 * Writes the font regions to CSS custom properties and switches the scoped
 * rules in `fonts.css` on. Installed font files are loaded first so the page
 * never swaps to a family the webview cannot render. Disabling the plugin
 * clears the class, returning the app to its bundled typography.
 */
export async function applyFonts(settings: FontPluginSettings | null, enabled: boolean) {
  const generation = ++applyGeneration;
  const root = document.documentElement;
  if (!enabled || !settings) {
    root.classList.remove('font-styled');
    for (const region of FONT_REGIONS) root.classList.remove(`font-color-${region.id}`);
    return;
  }
  const families = new Set(
    FONT_REGIONS.map((region) => settings[region.id].family).filter(
      (family) => family !== 'system' && family !== 'mono',
    ),
  );
  await Promise.all([...families].map(ensureFamilyLoaded));
  if (generation !== applyGeneration) return;
  for (const region of FONT_REGIONS) {
    setRegion(root, region.id, settings[region.id], region.id === 'code' ? 'mono' : 'sans');
  }
  root.classList.add('font-styled');
}

function setRegion(
  root: HTMLElement,
  id: FontRegionId,
  region: FontRegion,
  fallback: 'sans' | 'mono',
) {
  const prefix = `--plugin-${id}`;
  root.style.setProperty(`${prefix}-family`, fontFamilyStack(region.family, fallback));
  root.style.setProperty(`${prefix}-size`, `${region.size}px`);
  root.style.setProperty(`${prefix}-weight`, String(region.weight));
  root.style.setProperty(`${prefix}-leading`, String(region.lineHeight));
  root.style.setProperty(`${prefix}-tracking`, `${region.letterSpacing}px`);
  // Only override the theme colour when the user picked one.
  const color = /^#[0-9a-f]{6}$/i.test(region.color ?? '') ? region.color : '';
  root.classList.toggle(`font-color-${id}`, !!color);
  if (color) root.style.setProperty(`${prefix}-color`, color);
  else root.style.removeProperty(`${prefix}-color`);
}

export function normalizeFontSettings(value: unknown): FontPluginSettings {
  const result = { ...DEFAULT_FONT_SETTINGS };
  if (value && typeof value === 'object') {
    for (const region of FONT_REGIONS) {
      const entry = (value as Record<string, unknown>)[region.id];
      if (entry && typeof entry === 'object') {
        result[region.id] = { ...DEFAULT_FONT_SETTINGS[region.id], ...(entry as FontRegion) };
      }
    }
  }
  // The timetable no longer has its own category: it follows the global font.
  // Its size / weight defaults stay until the global controls are edited.
  result.timetable = { ...result.timetable, family: result.ui.family };
  return result;
}
