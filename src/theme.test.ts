import { describe, expect, it } from 'vitest';
import { ACCENT_PRESETS, accentTokens, contrast, normalizeAccent, readableAccent } from './theme';

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
