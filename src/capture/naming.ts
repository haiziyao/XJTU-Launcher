/**
 * Export names for capture results, following XJTU-PPT-Capture-Extension
 * v3.1.0 (MIT, Copyright (c) 2026 MeowSaint and contributors):
 *
 *   课程名-YYYY-MM-DD-HH-MM-SS.pdf
 *   001-HH-mm-ss.jpg
 *
 * Dates are campus wall time (UTC+8); a bare `2026-09-15 10:10:00` is read as
 * Beijing time rather than as the machine's local time.
 */
import { CAMPUS_OFFSET_MS, campusTimestamp } from '../domain';

const pad = (value: number, width = 2) => String(value).padStart(width, '0');

/** Strip characters a file system rejects plus the trailing student number. */
export function courseName(raw: string): string {
  return (
    raw
      .replace(/\s*[（(]\d{6,}[)）]\s*$/u, '')
      .replace(/[\\/:*?"<>|\u0000-\u001f]/g, ' ')
      .replace(/\s+/g, ' ')
      .trim()
      .slice(0, 100) || '回放截取'
  );
}

/** `YYYY-MM-DD-HH-MM-SS` in campus time, or '' when the timestamp is unusable. */
export function exportStamp(start: string): string {
  const ms = campusTimestamp(start);
  if (!Number.isFinite(ms)) return '';
  const date = new Date(ms + CAMPUS_OFFSET_MS);
  return [
    date.getUTCFullYear(),
    pad(date.getUTCMonth() + 1),
    pad(date.getUTCDate()),
    pad(date.getUTCHours()),
    pad(date.getUTCMinutes()),
    pad(date.getUTCSeconds()),
  ].join('-');
}

/** `课程名-2026-09-15-10-10-00.pdf`; empty stamp falls back to the course alone. */
export function exportBaseName(course: string, start: string): string {
  const name = courseName(course);
  const stamp = exportStamp(start);
  return stamp ? `${name}-${stamp}` : name;
}

/** `001-HH-mm-ss.jpg` from the position inside the replay. */
export function slideName(index: number, time: number): string {
  const total = Math.max(0, Math.floor(time));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = total % 60;
  return `${pad(index + 1, 3)}-${pad(hours)}-${pad(minutes)}-${pad(seconds)}.jpg`;
}
