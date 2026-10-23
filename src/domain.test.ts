import { describe, expect, it } from 'vitest';
import {
  academicWeek,
  courseHue,
  coursesForDay,
  defaultConfig,
  formatWeeks,
  moveItem,
  orderedPlugins,
  parseWeeks,
  visiblePlugins,
} from './domain';
import type { Plugin, Timetable } from './types';

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
});

it('shows enabled plugins and ignores the legacy hidden list', () => {
  const plugins = [{ id: 'local.one' }, { id: 'local.two' }] as Plugin[];
  const config = { ...defaultConfig(), hiddenPlugins: ['local.one'] };
  expect(visiblePlugins(plugins, config).map((p) => p.id)).toEqual(['local.one', 'local.two']);
  config.disabledPlugins = ['local.one'];
  expect(visiblePlugins(plugins, config).map((p) => p.id)).toEqual(['local.two']);
});
