import { bridge } from './bridge';
import { siteQuery } from './campusSession';
import type { LmsOverview } from './types';

// 待办 and 资料库 read the same aggregate. Sharing the in-flight request keeps
// switching between the two pages from starting a second LMS scan.
let pending: Promise<LmsOverview> | null = null;
let stored: { at: number; value: LmsOverview } | null = null;
let generation = 0;
const TTL = 30_000;

export function loadLmsOverview(force = false): Promise<LmsOverview> {
  if (!force && stored && Date.now() - stored.at < TTL) return Promise.resolve(stored.value);
  if (!force && pending) return pending;
  const own = generation;
  const request = siteQuery<LmsOverview>(
    { type: 'lmsOverview' },
    'lms',
    () => bridge.refreshLmsSession(),
    'lmsOverview',
    force,
  )
    .then((value) => {
      // A reset means the account changed; never keep the old answer for the
      // next account, even if this request finishes afterwards.
      if (own === generation) stored = { at: Date.now(), value };
      return value;
    })
    .finally(() => {
      if (pending === request) pending = null;
    });
  pending = request;
  return request;
}

/** Drop the in-memory copy when the active account changes. */
export function resetLmsOverview() {
  generation += 1;
  stored = null;
  pending = null;
}
