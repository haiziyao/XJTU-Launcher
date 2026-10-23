import { invoke } from '@tauri-apps/api/core';

/**
 * Stored campus results. The page shows the last value first and refreshes in
 * the background; entries are scoped to the active account by the backend.
 */
function stable(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
  if (value && typeof value === 'object') {
    const record = value as Record<string, unknown>;
    return `{${Object.keys(record)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${stable(record[key])}`)
      .join(',')}}`;
  }
  return JSON.stringify(value);
}

export function cacheName(name: string, params?: unknown): string {
  return params === undefined ? name : `${name}:${stable(params)}`;
}

export async function readCached<T>(name: string, params?: unknown): Promise<T | null> {
  try {
    const value = await invoke<T | null>('cache_read', { key: cacheName(name, params) });
    return value ?? null;
  } catch {
    // Caching is best effort; a miss behaves like an empty store.
    return null;
  }
}

export async function writeCached(name: string, params: unknown, value: unknown): Promise<void> {
  try {
    await invoke('cache_write', { key: cacheName(name, params), value });
  } catch {
    // Writing must never break the page that just loaded.
  }
}

/** Apply a stored value if there is one; the caller still refreshes after. */
export async function applyCached<T>(
  name: string,
  params: unknown,
  apply: (value: T) => void,
): Promise<boolean> {
  const value = await readCached<T>(name, params);
  if (value === null) return false;
  apply(value);
  return true;
}
