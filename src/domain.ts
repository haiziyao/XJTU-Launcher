import type { AppConfig, Course, Plugin, Timetable } from './types';

export const defaultConfig = (): AppConfig => ({
  version: 1,
  sidebarCollapsed: false,
  sidebarWidth: 196,
  nickname: '',
  avatar: null,
  wallpaper: null,
  reduceMotion: false,
  showSeconds: true,
  accent: 'blue',
  greeting: '把校园，放在手边。',
  widgets: ['timetable', 'links', 'plugins'],
  pluginOrder: [],
  disabledPlugins: [],
  hiddenPlugins: [],
  pluginServer: null,
  checkPluginUpdates: true,
  checkAppUpdates: true,
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
  return plugins.filter(
    (p) => !config.disabledPlugins.includes(p.id) && !config.hiddenPlugins.includes(p.id),
  );
}
