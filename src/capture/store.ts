/**
 * Capture work state, stored by the backend under the campus cache directory
 * for the active account (`cache/<account>/capture/<activity>/`).
 *
 * The page, its settings and the JPEG originals survive a reload so a long
 * lesson can be resumed, while switching or deleting an account removes the
 * whole folder with the rest of that account's cache. Nothing here holds the
 * replay address or a credential.
 *
 * Persistence is best effort: when the backend is unavailable the capture page
 * still works for the current session.
 */
import { invoke } from '@tauri-apps/api/core';
import type { Signature } from './frames';
import type { ScanMode } from './scanner';

export interface StoredSession {
  id: string;
  activity: number;
  course: string;
  title: string;
  start: string;
  mode: ScanMode;
  threshold: number;
  dedupe: boolean;
  roi: { x: number; y: number; w: number; h: number } | null;
  duration: number;
  checkpoint: number;
  total: number;
  suspects: { start: number; end: number; time: number }[];
  updated: number;
}

export interface StoredPage {
  id: number;
  order: number;
  time: number;
  width: number;
  height: number;
  signature: Signature;
}

export interface RestoredPage {
  page: StoredPage;
  blob: Blob;
}

export interface RestoredSession {
  session: StoredSession;
  pages: RestoredPage[];
}

function base64(bytes: Uint8Array): string {
  const chunks: string[] = [];
  const size = 0x8000;
  for (let offset = 0; offset < bytes.length; offset += size) {
    chunks.push(String.fromCharCode(...bytes.subarray(offset, offset + size)));
  }
  return btoa(chunks.join(''));
}

function fromBase64(value: string): Uint8Array<ArrayBuffer> {
  const raw = atob(value);
  const bytes = new Uint8Array(new ArrayBuffer(raw.length));
  for (let index = 0; index < raw.length; index += 1) bytes[index] = raw.charCodeAt(index);
  return bytes;
}

export async function saveSession(activity: number, session: StoredSession): Promise<void> {
  await invoke('capture_session_save', { activity, session });
}

export async function loadSession(activity: number): Promise<RestoredSession | null> {
  const stored = await invoke<{ session: StoredSession; pages: StoredPage[] } | null>(
    'capture_session_load',
    { activity },
  );
  if (!stored) return null;
  const pages: RestoredPage[] = [];
  if (stored.pages.length) {
    const images = await invoke<{ id: number; data: string }[]>('capture_pages_read', {
      activity,
      ids: stored.pages.map((page) => page.id),
    });
    const blobs = new Map(
      images.map((image) => [image.id, new Blob([fromBase64(image.data)], { type: 'image/jpeg' })]),
    );
    for (const page of [...stored.pages].sort((a, b) => a.order - b.order)) {
      const blob = blobs.get(page.id);
      if (blob) pages.push({ page, blob });
    }
  }
  return { session: stored.session, pages };
}

export async function appendPage(
  activity: number,
  order: number,
  page: Omit<StoredPage, 'id' | 'order'>,
  blob: Blob,
): Promise<number | null> {
  const data = base64(new Uint8Array(await blob.arrayBuffer()));
  const added = await invoke<StoredPage>('capture_page_add', {
    activity,
    page: { ...page, order },
    data,
  });
  return added.id;
}

export async function deletePages(activity: number, ids: number[]): Promise<void> {
  if (!ids.length) return;
  await invoke('capture_pages_delete', { activity, ids });
}

export async function updateOrder(
  activity: number,
  entries: { id: number; order: number }[],
): Promise<void> {
  if (!entries.length) return;
  await invoke('capture_pages_order', { activity, orders: entries });
}

export async function clearSession(activity: number): Promise<void> {
  await invoke('capture_session_clear', { activity });
}
