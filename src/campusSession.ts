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
export async function directQuery<T>(query: SchoolQuery): Promise<T> {
  try {
    return (await bridge.schoolQuery(query)) as T;
  } catch (error) {
    if (!expired(error)) throw error;
    await ensureDirectSession();
    return (await bridge.schoolQuery(query)) as T;
  }
}
