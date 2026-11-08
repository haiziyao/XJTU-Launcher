/**
 * Frame comparison for replay capture.
 *
 * Derived from XJTU-PPT-Capture-Extension v3.1.0 (MIT, Copyright (c) 2026
 * MeowSaint and contributors), rewritten as pure functions. See
 * THIRD_PARTY_NOTICES.md.
 *
 * Frames are downscaled to 192×108 and compared per channel. A constant
 * brightness offset (for example auto-exposure drift) is removed first, then a
 * pixel counts as changed when any channel moves by 24 or more. The result is
 * reported as the percentage of changed pixels, which is what the 新页差异
 * setting (default 1.2%) refers to.
 */
export const ANALYSIS_WIDTH = 192;
export const ANALYSIS_HEIGHT = 108;
export const ANALYSIS_PIXELS = ANALYSIS_WIDTH * ANALYSIS_HEIGHT;

/** Default share of changed pixels that marks a new slide, in percent. */
export const DEFAULT_THRESHOLD = 1.2;
/** Above this per-channel offset the difference is content, not lighting. */
export const BIAS_LIMIT = 8;
/** A channel change of at least this many levels counts as a changed pixel. */
export const PIXEL_DELTA = 24;
/** Mean change below this keeps two frames similar even on a busy slide. */
export const MEAN_LIMIT = 0.25;
/** Luminance spread under this is a black/white/fade frame with no slide. */
export const BLANK_SPREAD = 8;
/** Upstream stops one lesson at 1000 pages; keep the same hard limit. */
export const MAX_FRAMES = 1000;

/** RGBA pixels at ANALYSIS_WIDTH × ANALYSIS_HEIGHT. */
export type Signature = Uint8ClampedArray;

export interface FrameDifference {
  /** Percentage of pixels that changed by PIXEL_DELTA or more. */
  changed: number;
  /** Mean absolute channel change, in percent of the full 0..255 range. */
  mean: number;
}

/** Wrap RGBA pixels read from the analysis canvas. */
export function signature(pixels: Uint8ClampedArray): Signature {
  return pixels.slice();
}

/** Luminance of one pixel. */
function luma(r: number, g: number, b: number): number {
  return 0.299 * r + 0.587 * g + 0.114 * b;
}

/** Compare two signatures, removing a uniform per-channel brightness offset. */
export function difference(left: Signature, right: Signature): FrameDifference {
  const pixels = Math.min(left.length, right.length) >> 2;
  if (!pixels) return { changed: 0, mean: 0 };
  let sumR = 0;
  let sumG = 0;
  let sumB = 0;
  for (let index = 0; index < pixels * 4; index += 4) {
    sumR += left[index] - right[index];
    sumG += left[index + 1] - right[index + 1];
    sumB += left[index + 2] - right[index + 2];
  }
  const bias = (sum: number) => {
    const value = sum / pixels;
    return Math.abs(value) <= BIAS_LIMIT ? value : 0;
  };
  const biasR = bias(sumR);
  const biasG = bias(sumG);
  const biasB = bias(sumB);
  let changed = 0;
  let total = 0;
  for (let index = 0; index < pixels * 4; index += 4) {
    const dr = Math.abs(left[index] - right[index] - biasR);
    const dg = Math.abs(left[index + 1] - right[index + 1] - biasG);
    const db = Math.abs(left[index + 2] - right[index + 2] - biasB);
    if (Math.max(dr, dg, db) >= PIXEL_DELTA) changed += 1;
    total += dr + dg + db;
  }
  return {
    changed: (changed * 100) / pixels,
    mean: (total * 100) / (pixels * 765),
  };
}

/** Whether two frames show the same slide at the given 新页差异 threshold. */
export function isSimilar(
  left: Signature | null,
  right: Signature | null,
  threshold = DEFAULT_THRESHOLD,
): boolean {
  if (!left || !right) return false;
  const { changed, mean } = difference(left, right);
  return changed < threshold && mean < Math.max(MEAN_LIMIT, threshold / 2);
}

/** Nearly uniform frames carry no slide content (intro cards, fades). */
export function isBlank(frame: Signature): boolean {
  let low = 255;
  let high = 0;
  for (let index = 0; index < frame.length; index += 4) {
    const value = luma(frame[index], frame[index + 1], frame[index + 2]);
    if (value < low) low = value;
    if (value > high) high = value;
  }
  return high - low < BLANK_SPREAD;
}

/**
 * 清理相似页: merge neighbouring pages left by per-line animation. The
 * threshold is `max(2.5 × 新页差异, 3)` percent, and the last page of a run is
 * kept because it shows the finished slide.
 */
export function isNearDuplicate(
  left: Signature,
  right: Signature,
  threshold = DEFAULT_THRESHOLD,
): boolean {
  return difference(left, right).changed < Math.max(2.5 * threshold, 3);
}
