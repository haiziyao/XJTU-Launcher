import { describe, expect, it } from 'vitest';
import { academicWeek, coursesForDay, defaultConfig, moveItem, orderedPlugins } from './domain';
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
