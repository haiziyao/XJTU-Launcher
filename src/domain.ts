import type { AppConfig, Course, Plugin, Timetable } from './types';
import { DEFAULT_ACCENT } from './theme';
import { DEFAULT_PET } from './pets';

export const DEFAULT_SIDEBAR_WIDTH = 140;

export const defaultConfig = (): AppConfig => ({
  version: 1,
  sidebarCollapsed: false,
  sidebarWidth: DEFAULT_SIDEBAR_WIDTH,
  nickname: '',
  avatar: null,
  wallpaper: null,
  reduceMotion: false,
  showSeconds: true,
  accent: DEFAULT_ACCENT,
  pet: DEFAULT_PET,
  greeting: '把校园，放在手边。',
  widgets: ['timetable', 'links', 'plugins'],
  pluginOrder: [],
  disabledPlugins: [],
  hiddenPlugins: [],
  pluginServer: null,
  checkPluginUpdates: true,
  checkAppUpdates: true,
  campusPinned: [],
  homeClock: 'plain',
  homeSections: ['greeting', 'clock', 'timetable', 'links', 'plugins'],
  mcpServerEnabled: false,
  mcpServerPort: 39217,
  mcpServerDisabledTools: [],
  petDesktopEnabled: true,
  petCharacter: 'deepseek-girl',
  petBarStyle: 'glass-light',
  petBarHidden: false,
  petBubbleEnabled: true,
  petSources: ['campus', 'agent', 'update'],
  petPosition: null,
  pageTabSurfaces: false,
});

export const campusLinks = [
  { title: '一网通办', description: '校园办事与服务', url: 'https://ywtb.xjtu.edu.cn' },
  { title: '教务系统', description: '教学与学业信息', url: 'https://jwxt.xjtu.edu.cn' },
  { title: '思源学堂', description: '课程与学习资源', url: 'https://lms.xjtu.edu.cn' },
  { title: '图书馆', description: '文献检索与借阅', url: 'https://lib.xjtu.edu.cn' },
];

export function orderedPlugins(plugins: Plugin[], config: AppConfig): Plugin[] {
  const positions = new Map(config.pluginOrder.map((id, i) => [id, i]));
  return [...plugins].sort(
    (a, b) =>
      (positions.get(a.id) ?? Number.MAX_SAFE_INTEGER) -
        (positions.get(b.id) ?? Number.MAX_SAFE_INTEGER) || a.id.localeCompare(b.id),
  );
}

// Use local dates, so a UTC offset does not move a course to the previous day.
export function academicWeek(start: string, date = new Date()): number {
  const parts = start.slice(0, 10).split('-').map(Number);
  if (parts.length !== 3 || parts.some((n) => !Number.isFinite(n))) return 0;
  const beginning = Date.UTC(parts[0], parts[1] - 1, parts[2]);
  const today = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  return Math.floor((today - beginning) / 604800000) + 1;
}

// Same course always gets the same soft color.
export function courseHue(name: string): number {
  let sum = 0;
  for (const ch of name) sum = (sum + ch.charCodeAt(0)) % 997;
  return sum % 6;
}

/**
 * Reads the week text the school writes ("1-16,18", "1-8单"). It mirrors Rust
 * `campus::parse_weeks`, so the editor accepts exactly what the backend stores.
 */
export function parseWeeks(text: string): { weeks: number[]; error: string | null } {
  const found = new Set<number>();
  const normalized = text.replace(/[，、;；]/g, ',').replace(/周/g, '');
  for (const part of normalized
    .split(',')
    .map((value) => value.trim())
    .filter(Boolean)) {
    const odd = part.includes('单');
    const even = part.includes('双');
    const clean = part.replace(/[()（）单双\s]/g, '');
    const bounds = clean.split('-');
    if (bounds.length > 2) return { weeks: [], error: '周次格式不受支持' };
    const start = Number(bounds[0]);
    const end = Number(bounds[bounds.length - 1]);
    if (!Number.isInteger(start) || !Number.isInteger(end)) {
      return { weeks: [], error: '周次格式不受支持' };
    }
    if (start === 0 || start > end || end > 60) {
      return { weeks: [], error: '周次必须在 1 到 60 之间' };
    }
    for (let week = start; week <= end; week++) {
      if ((odd && week % 2 === 0) || (even && week % 2 === 1)) continue;
      found.add(week);
    }
  }
  const weeks = [...found].sort((a, b) => a - b);
  return weeks.length ? { weeks, error: null } : { weeks: [], error: '请填写周次，例如 1-16' };
}

/** Turns weeks back into the compact text the editor shows. */
export function formatWeeks(weeks: number[]): string {
  const sorted = [...new Set(weeks)].sort((a, b) => a - b);
  const parts: string[] = [];
  let index = 0;
  while (index < sorted.length) {
    let end = index;
    while (end + 1 < sorted.length && sorted[end + 1] === sorted[end] + 1) end++;
    parts.push(end === index ? String(sorted[index]) : `${sorted[index]}-${sorted[end]}`);
    index = end + 1;
  }
  return parts.join(',');
}

export function coursesForDay(timetable: Timetable, date = new Date()): Course[] {
  const week = academicWeek(timetable.startDate, date);
  const day = date.getDay() || 7;
  return timetable.courses
    .filter((c) => c.day === day && c.weeks.includes(week))
    .sort((a, b) => a.start - b.start);
}

export function moveItem<T>(items: T[], from: number, delta: number): T[] {
  const result = [...items];
  const to = from + delta;
  if (from < 0 || from >= result.length || to < 0 || to >= result.length) return result;
  const [item] = result.splice(from, 1);
  result.splice(to, 0, item);
  return result;
}

export function visiblePlugins(plugins: Plugin[], config: AppConfig): Plugin[] {
  return plugins.filter((p) => !config.disabledPlugins.includes(p.id));
}
