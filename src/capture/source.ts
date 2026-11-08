/**
 * Which replay stream to open first.
 *
 * A lesson usually has two streams: `INSTRUCTOR` is the classroom camera and
 * `ENCODER` is the screen recording. The slides are only in the screen stream,
 * so capture starts there; sound and the camera stay one click away.
 */
import type { LmsReplayVideo } from '../types';

export function defaultVideoIndex(videos: LmsReplayVideo[]): number {
  const screen = videos.findIndex((video) => video.label.toUpperCase().includes('ENCODER'));
  if (screen >= 0) return screen;
  const audible = videos.findIndex((video) => video.bestAudio);
  return audible >= 0 ? audible : 0;
}
