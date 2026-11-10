/**
 * Media seeking for the capture loop.
 *
 * A presigned replay address may stall a range request for a long time, so a
 * single seek is not allowed to abort the whole run: callers get a typed
 * timeout they can recover from (re-resolve the address and reload the video)
 * instead of a message that reads like the address is dead.
 */

/** The seek event never arrived within the budget. */
export class SeekTimeout extends Error {
  constructor() {
    super('视频跳转超时，回放地址可能已过期');
    this.name = 'SeekTimeout';
  }
}

export const SEEK_TIMEOUT = 20000;

/** Wait for one media event; rejects on error or timeout. */
export function waitEvent(
  video: HTMLVideoElement,
  event: string,
  message: string,
  timeout = SEEK_TIMEOUT,
): Promise<void> {
  return new Promise((resolve, reject) => {
    let done = false;
    const finish = (action: () => void) => {
      if (done) return;
      done = true;
      window.clearTimeout(timer);
      video.removeEventListener(event, ok);
      video.removeEventListener('error', fail);
      action();
    };
    const ok = () => finish(resolve);
    const fail = () => finish(() => reject(new Error(message)));
    const timer = window.setTimeout(() => finish(() => reject(new SeekTimeout())), timeout);
    video.addEventListener(event, ok);
    video.addEventListener('error', fail);
  });
}

/**
 * Jump to `time` and return the time actually reached. Resolves once the
 * browser has fired `seeked` and has a frame that can be drawn.
 */
export async function seekTo(
  video: HTMLVideoElement,
  time: number,
  timeout = SEEK_TIMEOUT,
): Promise<number> {
  const limit = Number.isFinite(video.duration) ? Math.max(0, video.duration - 0.05) : time;
  const target = Math.max(0, Math.min(time, limit));
  if (Math.abs(video.currentTime - target) < 0.01 && video.readyState >= 2) return target;
  const seeked = waitEvent(video, 'seeked', '视频无法解码，回放地址可能已过期', timeout);
  video.currentTime = target;
  await seeked;
  if (video.readyState < 2) {
    await waitEvent(video, 'loadeddata', '视频无法解码，回放地址可能已过期', timeout);
  }
  return target;
}

/**
 * Let the compositor publish the frame we just seeked to. `requestVideoFrame`
 * fires exactly when a new frame is presented; the timer covers browsers and
 * states where it never fires (for example an occluded window).
 */
export function frameReady(video: HTMLVideoElement): Promise<void> {
  return new Promise((resolve) => {
    if (typeof video.requestVideoFrameCallback !== 'function') {
      window.setTimeout(resolve, 32);
      return;
    }
    let done = false;
    const finish = () => {
      if (done) return;
      done = true;
      window.clearTimeout(timer);
      resolve();
    };
    // A short fallback: `seeked` already means the frame is decoded, so this
    // only covers an occluded window where the callback may not fire.
    const timer = window.setTimeout(finish, 150);
    video.requestVideoFrameCallback(finish);
  });
}
