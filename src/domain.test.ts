import { describe, expect, it } from 'vitest';
import {
  CAMPUS_OFFSET_MS,
  academicWeek,
  attendanceStats,
  campusDateKey,
  campusParts,
  cardSummary,
  courseHue,
  coursesForDay,
  currentAcademicStart,
  defaultConfig,
  displayCourseCode,
  dueInfo,
  examEntries,
  fileSize,
  formatWeeks,
  gradeStats,
  lmsTime,
  mergeSidebarOrder,
  money,
  moveItem,
  orderedPlugins,
  orderSidebarEntries,
  parseExamTime,
  parseWeeks,
  percent,
  sectionTime,
  sidebarPlugins,
  todoBuckets,
  updatedInfo,
  visiblePlugins,
} from './domain';
import type { CardTransaction, ExamRow, GradeRow, Plugin, Timetable } from './types';

describe('思源学堂 times', () => {
  it('shows UTC timestamps in campus time', () => {
    expect(lmsTime('2026-10-30T15:59:00Z')).toBe('2026-10-30 23:59');
    expect(lmsTime('2026-10-30T15:59:00.000Z')).toBe('2026-10-30 23:59');
  });

  it('moves late UTC times to the next campus day', () => {
    expect(lmsTime('2026-10-30T16:30:00Z')).toBe('2026-10-31 00:30');
    expect(lmsTime('2026-12-31T16:00:00Z')).toBe('2027-01-01 00:00');
  });

  it('respects explicit offsets and keeps zoneless values as they are', () => {
    expect(lmsTime('2026-10-30T23:59:00+08:00')).toBe('2026-10-30 23:59');
    expect(lmsTime('2026-10-30 23:59:00')).toBe('2026-10-30 23:59');
    expect(lmsTime('2026-10-30')).toBe('2026-10-30');
    expect(lmsTime('')).toBe('');
  });
});

describe('campus schedule dates', () => {
  it('handles week boundaries in local time', () => {
    expect(academicWeek('2026-09-07', new Date(2026, 8, 13, 23))).toBe(1);
    expect(academicWeek('2026-09-07', new Date(2026, 8, 14))).toBe(2);
    expect(academicWeek('2026-09-07', new Date(2026, 8, 6))).toBe(0);
  });

  it('filters alternating weeks and sorts by section', () => {
    const table: Timetable = {
      semester: '2026',
      startDate: '2026-09-07',
      endDate: '2027-01-15',
      courses: [
        { name: 'even', teacher: '', location: '', day: 1, start: 3, end: 4, weeks: [2, 4] },
        { name: 'odd', teacher: '', location: '', day: 1, start: 1, end: 2, weeks: [1, 3] },
        { name: 'every', teacher: '', location: '', day: 1, start: 7, end: 8, weeks: [1, 2, 3, 4] },
      ],
    };
    expect(coursesForDay(table, new Date(2026, 8, 14)).map((c) => c.name)).toEqual([
      'even',
      'every',
    ]);
    expect(coursesForDay(table, new Date(2026, 8, 15))).toEqual([]);
  });
});

describe('week text in the timetable editor', () => {
  it('reads ranges, lists and odd or even marks', () => {
    expect(parseWeeks('1-4').weeks).toEqual([1, 2, 3, 4]);
    expect(parseWeeks('1-8,10-12').weeks).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12]);
    expect(parseWeeks('1-8(单)').weeks).toEqual([1, 3, 5, 7]);
    expect(parseWeeks('2-8双').weeks).toEqual([2, 4, 6, 8]);
    expect(parseWeeks('1，3、5周').weeks).toEqual([1, 3, 5]);
    expect(parseWeeks(' 3 ').weeks).toEqual([3]);
  });

  it('explains what is wrong instead of guessing', () => {
    expect(parseWeeks('').error).toBe('请填写周次，例如 1-16');
    expect(parseWeeks('1-2-3').error).toBe('周次格式不受支持');
    expect(parseWeeks('三').error).toBe('周次格式不受支持');
    expect(parseWeeks('0-4').error).toBe('周次必须在 1 到 60 之间');
    expect(parseWeeks('8-4').error).toBe('周次必须在 1 到 60 之间');
  });

  it('formats runs back into text and round-trips', () => {
    expect(formatWeeks([1, 2, 3, 4, 5])).toBe('1-5');
    expect(formatWeeks([1, 2, 3, 5, 7, 8])).toBe('1-3,5,7-8');
    expect(formatWeeks([4, 2, 3, 3])).toBe('2-4');
    expect(formatWeeks([])).toBe('');
    expect(parseWeeks(formatWeeks([1, 3, 5, 9, 10])).weeks).toEqual([1, 3, 5, 9, 10]);
  });

  it('keeps a name on one hue', () => {
    expect(courseHue('高等数学')).toBe(courseHue('高等数学'));
    expect(courseHue('高等数学')).toBeLessThan(6);
  });
});

describe('workspace customization', () => {
  it('moves without mutating source and respects edges', () => {
    const source = ['a', 'b', 'c'];
    expect(moveItem(source, 1, -1)).toEqual(['b', 'a', 'c']);
    expect(source).toEqual(['a', 'b', 'c']);
    expect(moveItem(source, 0, -1)).toEqual(source);
  });
  it('keeps new plugins after the persisted order', () => {
    const plugins = ['new.plugin', 'old.b', 'old.a'].map((id) => ({ id }) as Plugin);
    expect(
      orderedPlugins(plugins, { ...defaultConfig(), pluginOrder: ['old.a', 'old.b'] }).map(
        (p) => p.id,
      ),
    ).toEqual(['old.a', 'old.b', 'new.plugin']);
  });
  it('orders sidebar entries and appends unknown ones', () => {
    const entries = ['home', 'agent', 'settings', 'new.plugin'].map((id) => ({ id }));
    expect(orderSidebarEntries(entries, ['settings', 'home']).map((e) => e.id)).toEqual([
      'settings',
      'home',
      'agent',
      'new.plugin',
    ]);
    expect(orderSidebarEntries(entries, [])).toEqual(entries);
  });
  it('keeps hidden sidebar entries in their slots when merging a drag', () => {
    const all = ['home', 'agent', 'campus', 'workbench', 'settings'];
    // Agent is hidden; dragging settings above workbench keeps agent's slot.
    expect(mergeSidebarOrder(all, ['home', 'campus', 'settings', 'workbench'])).toEqual([
      'home',
      'agent',
      'campus',
      'settings',
      'workbench',
    ]);
  });
});

/** 校园时间 → 时间戳；测试里固定用北京时间，避免依赖运行环境的时区。 */
const campus = (y: number, m: number, d: number, h = 0, min = 0) =>
  Date.UTC(y, m - 1, d, h, min) - CAMPUS_OFFSET_MS;

describe('campus clock', () => {
  it('reads wall-clock strings as campus time and zoned strings as instants', () => {
    expect(campusParts('2026-10-30 23:59')).toMatchObject({
      year: 2026,
      month: 10,
      day: 30,
      hour: 23,
      minute: 59,
      weekday: 5,
    });
    expect(campusParts('2026-10-30T15:59:00Z')).toMatchObject({ day: 30, hour: 23, minute: 59 });
    expect(campusParts(campus(2026, 10, 31))).toMatchObject({ day: 31, hour: 0 });
    expect(campusParts('不是时间')).toBeNull();
  });
});

describe('due labels', () => {
  const now = campus(2026, 10, 30, 12);

  it('names the day and counts down within a month', () => {
    expect(dueInfo(campus(2026, 10, 30, 23, 59), now)).toMatchObject({
      text: '今天 23:59',
      countdown: '还剩 11 小时',
      tone: 'warning',
    });
    expect(dueInfo(campus(2026, 10, 31, 8), now)).toMatchObject({
      text: '明天 08:00',
      countdown: '还剩 20 小时',
      tone: 'warning',
    });
    expect(dueInfo(campus(2026, 11, 1, 23, 59), now)).toMatchObject({
      text: '周日 23:59',
      countdown: '还剩 2 天',
    });
    expect(dueInfo(campus(2026, 11, 12, 23, 59), now)).toMatchObject({
      text: '11月12日 23:59',
      countdown: '还剩 13 天',
    });
  });

  it('marks urgent and overdue times', () => {
    expect(dueInfo(campus(2026, 10, 30, 14), now)).toMatchObject({
      countdown: '还剩 2 小时',
      tone: 'danger',
    });
    expect(dueInfo(campus(2026, 10, 30, 12, 45), now).countdown).toBe('还剩 45 分钟');
    expect(dueInfo(campus(2026, 10, 28, 23, 59), now)).toMatchObject({
      text: '10月28日 23:59',
      countdown: '已逾期 1 天',
      tone: 'danger',
      overdue: true,
    });
  });

  it('drops the countdown beyond 30 days and handles missing times', () => {
    expect(dueInfo(campus(2026, 12, 20), now).countdown).toBe('');
    expect(dueInfo(null, now)).toMatchObject({
      text: '无截止时间',
      countdown: '',
      tone: 'neutral',
    });
    expect(dueInfo(campus(2027, 1, 3, 9), now).text).toBe('2027年1月3日 09:00');
  });
});

describe('updated labels', () => {
  const now = campus(2026, 10, 30, 17, 30);
  it('uses relative text for the last hour and dates after today', () => {
    expect(updatedInfo(now - 20_000, now)).toBe('刚刚');
    expect(updatedInfo(now - 5 * 60_000, now)).toBe('5 分钟前');
    expect(updatedInfo(campus(2026, 10, 30, 16, 20), now)).toBe('今天 16:20');
    expect(updatedInfo(campus(2026, 10, 29, 9), now)).toBe('10月29日');
    expect(updatedInfo(null, now)).toBe('');
  });
});

describe('money, sizes and percentages', () => {
  it('formats cents without a minus sign for spending', () => {
    expect(money(1250)).toBe('¥12.50');
    expect(money(-1250)).toBe('¥12.50');
    expect(money(50000, true)).toBe('+¥500.00');
    expect(money(0, true)).toBe('¥0.00');
  });
  it('formats file sizes', () => {
    expect(fileSize(8)).toBe('8 B');
    expect(fileSize(8 * 1024)).toBe('8 KB');
    expect(fileSize(1.2 * 1024 * 1024)).toBe('1.2 MB');
    expect(fileSize(null)).toBe('');
  });
  it('formats ratios', () => {
    expect(percent(0.864)).toBe('86.4%');
    expect(percent(null)).toBe('—');
  });
});

describe('course code labels', () => {
  it('drops the semester prefix that the term heading already shows', () => {
    expect(displayCourseCode('202620271COMP45020502')).toBe('COMP45020502');
    expect(displayCourseCode('202620271CORE10030001')).toBe('CORE10030001');
    expect(displayCourseCode('202520262MATH1001')).toBe('MATH1001');
  });
  it('keeps codes without a semester prefix', () => {
    expect(displayCourseCode('COMP45020502')).toBe('COMP45020502');
    expect(displayCourseCode('20262027')).toBe('20262027');
    expect(displayCourseCode('')).toBe('');
    expect(displayCourseCode(null)).toBe('');
  });
});

describe('grade stats', () => {
  const row = (over: Partial<GradeRow>): GradeRow => ({
    id: Math.random().toString(36),
    name: '课',
    code: '',
    term: '2026 秋',
    credits: 2,
    score: '80',
    gpa: 3,
    category: '必修',
    passed: true,
    ...over,
  });

  it('weights by credits and keeps failed courses out of earned credits', () => {
    const stats = gradeStats([
      row({ credits: 4, score: '92', gpa: 4 }),
      row({ credits: 3, score: '85', gpa: 3.7 }),
      row({ credits: 2, score: '55', gpa: 1, passed: false }),
    ]);
    expect(stats.courses).toBe(3);
    expect(stats.credits).toBe(7);
    expect(stats.failed).toBe(1);
    expect(stats.failedCredits).toBe(2);
    expect(stats.gpa).toBeCloseTo((4 * 4 + 3.7 * 3 + 1 * 2) / 9, 2);
    expect(stats.average).toBeCloseTo((92 * 4 + 85 * 3 + 55 * 2) / 9, 1);
  });

  it('leaves letter grades out of the average', () => {
    const stats = gradeStats([row({ score: 'A＋', gpa: 4.3 }), row({ score: '90', gpa: 4 })]);
    expect(stats.average).toBe(90);
    expect(stats.unscored).toBe(1);
    expect(stats.credits).toBe(4);
  });

  it('returns null when nothing can be averaged', () => {
    expect(gradeStats([])).toMatchObject({ courses: 0, credits: 0, gpa: null, average: null });
  });
});

describe('todo buckets', () => {
  it('groups by campus day and sorts by deadline', () => {
    const now = campus(2026, 10, 30, 12);
    const groups = todoBuckets(
      [
        { name: 'later', dueAt: campus(2026, 11, 3) },
        { name: 'week', dueAt: campus(2026, 11, 1) },
        { name: 'none', dueAt: null },
        { name: 'overdue', dueAt: campus(2026, 10, 29) },
        { name: 'today', dueAt: campus(2026, 10, 30, 23) },
        { name: 'tomorrow', dueAt: campus(2026, 10, 31, 8) },
      ],
      now,
    );
    expect(groups.map((group) => group.id)).toEqual([
      'overdue',
      'today',
      'tomorrow',
      'week',
      'later',
      'none',
    ]);
    expect(groups[0].items.map((item) => item.name)).toEqual(['overdue']);
  });

  it('omits empty groups and orders several items in a group', () => {
    const now = campus(2026, 10, 30, 12);
    const groups = todoBuckets(
      [
        { name: 'late', dueAt: campus(2026, 10, 30, 23) },
        { name: 'early', dueAt: campus(2026, 10, 30, 18) },
      ],
      now,
    );
    expect(groups).toHaveLength(1);
    expect(groups[0].items.map((item) => item.name)).toEqual(['early', 'late']);
  });
});

describe('exam and section times', () => {
  it('parses the school exam string in campus time', () => {
    expect(parseExamTime('2026-11-06 08:30-10:30(星期五)')).toEqual({
      start: campus(2026, 11, 6, 8, 30),
      end: campus(2026, 11, 6, 10, 30),
    });
    expect(parseExamTime('2026-11-06 23:00-01:00')).toEqual({
      start: campus(2026, 11, 6, 23),
      end: campus(2026, 11, 7, 1),
    });
    expect(parseExamTime('待定')).toBeNull();
    expect(parseExamTime('2026-13-06 08:30-10:30')).toBeNull();
  });

  it('maps sections to clock times from the school periods', () => {
    const periods = [
      { periodNo: 1, start: '08:00', end: '08:45' },
      { periodNo: 2, start: '08:55', end: '09:40' },
      { periodNo: 3, start: '10:10', end: '10:55' },
      { periodNo: 4, start: '11:05', end: '11:50' },
    ];
    expect(sectionTime(periods, { start: 3, end: 4 })).toEqual({
      start: '10:10',
      end: '11:50',
    });
    expect(sectionTime(periods, { start: 3, end: 9 })).toBeNull();
    expect(sectionTime(null, { start: 3, end: 4 })).toBeNull();
  });
});

describe('campus date keys', () => {
  it('reads wall-clock, slash dates and instants in campus time', () => {
    expect(campusDateKey('2026-10-30 23:59')).toBe('2026-10-30');
    expect(campusDateKey('2026/10/30 12:05')).toBe('2026-10-30');
    expect(campusDateKey('2026-10-30T15:59:00Z')).toBe('2026-10-30');
    expect(campusDateKey(campus(2026, 10, 31))).toBe('2026-10-31');
    expect(campusDateKey('不是时间')).toBe('');
  });
});

describe('current academic year', () => {
  it('starts a new academic year in August and keeps January in the old one', () => {
    expect(currentAcademicStart(campus(2026, 10, 31))).toBe(2026);
    expect(currentAcademicStart(campus(2027, 1, 5))).toBe(2026);
    expect(currentAcademicStart(campus(2027, 7, 1))).toBe(2026);
    expect(currentAcademicStart(campus(2027, 8, 1))).toBe(2027);
  });
});

describe('card summary', () => {
  const row = (time: string, amount: number, merchant = '食堂'): CardTransaction => ({
    time,
    amount,
    balance: 10000,
    merchant,
    type: '消费',
    description: '',
  });

  it('adds spending and income and keeps one bucket per campus day', () => {
    const now = campus(2026, 10, 30, 18);
    const summary = cardSummary(
      [
        row('2026-10-30 12:05', -1250),
        row('2026-10-30 08:00', -800),
        row('2026-10-29 19:00', 50000),
        row('2026-10-23 09:00', -300),
        row('2026-10-22 09:00', -999),
      ],
      7,
      now,
    );
    expect(summary.spend).toBe(2050);
    expect(summary.income).toBe(50000);
    expect(summary.count).toBe(3);
    expect(summary.largest?.amount).toBe(-1250);
    expect(summary.days).toHaveLength(7);
    expect(summary.days[0].date).toBe('2026-10-24');
    expect(summary.days.at(-1)).toEqual({ date: '2026-10-30', spend: 2050 });
  });

  it('leaves rows outside the window out of every total', () => {
    const now = campus(2026, 10, 30, 18);
    const summary = cardSummary([row('2026-10-23 09:00', -300)], 7, now);
    expect(summary).toMatchObject({ spend: 0, income: 0, count: 0, largest: null });
    expect(summary.days.every((day) => day.spend === 0)).toBe(true);
  });

  it('ignores zero amounts and an unusable clock', () => {
    expect(cardSummary([row('2026-10-30 12:05', 0)], 7, campus(2026, 10, 30))).toMatchObject({
      spend: 0,
      income: 0,
      count: 0,
      largest: null,
    });
  });
});

describe('exam entries', () => {
  const base: Omit<ExamRow, 'course' | 'time'> = {
    date: '2026-11-06 08:30:00',
    location: '主楼 A-101',
    seat: '23',
    campus: '兴庆校区',
  };

  it('parses school times into readable dates and ranges', () => {
    const entries = examEntries(
      [
        {
          ...base,
          course: '大学英语',
          time: '2026-11-10 14:00-16:00(星期二)',
        },
        { ...base, course: '高等数学', time: '2026-11-06 08:30-10:30(星期五)' },
      ],
      campus(2026, 10, 30, 12),
    );
    expect(entries.map((entry) => entry.course)).toEqual(['高等数学', '大学英语']);
    expect(entries[0]).toMatchObject({
      dateLabel: '11月6日 周五',
      range: '08:30–10:30',
      ended: false,
    });
  });

  it('marks past exams and keeps unparsable times last', () => {
    const entries = examEntries(
      [
        { ...base, course: '待定课', time: '待定' },
        { ...base, course: '已结束', time: '2026-10-20 08:30-10:30' },
      ],
      campus(2026, 10, 30, 12),
    );
    expect(entries[0]).toMatchObject({ course: '已结束', ended: true });
    expect(entries[1]).toMatchObject({ course: '待定课', dateLabel: base.date, range: '待定' });
  });
});

describe('attendance stats', () => {
  it('leaves leave and unknown states out of the denominator', () => {
    const stats = attendanceStats([
      { status: 'NORMAL' },
      { status: 'NORMAL' },
      { status: 'LATE' },
      { status: 'ABSENT' },
      { status: 'LEAVE' },
      { status: 'PENDING' },
    ]);
    expect(stats).toMatchObject({
      normal: 2,
      late: 1,
      absent: 1,
      leave: 1,
      ignored: 1,
      rate: 0.75,
      onTimeRate: 0.5,
    });
    expect(attendanceStats([]).rate).toBeNull();
  });
});

it('shows enabled plugins and ignores the legacy hidden list', () => {
  const plugins = [{ id: 'local.one' }, { id: 'local.two' }] as Plugin[];
  const config = { ...defaultConfig(), hiddenPlugins: ['local.one'] };
  expect(visiblePlugins(plugins, config).map((p) => p.id)).toEqual(['local.one', 'local.two']);
  config.disabledPlugins = ['local.one'];
  expect(visiblePlugins(plugins, config).map((p) => p.id)).toEqual(['local.two']);
});

it('keeps workflow pages out of the sidebar without disabling them', () => {
  const plugins = [{ id: 'workflow.page', sidebar: false }, { id: 'normal.page' }] as Plugin[];
  const config = defaultConfig();
  expect(sidebarPlugins(plugins, config).map((p) => p.id)).toEqual(['normal.page']);
  expect(visiblePlugins(plugins, config).map((p) => p.id)).toEqual([
    'workflow.page',
    'normal.page',
  ]);
});
