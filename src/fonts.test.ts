import { describe, expect, it } from 'vitest';
import {
  DEFAULT_FONT_SETTINGS,
  FONT_CATEGORIES,
  FONT_REGIONS,
  applyFontToRegions,
  fontFamilyStack,
  normalizeFontSettings,
} from './fonts';

describe('font family stack', () => {
  it('keeps the bundled stacks for the two built-in ids', () => {
    expect(fontFamilyStack('system', 'sans')).toBe('var(--font-sans)');
    expect(fontFamilyStack('mono', 'mono')).toBe('var(--font-mono)');
  });

  it('quotes downloaded and local family names, including Chinese', () => {
    expect(fontFamilyStack('霞鹜文楷', 'sans')).toBe('"霞鹜文楷", var(--font-sans)');
    expect(fontFamilyStack('Noto Serif SC', 'sans')).toBe('"Noto Serif SC", var(--font-sans)');
  });

  it('strips characters that could break out of the quoted family', () => {
    expect(fontFamilyStack('A"B\\C', 'sans')).toBe('"ABC", var(--font-sans)');
    expect(fontFamilyStack('   ', 'sans')).toBe('var(--font-sans)');
  });
});

describe('applying one font to every region', () => {
  it('changes only the family and keeps the fine tuning', () => {
    const next = applyFontToRegions(DEFAULT_FONT_SETTINGS, '霞鹜文楷');
    for (const region of FONT_REGIONS) {
      expect(next[region.id].family).toBe('霞鹜文楷');
      expect(next[region.id].size).toBe(DEFAULT_FONT_SETTINGS[region.id].size);
      expect(next[region.id].weight).toBe(DEFAULT_FONT_SETTINGS[region.id].weight);
      expect(next[region.id].lineHeight).toBe(DEFAULT_FONT_SETTINGS[region.id].lineHeight);
      expect(next[region.id].letterSpacing).toBe(DEFAULT_FONT_SETTINGS[region.id].letterSpacing);
    }
  });

  it('does not mutate the settings object it was given', () => {
    const before = structuredClone(DEFAULT_FONT_SETTINGS);
    applyFontToRegions(DEFAULT_FONT_SETTINGS, '思源宋体');
    expect(DEFAULT_FONT_SETTINGS).toEqual(before);
  });
});

describe('font categories', () => {
  it('covers every stored region exactly once', () => {
    const ids = FONT_CATEGORIES.flatMap((category) => category.regions);
    expect(new Set(ids).size).toBe(ids.length);
    expect(new Set(ids)).toEqual(new Set(FONT_REGIONS.map((region) => region.id)));
  });
});

describe('normalizing stored settings', () => {
  it('fills missing regions with defaults', () => {
    const next = normalizeFontSettings({ ui: { family: '霞鹜文楷' } });
    expect(next.ui.family).toBe('霞鹜文楷');
    expect(next.ui.size).toBe(DEFAULT_FONT_SETTINGS.ui.size);
    expect(next.code).toEqual(DEFAULT_FONT_SETTINGS.code);
  });

  it('keeps the timetable on the global font but its own numeric defaults', () => {
    const next = normalizeFontSettings({
      ui: { family: '霞鹜文楷' },
      timetable: { family: '楷体', weight: 700 },
    });
    expect(next.timetable.family).toBe('霞鹜文楷');
    expect(next.timetable.weight).toBe(700);
  });
});
