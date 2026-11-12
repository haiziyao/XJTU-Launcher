import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { dismissNotice, noticeDismissed } from './notices';

const store = new Map<string, string>();

beforeEach(() => {
  store.clear();
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => store.get(key) ?? null,
    setItem: (key: string, value: string) => void store.set(key, value),
  });
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('one-time notice strips', () => {
  it('stays dismissed after a close, per notice id', () => {
    expect(noticeDismissed('venue-booking')).toBe(false);
    dismissNotice('venue-booking');
    expect(noticeDismissed('venue-booking')).toBe(true);
    // Other strips are independent decisions.
    expect(noticeDismissed('notice-body')).toBe(false);
  });

  it('treats unavailable storage as "not dismissed"', () => {
    vi.stubGlobal('localStorage', {
      getItem: () => {
        throw new Error('blocked');
      },
      setItem: () => {
        throw new Error('blocked');
      },
    });
    expect(noticeDismissed('venue-booking')).toBe(false);
    expect(() => dismissNotice('venue-booking')).not.toThrow();
  });
});
