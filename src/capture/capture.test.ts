import { describe, expect, it } from 'vitest';
import {
  ANALYSIS_PIXELS,
  difference,
  isBlank,
  isNearDuplicate,
  isSimilar,
  signature,
} from './frames';
import { buildNotesPdf, buildPdf, buildZip, crc32 } from './export';
import { courseName, exportBaseName, slideName } from './naming';
import { mergeSuspects, scan, type ScanHooks, type ScanSettings } from './scanner';
import { defaultVideoIndex } from './source';
import type { LmsReplayVideo } from '../types';

function stream(label: string, bestAudio = false): LmsReplayVideo {
  return {
    label,
    size: 1,
    mute: !bestAudio,
    bestAudio,
    downloadUrl: `https://example.test/${label}`,
  };
}

/** A textured frame; a solid colour would count as blank. */
function frame(value: number): Uint8ClampedArray {
  const pixels = new Uint8ClampedArray(ANALYSIS_PIXELS * 4);
  for (let index = 0; index < pixels.length; index += 4) {
    const level = value + (((index >> 2) * 37) % 40) - 20;
    pixels[index] = level;
    pixels[index + 1] = level;
    pixels[index + 2] = level;
    pixels[index + 3] = 255;
  }
  return pixels;
}

function solid(value: number): Uint8ClampedArray {
  const pixels = new Uint8ClampedArray(ANALYSIS_PIXELS * 4);
  for (let index = 0; index < pixels.length; index += 4) {
    pixels[index] = value;
    pixels[index + 1] = value;
    pixels[index + 2] = value;
    pixels[index + 3] = 255;
  }
  return pixels;
}

describe('frame comparison', () => {
  it('measures changed pixels and ignores a uniform brightness offset', () => {
    const base = signature(frame(100));
    expect(base).toHaveLength(ANALYSIS_PIXELS * 4);
    expect(difference(base, signature(frame(100)))).toEqual({ changed: 0, mean: 0 });
    expect(difference(base, signature(frame(106))).changed).toBe(0);
    expect(difference(base, signature(frame(160))).changed).toBe(100);
  });

  it('keeps a busy slide out of the blank check and skips fades', () => {
    expect(isBlank(signature(solid(255)))).toBe(true);
    expect(isBlank(signature(solid(0)))).toBe(true);
    expect(isBlank(signature(frame(120)))).toBe(false);
  });

  it('uses the 新页差异 percentage for similarity and similar-page cleanup', () => {
    const slide = signature(frame(100));
    const other = signature(frame(160));
    expect(isSimilar(slide, other, 1.2)).toBe(false);
    expect(isSimilar(slide, signature(frame(104)), 1.2)).toBe(true);
    expect(isNearDuplicate(slide, other)).toBe(false);
    const slight = signature(frame(104));
    expect(isNearDuplicate(slide, slight)).toBe(true);
  });
});

/**
 * Synthetic player: a signature per video second, returned for any time.
 * `valueAt` decides what the screen shows; capture returns the same frame.
 */
function player(valueAt: (time: number) => number) {
  const captured: number[] = [];
  const suspects: { start: number; end: number; time: number }[] = [];
  const progress: [number, number][] = [];
  const hooks: ScanHooks = {
    async sample(time) {
      return { time, signature: signature(frame(valueAt(time))) };
    },
    async capture(time) {
      return {
        time,
        signature: signature(frame(valueAt(time))),
        blob: new Blob(),
        width: 1280,
        height: 720,
      };
    },
    async save(capture) {
      captured.push(capture.time);
    },
    suspect(range) {
      suspects.push(range);
    },
    progress(done, total) {
      progress.push([done, total]);
    },
    cancelled: () => false,
  };
  return { hooks, captured, suspects, progress };
}

const balanced: ScanSettings = { mode: 'balanced', threshold: 1.2, dedupe: true };

describe('replay scanner', () => {
  it('saves one page per stable slide and re-scans between coarse samples', async () => {
    const value = (time: number) => (time < 4 ? 100 : 200);
    const run = player(value);
    const result = await scan(10, balanced, run.hooks);
    expect(run.captured).toEqual([0, 4]);
    expect(result.pages).toBe(2);
    expect(result.suspects).toEqual([]);
    expect(run.suspects).toEqual([]);
    expect(run.progress.at(-1)).toEqual([5, 5]);
  });

  it('records a review range for frames that keep changing instead of saving them', async () => {
    const value = (time: number) => {
      if (time < 2) return 100;
      if (time < 4) return 150 + (Math.floor(time * 4) % 4) * 30;
      return 200;
    };
    const run = player(value);
    const result = await scan(8, balanced, run.hooks);
    expect(result.suspects.length).toBeGreaterThan(0);
    expect(run.captured).not.toContain(2);
    expect(run.captured.at(-1)).toBe(4);
  });

  it('drops a slide that was already kept when dedupe is on', async () => {
    const value = (time: number) => (time < 4 ? 100 : time < 8 ? 200 : 100);
    const on = player(value);
    expect((await scan(10, balanced, on.hooks)).pages).toBe(2);
    const off = player(value);
    expect((await scan(10, { ...balanced, dedupe: false }, off.hooks)).pages).toBe(3);
  });

  it('merges overlapping review ranges', () => {
    expect(
      mergeSuspects([
        { start: 1, end: 3, time: 2 },
        { start: 2.5, end: 4, time: 3 },
        { start: 8, end: 9, time: 8.5 },
      ]),
    ).toEqual([
      { start: 1, end: 4, time: 2 },
      { start: 8, end: 9, time: 8.5 },
    ]);
  });
});

describe('replay stream choice', () => {
  it('starts on the screen stream, not the camera', () => {
    expect(defaultVideoIndex([stream('INSTRUCTOR'), stream('ENCODER', true)])).toBe(1);
    expect(defaultVideoIndex([stream('encoder'), stream('INSTRUCTOR')])).toBe(0);
  });

  it('falls back to the stream with audio, then the first one', () => {
    expect(defaultVideoIndex([stream('INSTRUCTOR'), stream('SLIDES', true)])).toBe(1);
    expect(defaultVideoIndex([stream('INSTRUCTOR')])).toBe(0);
    expect(defaultVideoIndex([])).toBe(0);
  });
});

describe('export writers', () => {
  it('computes the standard CRC-32 check value', () => {
    expect(crc32(new TextEncoder().encode('123456789'))).toBe(0xcbf43926);
  });

  it('builds a store ZIP with an end record', () => {
    const zip = buildZip([
      { name: '001.jpg', data: new Uint8Array([1, 2, 3, 4]) },
      { name: '002.jpg', data: new Uint8Array([5, 6]) },
    ]);
    const view = new DataView(zip.buffer, zip.byteOffset, zip.byteLength);
    expect(view.getUint32(0, true)).toBe(0x04034b50);
    const end = zip.length - 22;
    expect(view.getUint32(end, true)).toBe(0x06054b50);
    expect(view.getUint16(end + 10, true)).toBe(2);
    expect(view.getUint32(end + 12, true)).toBeGreaterThan(0);
  });

  it('builds the A4 note handout with two slides per page', () => {
    const page = { data: new Uint8Array([0xff, 0xd8, 0xff, 0xd9]), width: 1280, height: 720 };
    const pdf = buildNotesPdf([page, page, page]);
    const text = new TextDecoder('latin1').decode(pdf);
    expect(text.startsWith('%PDF-1.4')).toBe(true);
    expect(text).toContain('/MediaBox [0 0 595 842]');
    expect(text).toContain('/Count 2');
    expect(text).toContain('(3) Tj');
    const start = Number(/startxref\n(\d+)/.exec(text)?.[1]);
    expect(text.slice(start, start + 4)).toBe('xref');
  });

  it('builds a PDF with one page per image and a valid xref', () => {
    const pdf = buildPdf([
      { data: new Uint8Array([0xff, 0xd8, 0xff, 0xd9]), width: 1280, height: 720 },
      { data: new Uint8Array([0xff, 0xd8, 0xff, 0xd9]), width: 1280, height: 720 },
    ]);
    const text = new TextDecoder('latin1').decode(pdf);
    expect(text.startsWith('%PDF-1.4')).toBe(true);
    expect(text).toContain('/Count 2');
    expect(text.trimEnd().endsWith('%%EOF')).toBe(true);
    const start = Number(/startxref\n(\d+)/.exec(text)?.[1]);
    expect(text.slice(start, start + 4)).toBe('xref');
    expect(text).toContain('/Size 9');
  });
});

describe('export naming', () => {
  it('reads a bare lesson time as Beijing time and drops the student number', () => {
    expect(exportBaseName('传感网络(2024123456)', '2026-09-15 10:10:00')).toBe(
      '传感网络-2026-09-15-10-10-00',
    );
    expect(exportBaseName('  离散数学  ', '')).toBe('离散数学');
  });

  it('sanitizes path characters and names slides by replay position', () => {
    expect(courseName('a/b:c*d?')).toBe('a b c d');
    expect(slideName(0, 3725)).toBe('001-01-02-05.jpg');
    expect(slideName(41, 4)).toBe('042-00-00-04.jpg');
  });
});

describe('replay scanner resume', () => {
  it('continues after a checkpoint without rescanning earlier seconds', async () => {
    const value = (time: number) => (time < 4 ? 100 : 200);
    const run = player(value);
    const checkpoints: number[] = [];
    run.hooks.known = [signature(frame(100))];
    run.hooks.baseline = signature(frame(100));
    run.hooks.checkpoint = (done, total, time) => {
      checkpoints.push(time);
    };
    const result = await scan(10, balanced, run.hooks, 4);
    expect(run.captured).toEqual([4]);
    expect(result.pages).toBe(1);
    expect(checkpoints.at(-1)).toBe(10);
  });
});
