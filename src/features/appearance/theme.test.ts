import { describe, expect, it } from 'vitest';
import {
  ACCENT_PRESETS,
  WALLPAPER_MODES,
  accentTokens,
  contrast,
  normalizeAccent,
  normalizeWallpaperDisplay,
  readableAccent,
  wallpaperMode,
  wallpaperPanelOpacity,
  wallpaperTokens,
} from './theme';
import type { WallpaperMode } from './theme';

describe('normalizeAccent', () => {
  it('maps legacy names and lower-cases hex colours', () => {
    expect(normalizeAccent('blue')).toBe('#1f6fe5');
    expect(normalizeAccent('sage')).toBe('#2c8653');
    expect(normalizeAccent('#0F8F86')).toBe('#0f8f86');
  });

  it('falls back to the default for anything else', () => {
    expect(normalizeAccent('not-a-colour')).toBe('#1f6fe5');
    expect(normalizeAccent('#12345')).toBe('#1f6fe5');
    expect(normalizeAccent(undefined)).toBe('#1f6fe5');
  });
});

describe('wallpaper display presets', () => {
  // The shell paints wallpaper → white veil → panel. Over a black picture that
  // lands at white with alpha `panel + veil * (1 - panel)`.
  function overBlack(panel: number, veil: number): string {
    const channel = Math.round(255 * (panel + (veil / 100) * (1 - panel)));
    return `#${channel.toString(16).padStart(2, '0').repeat(3)}`;
  }

  it('falls back to balanced for anything unknown', () => {
    expect(normalizeWallpaperDisplay('vivid')).toBe('vivid');
    expect(normalizeWallpaperDisplay('clear')).toBe('clear');
    expect(normalizeWallpaperDisplay('neon')).toBe('balanced');
    expect(normalizeWallpaperDisplay(undefined)).toBe('balanced');
    expect(wallpaperMode('nope').id).toBe('balanced');
  });

  it('lets the picture show more from clear to vivid', () => {
    const [clear, balanced, vivid] = ['clear', 'balanced', 'vivid'].map((id) =>
      wallpaperMode(id),
    ) as [WallpaperMode, WallpaperMode, WallpaperMode];
    expect(clear.panel).toBeGreaterThan(balanced.panel);
    expect(balanced.panel).toBeGreaterThan(vivid.panel);
    expect(clear.scrim[0]).toBeGreaterThan(vivid.scrim[0]);
    expect(clear.scrim[1]).toBeGreaterThan(vivid.scrim[1]);
    expect(WALLPAPER_MODES.every((mode) => mode.label && mode.hint)).toBe(true);
  });

  it('clamps the stored opacity to what the slider allows', () => {
    const balanced = wallpaperMode('balanced');
    expect(wallpaperPanelOpacity({ balanced: 66 }, balanced)).toBe(66);
    expect(wallpaperPanelOpacity({ balanced: 10 }, balanced)).toBe(40);
    expect(wallpaperPanelOpacity({ balanced: 400 }, balanced)).toBe(100);
    expect(wallpaperPanelOpacity({ balanced: 61.4 }, balanced)).toBe(61);
    expect(wallpaperPanelOpacity({}, balanced)).toBe(balanced.panel);
    expect(wallpaperPanelOpacity({ balanced: Number.NaN }, balanced)).toBe(balanced.panel);
    expect(wallpaperPanelOpacity(undefined, balanced)).toBe(balanced.panel);
  });

  it('keeps secondary and tertiary text readable over a dark picture', () => {
    for (const mode of WALLPAPER_MODES) {
      // The loosest setting the slider allows is the worst backdrop a panel sees.
      const tokens = wallpaperTokens(mode, 40, '#1f6fe5');
      const veil = (mode.scrim[0] + mode.scrim[1]) / 2;
      const backdrop = overBlack(0.4, veil);
      expect(contrast(tokens['--text-secondary'], backdrop)).toBeGreaterThanOrEqual(4.5);
      expect(contrast(tokens['--text-tertiary'], backdrop)).toBeGreaterThanOrEqual(3.5);
      // And on plain white it never comes out lighter than the page tokens.
      expect(contrast(tokens['--text-secondary'], '#ffffff')).toBeGreaterThanOrEqual(7);
      expect(contrast(tokens['--text-tertiary'], '#ffffff')).toBeGreaterThanOrEqual(5);
    }
  });

  it('moves the veil, panel and controls with the preset', () => {
    const clear = wallpaperTokens(wallpaperMode('clear'), wallpaperMode('clear').panel, '#1f6fe5');
    const vivid = wallpaperTokens(wallpaperMode('vivid'), wallpaperMode('vivid').panel, '#1f6fe5');
    expect(clear['--wp-scrim-top']).not.toBe(vivid['--wp-scrim-top']);
    expect(clear['--panel']).not.toBe(vivid['--panel']);
    expect(clear['--control-bg']).not.toBe(vivid['--control-bg']);
    expect(clear['--panel']).toBe('rgba(255, 255, 255, 0.92)');
    expect(vivid['--panel']).toBe('rgba(255, 255, 255, 0.52)');
  });
});

describe('accentTokens', () => {
  it('keeps every preset readable on the selected highlight and on the accent fill', () => {
    for (const preset of ACCENT_PRESETS) {
      const tokens = accentTokens(preset.value);
      expect(tokens['--accent']).toBe(preset.value);
      expect(contrast(tokens['--nav-active-text'], tokens['--nav-active'])).toBeGreaterThanOrEqual(
        4.5,
      );
      expect(contrast(tokens['--accent-contrast'], preset.value)).toBeGreaterThanOrEqual(3);
    }
  });

  it('tints muted text with the accent and keeps it readable', () => {
    const blue = accentTokens('#1f6fe5');
    const rose = accentTokens('#c93a6b');
    expect(blue['--text-secondary']).not.toBe(rose['--text-secondary']);
    for (const preset of ACCENT_PRESETS) {
      const tokens = accentTokens(preset.value);
      expect(contrast(tokens['--text-secondary'], '#ffffff')).toBeGreaterThanOrEqual(7);
      expect(contrast(tokens['--text-tertiary'], '#ffffff')).toBeGreaterThanOrEqual(5);
    }
  });

  it('darkens text and picks dark ink for a very light accent', () => {
    const tokens = accentTokens('#f5f5f5');
    expect(tokens['--accent-contrast']).toBe('#1f2329');
    expect(contrast(tokens['--nav-active-text'], tokens['--nav-active'])).toBeGreaterThanOrEqual(
      4.5,
    );
  });

  it('keeps white on the default accent', () => {
    expect(accentTokens('#1f6fe5')['--accent-contrast']).toBe('#ffffff');
    expect(readableAccent('#1f6fe5')).toMatch(/^#[0-9a-f]{6}$/);
  });
});
