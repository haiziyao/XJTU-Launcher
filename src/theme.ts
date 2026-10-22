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

/** Darkens the accent just enough to read as text on the given background. */
export function readableAccent(accent: string, background = '#ffffff'): string {
  let candidate = accent;
  for (let step = 1; step <= 32; step += 1) {
    if (contrast(candidate, background) >= 4.5) return candidate;
    candidate = mix(accent, '#000000', step * 0.04);
  }
  return candidate;
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
    '--nav-active': highlight,
    '--nav-active-text': text,
    '--focus-ring': `0 0 0 3px ${rgba(accent, 0.22)}`,
  };
}

export function applyAccent(value: unknown): void {
  const style = document.documentElement.style;
  for (const [name, token] of Object.entries(accentTokens(value))) style.setProperty(name, token);
}
