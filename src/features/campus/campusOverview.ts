import { bridge } from '../../shared/bridge';
import { siteQuery } from './campusSession';
import type { LmsOverview, LmsTodos } from '../../types';

// 课程资料 reads the per-course aggregate. The in-memory copy keeps switching
// back to the page from starting a second LMS scan.
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

// 待办 has its own short-lived list from the school's official endpoint. It
// must not ride on the materials scan: that answer is cached for minutes and
// only refreshes correctly on an explicit refresh.
let pendingTodos: Promise<LmsTodos> | null = null;
let storedTodos: { at: number; value: LmsTodos } | null = null;
const TODOS_TTL = 60_000;

export function loadLmsTodos(force = false): Promise<LmsTodos> {
  if (!force && storedTodos && Date.now() - storedTodos.at < TODOS_TTL) {
    return Promise.resolve(storedTodos.value);
  }
  if (!force && pendingTodos) return pendingTodos;
  const own = generation;
  const request = siteQuery<LmsTodos>(
    { type: 'lmsTodos' },
    'lms',
    () => bridge.refreshLmsSession(),
    'lmsTodos',
    force,
  )
    .then((value) => {
      if (own === generation) storedTodos = { at: Date.now(), value };
      return value;
    })
    .finally(() => {
      if (pendingTodos === request) pendingTodos = null;
    });
  pendingTodos = request;
  return request;
}

/** Drop the in-memory copies when the active account changes. */
export function resetLmsCaches() {
  generation += 1;
  stored = null;
  pending = null;
  storedTodos = null;
  pendingTodos = null;
}
