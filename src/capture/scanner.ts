/**
 * Replay scanner.
 *
 * Ported from XJTU-PPT-Capture-Extension v3.1.0 (MIT, Copyright (c) 2026
 * MeowSaint and contributors): coarse sampling per mode, a 0.5 s re-scan
 * between two coarse samples that differ, a 0.25 s confirmation before a page
 * is kept, and a review range for frames that keep changing (transition
 * animations). See THIRD_PARTY_NOTICES.md.
 *
 * The video itself is injected through hooks so the scan can be tested with
 * synthetic frames and no DOM.
 */
import { DEFAULT_THRESHOLD, difference, isBlank, isSimilar, type Signature } from './frames';

export type ScanMode = 'balanced' | 'fine' | 'fast';

/** Coarse step per mode; upstream's 均衡 / 细查 / 快速. */
export const SCAN_MODES: Record<ScanMode, { step: number; title: string }> = {
  balanced: { step: 2, title: '均衡' },
  fine: { step: 0.5, title: '细查' },
  fast: { step: 8, title: '快速' },
};

/** Re-scan step used between two coarse samples that differ. */
export const FINE_STEP = 0.5;
/** How far after a candidate the stability check is taken. */
export const CONFIRM_OFFSET = 0.25;
/** Two coarse samples closer than this are not split by a re-scan. */
export const RESCAN_GAP = 0.55;

export interface ScanSettings {
  mode: ScanMode;
  /** 新页差异: percent of changed pixels, default 1.2. */
  threshold: number;
  /** 去除重复页: compare against every page kept in this lesson. */
  dedupe: boolean;
}

export interface ScanSample {
  time: number;
  signature: Signature;
}

export interface ScanCapture extends ScanSample {
  blob: Blob;
  width: number;
  height: number;
}

export interface SuspectRange {
  start: number;
  end: number;
  /** The unstable time point that produced the range. */
  time: number;
}

export interface ScanHooks {
  /** Draw the frame at `time` for comparison; null means blank. */
  sample(time: number): Promise<ScanSample | null>;
  /** Seek back and encode the frame that should be saved. */
  capture(time: number): Promise<ScanCapture>;
  save(capture: ScanCapture): Promise<void>;
  suspect(range: SuspectRange): Promise<void> | void;
  progress(done: number, total: number): void;
  cancelled(): boolean;
  /** Signatures of pages already kept (resume and dedupe). */
  known?: Signature[];
  /** Signature of the last page kept before a resumed scan. */
  baseline?: Signature | null;
  /** Called after every coarse sample so the caller can persist progress. */
  checkpoint?(done: number, total: number, time: number): Promise<void> | void;
}

export interface ScanResult {
  pages: number;
  suspects: SuspectRange[];
  cancelled: boolean;
}

export function settingsFor(
  mode: ScanMode,
  threshold = DEFAULT_THRESHOLD,
  dedupe = true,
): ScanSettings {
  return { mode, threshold, dedupe };
}

/** Merge overlapping or touching review ranges, keeping them in time order. */
export function mergeSuspects(ranges: SuspectRange[]): SuspectRange[] {
  const sorted = [...ranges].sort((a, b) => a.start - b.start || a.end - b.end);
  const merged: SuspectRange[] = [];
  for (const range of sorted) {
    const last = merged[merged.length - 1];
    if (last && range.start <= last.end) {
      last.end = Math.max(last.end, range.end);
      continue;
    }
    merged.push({ ...range });
  }
  return merged;
}

/**
 * Run one scan over `duration` seconds. `startTime` resumes after a checkpoint:
 * the scan continues from that point and keeps the earlier pages via `known`.
 */
export async function scan(
  duration: number,
  settings: ScanSettings,
  hooks: ScanHooks,
  startTime = 0,
): Promise<ScanResult> {
  const step = SCAN_MODES[settings.mode].step;
  const threshold = settings.threshold;
  const total = Math.max(1, Math.ceil(Math.max(0, duration - startTime) / step));
  const history = [...(hooks.known ?? [])];
  const suspects: SuspectRange[] = [];
  let baseline: Signature | null = hooks.baseline ?? history.at(-1) ?? null;
  let previousTime = startTime - step;
  let pages = 0;

  const recordSuspect = async (range: SuspectRange) => {
    suspects.push(range);
    await hooks.suspect(range);
  };

  const consider = async (time: number, sample: ScanSample, limit: number) => {
    // A page that is still changing after CONFIRM_OFFSET is a transition, not
    // a slide: remember the range for review instead of saving it.
    const confirmAt = Math.min(time + CONFIRM_OFFSET, limit);
    if (confirmAt > time + 0.01) {
      const confirm = await hooks.sample(confirmAt);
      if (
        confirm &&
        difference(sample.signature, confirm.signature).changed > Math.min(threshold, 0.8)
      ) {
        await recordSuspect({
          start: Math.max(0, time - FINE_STEP),
          end: Math.min(duration, time + FINE_STEP),
          time,
        });
        return;
      }
    }
    // Dedupe runs after the stability check, so a transition never hides a
    // later real page.
    if (settings.dedupe && history.some((entry) => isSimilar(entry, sample.signature, threshold))) {
      return;
    }
    const capture = await hooks.capture(time);
    await hooks.save(capture);
    history.push(sample.signature);
    pages += 1;
  };

  const rescan = async (from: number, to: number) => {
    for (let time = from + FINE_STEP; time < to - 0.02; time += FINE_STEP) {
      if (hooks.cancelled()) return;
      const sample = await hooks.sample(time);
      if (!sample || isBlank(sample.signature)) continue;
      if (!baseline || isSimilar(sample.signature, baseline, threshold)) continue;
      await consider(time, sample, to);
    }
  };

  for (let index = 0; index < total; index += 1) {
    if (hooks.cancelled()) return { pages, suspects, cancelled: true };
    const time = startTime + index * step;
    const sample = await hooks.sample(time);
    hooks.progress(index + 1, total);
    if (
      sample &&
      !isBlank(sample.signature) &&
      !(baseline && isSimilar(sample.signature, baseline, threshold))
    ) {
      if (baseline && time - previousTime > RESCAN_GAP) await rescan(previousTime, time);
      await consider(time, sample, Math.min(time + step, duration));
      baseline = sample.signature;
      previousTime = time;
    }
    await hooks.checkpoint?.(index + 1, total, Math.min(time + step, duration));
  }

  return { pages, suspects: mergeSuspects(suspects), cancelled: hooks.cancelled() };
}
