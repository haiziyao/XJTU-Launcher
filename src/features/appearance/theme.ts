// Theme colours are derived from a single accent value so the sidebar highlight
// and every accent-tinted label stay in step. A later plugin theme can replace
// `applyAccent` without touching component styles.

export interface AccentPreset {
  name: string;
  value: string;
}

export const DEFAULT_ACCENT = '#1f6fe5';

export const ACCENT_PRESETS: AccentPreset[] = [
  { name: '经典蓝', value: DEFAULT_ACCENT },
  { name: '青碧', value: '#0f8f86' },
  { name: '松绿', value: '#2c8653' },
  { name: '紫罗兰', value: '#7a5af8' },
  { name: '琥珀', value: '#c2740b' },
  { name: '玫红', value: '#c93a6b' },
];

// Configs written before the picker existed stored colour names.
const LEGACY_ACCENTS: Record<string, string> = {
  blue: DEFAULT_ACCENT,
  sage: '#2c8653',
  violet: '#7a5af8',
  amber: '#c2740b',
};

const HEX = /^#[0-9a-f]{6}$/i;
type Rgb = [number, number, number];

export function normalizeAccent(value: unknown): string {
  if (typeof value !== 'string') return DEFAULT_ACCENT;
  if (LEGACY_ACCENTS[value]) return LEGACY_ACCENTS[value];
  return HEX.test(value) ? value.toLowerCase() : DEFAULT_ACCENT;
}

function parse(color: string): Rgb {
  return [1, 3, 5].map((start) => parseInt(color.slice(start, start + 2), 16)) as Rgb;
}

function toHex(rgb: Rgb): string {
  return `#${rgb.map((v) => Math.round(v).toString(16).padStart(2, '0')).join('')}`;
}

function mix(from: string, to: string, amount: number): string {
  const a = parse(from);
  const b = parse(to);
  const t = Math.max(0, Math.min(1, amount));
  return toHex([0, 1, 2].map((i) => a[i] + (b[i] - a[i]) * t) as Rgb);
}

function channel(value: number): number {
  const c = value / 255;
  return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
}

function luminance(color: string): number {
  const [r, g, b] = parse(color).map(channel) as Rgb;
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

export function contrast(a: string, b: string): number {
  const values = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (values[0] + 0.05) / (values[1] + 0.05);
}

function rgba(color: string, alpha: number): string {
  const [r, g, b] = parse(color);
  return `rgba(${r}, ${g}, ${b}, ${alpha})`;
}

// Muted text keeps the neutral greys' hierarchy but picks up a little of the
// accent's hue, so it still reads where a wallpaper shows through the page.
const MUTED_BASE = '#4a505a';
const FAINT_BASE = '#6f7681';

/** Darkens `color` towards black until it reaches the required contrast. */
function ensureContrast(color: string, background: string, target: number): string {
  let candidate = color;
  for (let step = 1; step <= 32; step += 1) {
    if (contrast(candidate, background) >= target) return candidate;
    candidate = mix(color, '#000000', step * 0.04);
  }
  return candidate;
}

/** Darkens the accent just enough to read as text on the given background. */
export function readableAccent(accent: string, background = '#ffffff'): string {
  return ensureContrast(accent, background, 4.5);
}

/** Text colour for content set directly on an accent fill. */
export function onAccent(accent: string): string {
  const ink = '#1f2329';
  return contrast('#ffffff', accent) >= contrast(ink, accent) ? '#ffffff' : ink;
}

export function accentTokens(value: unknown): Record<string, string> {
  const accent = normalizeAccent(value);
  const highlight = mix(accent, '#ffffff', 0.85);
  const text = readableAccent(accent, highlight);
  return {
    '--accent': accent,
    '--accent-hover': mix(accent, '#000000', 0.12),
    '--accent-pressed': mix(accent, '#000000', 0.22),
    '--accent-soft': mix(accent, '#ffffff', 0.91),
    '--accent-text': text,
    '--accent-contrast': onAccent(accent),
    '--accent-tint': rgba(accent, 0.13),
    '--accent-outline': rgba(accent, 0.35),
    '--accent-wash': rgba(accent, 0.07),
    '--nav-active': highlight,
    '--nav-active-text': text,
    '--text-secondary': ensureContrast(mix(MUTED_BASE, accent, 0.18), '#ffffff', 7),
    '--text-tertiary': ensureContrast(mix(FAINT_BASE, accent, 0.2), '#ffffff', 5),
    '--focus-ring': `0 0 0 3px ${rgba(accent, 0.22)}`,
  };
}

export function applyAccent(value: unknown): void {
  const style = document.documentElement.style;
  for (const [name, token] of Object.entries(accentTokens(value))) style.setProperty(name, token);
}

/**
 * Wallpaper display presets. A preset is a whole set of values, never a single
 * number: the white veil over the picture, the panel fill and the frosted
 * controls move together so every preset keeps its text readable. Only the
 * panel opacity is user-editable; the veil stays inside the preset.
 */
export type WallpaperDisplay = 'clear' | 'balanced' | 'vivid';

export interface WallpaperMode {
  id: WallpaperDisplay;
  label: string;
  hint: string;
  /** Default panel opacity, percent. */
  panel: number;
  /** White veil over the wallpaper: [top, bottom] of the gradient, percent. */
  scrim: [number, number];
}

export const WALLPAPER_MODES: WallpaperMode[] = [
  { id: 'clear', label: '清晰', hint: '壁纸更淡，文字最稳', panel: 92, scrim: [48, 62] },
  { id: 'balanced', label: '均衡', hint: '默认，壁纸隐约可见', panel: 72, scrim: [20, 38] },
  { id: 'vivid', label: '透亮', hint: '壁纸更明显，面板更透', panel: 52, scrim: [8, 16] },
];

export function normalizeWallpaperDisplay(value: unknown): WallpaperDisplay {
  const id = typeof value === 'string' ? value : '';
  return (WALLPAPER_MODES.find((mode) => mode.id === id)?.id ?? 'balanced') as WallpaperDisplay;
}

export function wallpaperMode(value: unknown): WallpaperMode {
  const id = normalizeWallpaperDisplay(value);
  return WALLPAPER_MODES.find((mode) => mode.id === id) as WallpaperMode;
}

/** The panel opacity to use for a preset, clamped to what the slider allows. */
export function wallpaperPanelOpacity(
  panels: Record<string, number> | undefined,
  mode: WallpaperMode,
): number {
  const raw = panels?.[mode.id];
  const value = typeof raw === 'number' && Number.isFinite(raw) ? raw : mode.panel;
  return Math.min(100, Math.max(40, Math.round(value)));
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

/**
 * CSS tokens for the wallpaper shell. Text tokens are re-derived against the
 * worst case a panel can sit on — a black picture under the veil and the panel —
 * so secondary text keeps its contrast even behind a dark wallpaper. The targets
 * are the AA text floors, not the 7:1 used on plain white: past a certain panel
 * transparency no darker grey exists, and pure black must still satisfy them.
 */
export function wallpaperTokens(
  mode: WallpaperMode,
  panelPercent: number,
  accentValue: unknown,
): Record<string, string> {
  const accent = normalizeAccent(accentValue);
  const panel = clamp(panelPercent, 40, 100) / 100;
  const veil = (mode.scrim[0] + mode.scrim[1]) / 200;
  const worst = panel + veil * (1 - panel);
  const worstCase = `#${[worst, worst, worst]
    .map((v) =>
      Math.round(v * 255)
        .toString(16)
        .padStart(2, '0'),
    )
    .join('')}`;
  return {
    '--wp-scrim-top': rgba('#ffffff', mode.scrim[0] / 100),
    '--wp-scrim-bottom': rgba('#ffffff', mode.scrim[1] / 100),
    '--panel': rgba('#ffffff', panel),
    '--control-bg': rgba('#ffffff', clamp(panel - 0.17, 0.35, 1)),
    '--control-bg-subtle': rgba('#ffffff', clamp(panel - 0.32, 0.25, 1)),
    '--sidebar-bg': rgba('#f1f2f4', clamp(panel - 0.12, 0.4, 1)),
    // Never lighter than the plain-page token: first tune for white, then darken
    // further if the translucent panel could sit on a dark picture.
    '--text-secondary': ensureContrast(
      ensureContrast(mix(MUTED_BASE, accent, 0.18), '#ffffff', 7),
      worstCase,
      4.5,
    ),
    '--text-tertiary': ensureContrast(
      ensureContrast(mix(FAINT_BASE, accent, 0.2), '#ffffff', 5),
      worstCase,
      3.5,
    ),
  };
}
