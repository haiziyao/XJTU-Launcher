import { bridge } from './bridge';
import type { SchoolQuery } from './types';

// Services such as the campus card reject WebVPN sessions, so they need a CAS
// session of their own before the first query. Concurrent opens share one login.
let pending: Promise<void> | null = null;

export function ensureDirectSession(): Promise<void> {
  if (!pending) {
    pending = bridge
      .refreshDirectSession()
      .then(() => undefined)
      .finally(() => {
        pending = null;
      });
  }
  return pending;
}

const expired = (error: unknown) =>
  /失效|重新登录/.test(error instanceof Error ? error.message : String(error));

/** Query a direct-only service, renewing its session once when it expired. */
export async function directQuery<T>(query: SchoolQuery, cache?: string): Promise<T> {
  try {
    return (await bridge.schoolQuery(query, cache)) as T;
  } catch (error) {
    if (!expired(error)) throw error;
    await ensureDirectSession();
    return (await bridge.schoolQuery(query, cache)) as T;
  }
}

// Sites such as 思源学堂 and the venue system log in separately from the
// academic system, so the first query of a session may have to sign in with
// the saved password. Concurrent opens of one service share that login.
const sitePending = new Map<string, Promise<void>>();
const missingSession = (error: unknown) =>
  /请先完成|失效/.test(error instanceof Error ? error.message : String(error));

export async function siteQuery<T>(
  query: SchoolQuery,
  site: string,
  refresh: () => Promise<boolean>,
  cache?: string,
): Promise<T> {
  try {
    return (await bridge.schoolQuery(query, cache)) as T;
  } catch (error) {
    if (!missingSession(error)) throw error;
    let pending = sitePending.get(site);
    if (!pending) {
      pending = refresh()
        .then(() => undefined)
        .finally(() => sitePending.delete(site));
      sitePending.set(site, pending);
    }
    await pending;
    return (await bridge.schoolQuery(query, cache)) as T;
  }
}
