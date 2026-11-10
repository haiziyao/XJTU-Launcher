import type {
  AppConfig,
  CardTransaction,
  Course,
  ExamRow,
  GradeRow,
  Plugin,
  Timetable,
} from '../types';
import { DEFAULT_ACCENT } from '../features/appearance/theme';
import { DEFAULT_PET } from './pets';

export const DEFAULT_SIDEBAR_WIDTH = 140;

export const defaultConfig = (): AppConfig => ({
  version: 1,
  sidebarCollapsed: false,
  sidebarWidth: DEFAULT_SIDEBAR_WIDTH,
  sidebarOrder: [],
  agentSidebarVisible: true,
  agentSearchEnabled: false,
  nickname: '',
  avatar: null,
  wallpaper: null,
  reduceMotion: false,
  showSeconds: true,
  accent: DEFAULT_ACCENT,
  pet: DEFAULT_PET,
  greeting: '把校园，放在手边。',
  widgets: ['timetable', 'links', 'notices', 'plugins'],
  pluginOrder: [],
  disabledPlugins: ['cn.xjtu.desktop-pet', 'cn.xjtu.font-beautify'],
  hiddenPlugins: [],
  pluginLayout: 1,
  checkAppUpdates: true,
  campusPinned: [],
  campusRailCollapsed: false,
  homeClock: 'plain',
  homeSections: ['greeting', 'clock', 'timetable', 'links', 'notices'],
  mcpServerEnabled: false,
  mcpServerPort: 39217,
  mcpServerDisabledTools: [],
  workbenchAutoArchive: true,
  pageTabSurfaces: false,
  wallpaperDisplay: 'balanced',
  wallpaperPanel: { clear: 92, balanced: 72, vivid: 52 },
  updateEndpoint: '',
});

export const campusLinks = [
  { title: '一网通办', description: '校园办事与服务', url: 'https://ywtb.xjtu.edu.cn' },
  { title: '教务系统', description: '教学与学业信息', url: 'https://jwxt.xjtu.edu.cn' },
  { title: '思源学堂', description: '课程与学习资源', url: 'https://lms.xjtu.edu.cn' },
  { title: '图书馆', description: '文献检索与借阅', url: 'https://www.lib.xjtu.edu.cn' },
];

export function orderedPlugins(plugins: Plugin[], config: AppConfig): Plugin[] {
  const positions = new Map(config.pluginOrder.map((id, i) => [id, i]));
  return [...plugins].sort(
    (a, b) =>
      (positions.get(a.id) ?? Number.MAX_SAFE_INTEGER) -
        (positions.get(b.id) ?? Number.MAX_SAFE_INTEGER) || a.id.localeCompare(b.id),
  );
}

/**
 * Sort sidebar entries by the stored order. Unknown ids (new plugins, new
 * fixed entries) stay after the listed ones, in their built-in order.
 */
export function orderSidebarEntries<T extends { id: string }>(entries: T[], order: string[]): T[] {
  if (!order.length) return entries;
  const rank = new Map(order.map((id, index) => [id, index]));
  return entries
    .map((entry, index) => ({ entry, index }))
    .sort((a, b) => {
      const left = rank.get(a.entry.id);
      const right = rank.get(b.entry.id);
      if (left === undefined && right === undefined) return a.index - b.index;
      if (left === undefined) return 1;
      if (right === undefined) return -1;
      return left - right;
    })
    .map(({ entry }) => entry);
}

/**
 * Put a reordered visible list back into the full entry list. Hidden entries
 * keep their slots, so re-enabling a plugin restores where it used to sit.
 */
export function mergeSidebarOrder(allIds: string[], visible: string[]): string[] {
  const visibleSet = new Set(visible);
  let cursor = 0;
  return allIds.map((id) => (visibleSet.has(id) ? visible[cursor++] : id));
}

// Use local dates, so a UTC offset does not move a course to the previous day.
export function academicWeek(start: string, date = new Date()): number {
  const parts = start.slice(0, 10).split('-').map(Number);
  if (parts.length !== 3 || parts.some((n) => !Number.isFinite(n))) return 0;
  const beginning = Date.UTC(parts[0], parts[1] - 1, parts[2]);
  const today = Date.UTC(date.getFullYear(), date.getMonth(), date.getDate());
  return Math.floor((today - beginning) / 604800000) + 1;
}

const ZONED = /(Z|[+-]\d{2}:?\d{2})$/i;

/**
 * 思源学堂 returns UTC ISO times ("…Z"). Show them in campus time (UTC+8, no
 * daylight saving) to the minute; values without a zone are already local.
 */
export function lmsTime(value: string): string {
  const text = value.trim();
  if (!text) return '';
  const date = ZONED.test(text) ? new Date(text) : null;
  if (!date || Number.isNaN(date.getTime())) return text.replace('T', ' ').slice(0, 16);
  const local = new Date(date.getTime() + 8 * 3600000);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${local.getUTCFullYear()}-${pad(local.getUTCMonth() + 1)}-${pad(local.getUTCDate())} ${pad(
    local.getUTCHours(),
  )}:${pad(local.getUTCMinutes())}`;
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

/** Plugins that get a sidebar entry; workflow pages opt out in their manifest. */
export function sidebarPlugins(plugins: Plugin[], config: AppConfig): Plugin[] {
  return visiblePlugins(plugins, config).filter((p) => p.sidebar !== false);
}

/* ---------------------------------------------------------------------------
 * 校园原生化（v0.6）用的纯函数：时间文案、金额、文件大小和派生指标。
 * 所有时间都按 Asia/Shanghai（UTC+8，无夏令时）计算，不依赖运行环境时区。
 * ------------------------------------------------------------------------- */

/** 校园时区相对 UTC 的偏移。 */
export const CAMPUS_OFFSET_MS = 8 * 3600000;

const pad2 = (value: number) => String(value).padStart(2, '0');
const WEEKDAYS = ['日', '一', '二', '三', '四', '五', '六'];

/**
 * 把时间值读成时间戳。数字和带时区的字符串按真实时刻；不带时区的
 * `YYYY-MM-DD HH:mm` 按校园墙上时间理解（后端已统一转成北京时间）。
 */
export function campusTimestamp(value: number | string | Date): number {
  if (typeof value === 'number') return value;
  if (value instanceof Date) return value.getTime();
  const text = value.trim();
  if (!text) return NaN;
  if (ZONED.test(text)) return Date.parse(text);
  const match = /^(\d{4})-(\d{2})-(\d{2})(?:[ T](\d{1,2}):(\d{2}))?/.exec(text);
  if (!match) return Date.parse(text);
  return (
    Date.UTC(+match[1], +match[2] - 1, +match[3], +(match[4] ?? 0), +(match[5] ?? 0)) -
    CAMPUS_OFFSET_MS
  );
}

export interface CampusParts {
  year: number;
  month: number;
  day: number;
  hour: number;
  minute: number;
  /** 0 = 周日。 */
  weekday: number;
}

/** 拆成校园时间的年月日时分；无法解析时返回 null。 */
export function campusParts(value: number | string | Date): CampusParts | null {
  const stamp = campusTimestamp(value);
  if (!Number.isFinite(stamp)) return null;
  const local = new Date(stamp + CAMPUS_OFFSET_MS);
  return {
    year: local.getUTCFullYear(),
    month: local.getUTCMonth() + 1,
    day: local.getUTCDate(),
    hour: local.getUTCHours(),
    minute: local.getUTCMinutes(),
    weekday: local.getUTCDay(),
  };
}

/**
 * 当前校园学年开始年份（8 月进入新学年，1–7 月仍属上一学年）。
 * 用来隐藏尚未开课 / 尚未体测的学年，避免下拉框里全是空数据。
 */
export function currentAcademicStart(now: number | Date = Date.now()): number {
  const parts = campusParts(now);
  if (!parts) return new Date(now).getFullYear();
  return parts.month >= 8 ? parts.year : parts.year - 1;
}

/** 校园日历日的序号，用来算“今天 / 明天 / 差几天”。 */
function campusDay(parts: CampusParts): number {
  return Date.UTC(parts.year, parts.month - 1, parts.day) / 86400000;
}

export type DueTone = 'danger' | 'warning' | 'neutral';

export interface DueInfo {
  /** 截止文案：今天 23:59 / 明天 08:00 / 周五 23:59 / 11月12日 23:59 / 无截止时间。 */
  text: string;
  /** 倒计时：还剩 45 分钟 / 还剩 3 小时 / 还剩 2 天 / 已逾期 1 天；超过 30 天为空。 */
  countdown: string;
  tone: DueTone;
  overdue: boolean;
}

function countdownText(diff: number): string {
  // 按已过去的整段时间取整，和“还剩 13 小时”这类示例一致。
  const minutes = Math.floor(Math.abs(diff) / 60000);
  const prefix = diff < 0 ? '已逾期' : '还剩';
  if (minutes < 60) return `${prefix} ${Math.max(1, minutes)} 分钟`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${prefix} ${hours} 小时`;
  return `${prefix} ${Math.floor(hours / 24)} 天`;
}

/** 截止时间按 LAYOUT §5.3 出文案和状态色，`at` 为空时表示没有截止时间。 */
export function dueInfo(at: number | null, now: number | Date = Date.now()): DueInfo {
  const nowStamp = typeof now === 'number' ? now : now.getTime();
  if (at == null) {
    return { text: '无截止时间', countdown: '', tone: 'neutral', overdue: false };
  }
  const due = campusParts(at);
  const current = campusParts(nowStamp);
  if (!due || !current) {
    return { text: '无截止时间', countdown: '', tone: 'neutral', overdue: false };
  }
  const clock = `${pad2(due.hour)}:${pad2(due.minute)}`;
  const days = campusDay(due) - campusDay(current);
  let text: string;
  if (days === 0) text = `今天 ${clock}`;
  else if (days === 1) text = `明天 ${clock}`;
  else if (days >= 2 && days <= 6) text = `周${WEEKDAYS[due.weekday]} ${clock}`;
  else if (due.year === current.year) text = `${due.month}月${due.day}日 ${clock}`;
  else text = `${due.year}年${due.month}月${due.day}日 ${clock}`;

  const diff = at - nowStamp;
  const overdue = diff < 0;
  // 超过 30 天没有可操作意义，不显示倒计时。
  const countdown = Math.abs(diff) > 30 * 86400000 ? '' : countdownText(diff);
  const tone: DueTone =
    diff < 0 || diff <= 3 * 3600000 ? 'danger' : diff <= 24 * 3600000 ? 'warning' : 'neutral';
  return { text, countdown, tone, overdue };
}

/** 更新时间：刚刚 / 5 分钟前 / 今天 16:20 / 10月29日 / 2025年10月29日。 */
export function updatedInfo(value: number | string | Date | null, now = Date.now()): string {
  if (value == null) return '';
  const stamp = campusTimestamp(value);
  if (!Number.isFinite(stamp)) return '';
  const parts = campusParts(stamp);
  const current = campusParts(now);
  if (!parts || !current) return '';
  const diff = now - stamp;
  if (diff < 60000) return '刚刚';
  if (diff < 3600000) return `${Math.floor(diff / 60000)} 分钟前`;
  const clock = `${pad2(parts.hour)}:${pad2(parts.minute)}`;
  if (campusDay(parts) === campusDay(current)) return `今天 ${clock}`;
  if (parts.year === current.year) return `${parts.month}月${parts.day}日`;
  return `${parts.year}年${parts.month}月${parts.day}日`;
}

/** 金额以分为单位；支出不加负号，用“支出”二字区分，收入传 `signed` 得到 “+¥500.00”。 */
export function money(cents: number, signed = false): string {
  const value = (Math.abs(cents) / 100).toFixed(2);
  const plus = signed && cents > 0 ? '+' : '';
  return `${plus}¥${value}`;
}

/** 文件大小：8 B / 320 KB / 1.2 MB / 2.0 GB。 */
export function fileSize(bytes: number | null | undefined): string {
  if (bytes == null || !Number.isFinite(bytes)) return '';
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  if (bytes < 1024 * 1024) return `${Math.round(bytes / 1024)} KB`;
  if (bytes < 1024 * 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
  return `${(bytes / 1024 / 1024 / 1024).toFixed(1)} GB`;
}

/**
 * 思源学堂的课程编号前 9 位是学期码（`202620271` = 2026-2027 学年第一学期）。
 * 工作台已经按学期分组，界面上只显示后面的课程号；没有学期前缀就原样返回。
 */
export function displayCourseCode(code: string | null | undefined): string {
  if (!code) return '';
  return code.replace(/^\d{8}\d?(?=[A-Za-z])/, '');
}

/** 比例出百分比文案：0.864 → “86.4%”；没有值显示 “—”。 */
export function percent(ratio: number | null, digits = 1): string {
  if (ratio == null || !Number.isFinite(ratio)) return '—';
  return `${(ratio * 100).toFixed(digits)}%`;
}

export interface GradeStats {
  courses: number;
  /** 已修学分：成绩未标记不及格且学分大于 0。 */
  credits: number;
  /** 加权绩点，保留两位小数。 */
  gpa: number | null;
  /** 加权均分，保留一位小数；等级制成绩不计入。 */
  average: number | null;
  failed: number;
  failedCredits: number;
  /** 成绩不是 0–100 数字、没计入均分的门数。 */
  unscored: number;
}

function numericScore(score: string): number | null {
  const text = score.trim();
  if (!/^\d+(\.\d+)?$/.test(text)) return null;
  const value = Number(text);
  return value >= 0 && value <= 100 ? value : null;
}

/** 按本页成绩行计算指标；学校官方口径未知，界面须注明“按本页数据计算”。 */
export function gradeStats(rows: GradeRow[]): GradeStats {
  let credits = 0;
  let gpaCredits = 0;
  let gpaSum = 0;
  let averageCredits = 0;
  let averageSum = 0;
  let failed = 0;
  let failedCredits = 0;
  let unscored = 0;
  for (const row of rows) {
    const credit = row.credits ?? 0;
    if (credit > 0 && row.passed !== false) credits += credit;
    if (row.passed === false) {
      failed++;
      if (credit > 0) failedCredits += credit;
    }
    const score = numericScore(row.score);
    if (credit > 0 && score == null) unscored++;
    if (credit > 0 && row.gpa != null) {
      gpaSum += row.gpa * credit;
      gpaCredits += credit;
    }
    if (credit > 0 && score != null) {
      averageSum += score * credit;
      averageCredits += credit;
    }
  }
  return {
    courses: rows.length,
    credits,
    gpa: gpaCredits ? Math.round((gpaSum / gpaCredits) * 100) / 100 : null,
    average: averageCredits ? Math.round((averageSum / averageCredits) * 10) / 10 : null,
    failed,
    failedCredits,
    unscored,
  };
}

export type TodoBucket = 'overdue' | 'today' | 'tomorrow' | 'week' | 'later' | 'none';

export interface TodoGroup<T> {
  id: TodoBucket;
  title: string;
  items: T[];
}

const TODO_ORDER: { id: TodoBucket; title: string }[] = [
  { id: 'overdue', title: '已逾期' },
  { id: 'today', title: '今天' },
  { id: 'tomorrow', title: '明天' },
  { id: 'week', title: '本周' },
  { id: 'later', title: '以后' },
  { id: 'none', title: '无截止' },
];

/** 待办按校园日期分组；空组不返回，组内按截止时间升序，无截止排最后。 */
export function todoBuckets<T extends { dueAt: number | null }>(
  items: T[],
  now = Date.now(),
): TodoGroup<T>[] {
  const current = campusParts(now);
  if (!current) return items.length ? [{ id: 'none', title: '无截止', items }] : [];
  const today = campusDay(current);
  // 周一为一周的开始，和学校日历一致。
  const monday = today - ((current.weekday + 6) % 7);
  const groups = new Map<TodoBucket, T[]>();
  for (const item of items) {
    const parts = item.dueAt == null ? null : campusParts(item.dueAt);
    let bucket: TodoBucket;
    if (!parts || item.dueAt == null) bucket = 'none';
    else {
      const day = campusDay(parts);
      if (item.dueAt < now) bucket = 'overdue';
      else if (day === today) bucket = 'today';
      else if (day === today + 1) bucket = 'tomorrow';
      else if (day < monday + 7) bucket = 'week';
      else bucket = 'later';
    }
    const list = groups.get(bucket);
    if (list) list.push(item);
    else groups.set(bucket, [item]);
  }
  return TODO_ORDER.filter((entry) => groups.has(entry.id)).map((entry) => ({
    ...entry,
    items: groups
      .get(entry.id)!
      .slice()
      .sort((a, b) => (a.dueAt ?? Infinity) - (b.dueAt ?? Infinity)),
  }));
}

/** 解析教务的考试时间 `YYYY-MM-DD HH:MM-HH:MM(星期X)`；解析失败返回 null，不显示倒计时。 */
export function parseExamTime(text: string): { start: number; end: number } | null {
  const match = /^(\d{4})-(\d{2})-(\d{2})\s+(\d{1,2}):(\d{2})\s*[-–—~至]\s*(\d{1,2}):(\d{2})/.exec(
    text.trim(),
  );
  if (!match) return null;
  const [year, month, day, startHour, startMinute, endHour, endMinute] = match
    .slice(1, 8)
    .map(Number);
  if (
    month < 1 ||
    month > 12 ||
    day < 1 ||
    day > 31 ||
    startHour > 23 ||
    endHour > 23 ||
    startMinute > 59 ||
    endMinute > 59
  ) {
    return null;
  }
  const start = Date.UTC(year, month - 1, day, startHour, startMinute) - CAMPUS_OFFSET_MS;
  let end = Date.UTC(year, month - 1, day, endHour, endMinute) - CAMPUS_OFFSET_MS;
  if (end <= start) end += 86400000;
  return { start, end };
}

export interface SectionPeriod {
  periodNo: number;
  start: string;
  end: string;
}

/** 用课表接口返回的作息把节次换成钟点；作息缺失时返回 null，界面只显示节次。 */
export function sectionTime(
  periods: SectionPeriod[] | null | undefined,
  section: { start: number; end: number },
): { start: string; end: string } | null {
  if (!periods?.length) return null;
  const first = periods.find((period) => period.periodNo === section.start);
  const last = periods.find((period) => period.periodNo === section.end);
  if (!first || !last) return null;
  return { start: first.start, end: last.end };
}

export interface AttendanceStats {
  normal: number;
  late: number;
  absent: number;
  leave: number;
  /** 请假、无需考勤、待定、未知的次数，不计入分母。 */
  ignored: number;
  /** 到课率 = (正常 + 迟到) / (正常 + 迟到 + 缺勤)；分母为 0 时为 null。 */
  rate: number | null;
  /** 准时率 = 正常 / (正常 + 迟到 + 缺勤)。 */
  onTimeRate: number | null;
}

/** 汇总考勤记录；状态来自学校接口（NORMAL / LATE / ABSENT / LEAVE）。 */
export function attendanceStats(rows: { status: string }[]): AttendanceStats {
  let normal = 0;
  let late = 0;
  let absent = 0;
  let leave = 0;
  let ignored = 0;
  for (const row of rows) {
    switch (row.status.trim().toUpperCase()) {
      case 'NORMAL':
        normal++;
        break;
      case 'LATE':
        late++;
        break;
      case 'ABSENT':
        absent++;
        break;
      case 'LEAVE':
        leave++;
        break;
      default:
        ignored++;
    }
  }
  const total = normal + late + absent;
  return {
    normal,
    late,
    absent,
    leave,
    ignored,
    rate: total ? (normal + late) / total : null,
    onTimeRate: total ? normal / total : null,
  };
}

/** 校区日期（YYYY-MM-DD），兼容 `2026-10-30 12:05` 与 `2026/10/30 12:05`。 */
export function campusDateKey(value: number | string | Date): string {
  const parts = campusParts(value);
  if (parts) return `${parts.year}-${pad2(parts.month)}-${pad2(parts.day)}`;
  if (typeof value === 'string') {
    const match = /^(\d{4})[-/](\d{1,2})[-/](\d{1,2})/.exec(value.trim());
    if (match) return `${match[1]}-${pad2(+match[2])}-${pad2(+match[3])}`;
  }
  return '';
}

export interface CardTotals {
  spend: number;
  income: number;
  count: number;
  largest: CardTransaction | null;
  /** One entry per day in the window, oldest first. */
  days: { date: string; spend: number }[];
}

/**
 * Totals for the card page, calculated from the transactions already on screen.
 * Amounts stay in cents; spend is positive. Days are campus dates, not the
 * runtime timezone.
 */
export function cardSummary(
  rows: CardTransaction[],
  windowDays = 7,
  now: number | Date = Date.now(),
): CardTotals {
  const today = campusDateKey(now);
  const days: { date: string; spend: number }[] = [];
  if (today) {
    const stamp = campusTimestamp(`${today} 00:00`);
    for (let offset = windowDays - 1; offset >= 0; offset--) {
      days.push({ date: campusDateKey(stamp - offset * 86400000), spend: 0 });
    }
  }
  const byDate = new Map(days.map((day) => [day.date, day]));
  let spend = 0;
  let income = 0;
  let count = 0;
  let largest: CardTransaction | null = null;
  for (const row of rows) {
    if (!row.amount) continue;
    const day = byDate.get(campusDateKey(row.time));
    if (!day) continue;
    count++;
    if (row.amount < 0) {
      const value = -row.amount;
      spend += value;
      if (!largest || value > -largest.amount) largest = row;
      day.spend += value;
    } else {
      income += row.amount;
    }
  }
  return { spend, income, count, largest, days };
}

export interface ExamEntry {
  course: string;
  campus: string;
  location: string;
  seat: string;
  /** 11月6日 周四 */
  dateLabel: string;
  /** 08:30–10:30 */
  range: string;
  start: number | null;
  end: number | null;
  ended: boolean;
}

/** Parse exam rows into a sorted schedule; unparsable times keep their text. */
export function examEntries(rows: ExamRow[], now: number | Date = Date.now()): ExamEntry[] {
  const stamp = typeof now === 'number' ? now : now.getTime();
  const entries = rows.map((row): ExamEntry => {
    const parsed = parseExamTime(row.time) || parseExamTime(`${row.date} ${row.time}`);
    const startParts = parsed ? campusParts(parsed.start) : campusParts(row.time);
    const endParts = parsed ? campusParts(parsed.end) : null;
    return {
      course: row.course,
      campus: row.campus,
      location: row.location,
      seat: row.seat,
      dateLabel: startParts
        ? `${startParts.month}月${startParts.day}日 周${WEEKDAYS[startParts.weekday]}`
        : row.date,
      range:
        startParts && endParts
          ? `${pad2(startParts.hour)}:${pad2(startParts.minute)}–${pad2(endParts.hour)}:${pad2(
              endParts.minute,
            )}`
          : row.time,
      start: parsed?.start ?? null,
      end: parsed?.end ?? null,
      ended: parsed ? parsed.end < stamp : false,
    };
  });
  return entries.sort((a, b) => {
    if (a.start == null && b.start == null) return 0;
    if (a.start == null) return 1;
    if (b.start == null) return -1;
    return a.start - b.start;
  });
}
