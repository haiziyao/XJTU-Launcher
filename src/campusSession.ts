import { bridge } from './bridge';
import type { SchoolQuery } from './types';

// Services such as the campus card reject WebVPN sessions, so they need a CAS
// session of their own before the first query. Concurrent opens share one login;
// a forced renewal is used when an existing direct session has expired.
const pending = new Map<boolean, Promise<void>>();

export function ensureDirectSession(force = false): Promise<void> {
  let flight = pending.get(force);
  if (!flight) {
    flight = bridge
      .refreshDirectSession(force)
      .then(() => undefined)
      .finally(() => pending.delete(force));
    pending.set(force, flight);
  }
  return flight;
}

const expired = (error: unknown) =>
  /失效|重新登录/.test(error instanceof Error ? error.message : String(error));

/** Query a direct-only service, renewing its session once when it expired. */
export async function directQuery<T>(
  query: SchoolQuery,
  cache?: string,
  force = false,
): Promise<T> {
  try {
    return await read<T>(query, cache, force);
  } catch (error) {
    if (!expired(error)) throw error;
    await ensureDirectSession(true);
    return await read<T>(query, cache, force);
  }
}

// Sites such as 思源学堂 and the venue system log in separately from the
// academic system, so the first query of a session may have to sign in with
// the saved password. Concurrent opens of one service share that login.
const sitePending = new Map<string, Promise<void>>();
const missingSession = (error: unknown) =>
  /请先完成|请先打开|失效|重新登录|未登录|非预期数据/.test(
    error instanceof Error ? error.message : String(error),
  );

// Read-only queries are safe to repeat. The campus services occasionally drop
// a connection or answer 502; retrying twice turns a transient failure into a
// normal load instead of an error page.
const transient = (error: unknown) =>
  /无法连接|暂不可用|超时|网络|连接失败|非预期数据/.test(
    error instanceof Error ? error.message : String(error),
  );
const wait = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

async function read<T>(query: SchoolQuery, cache?: string, force = false): Promise<T> {
  let last: unknown;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    if (attempt) await wait(attempt === 1 ? 500 : 1200);
    try {
      return (await bridge.schoolQuery(query, cache, force)) as T;
    } catch (error) {
      last = error;
      // A missing/expired session needs a login, not another identical request.
      if (!transient(error) || missingSession(error)) throw error;
    }
  }
  throw last;
}

export async function siteQuery<T>(
  query: SchoolQuery,
  site: string,
  refresh: () => Promise<boolean>,
  cache?: string,
  force = false,
): Promise<T> {
  try {
    return await read<T>(query, cache, force);
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
    return await read<T>(query, cache, force);
  }
}
