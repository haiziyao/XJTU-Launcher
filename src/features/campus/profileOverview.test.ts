import { describe, expect, it } from 'vitest';
import {
  bmiOf,
  classifyMerchant,
  clockText,
  measureValue,
  mealStats,
  spendTotals,
  weekdayAverages,
} from './profileOverview';
import type { CardTransaction } from '../../types';

const row = (merchant: string, amount: number, time: string, type = ''): CardTransaction => ({
  merchant,
  amount,
  time,
  type,
  balance: 0,
  description: '',
});

describe('消费归类', () => {
  it('热水优先于餐饮关键词，未知商户归到其他', () => {
    expect(classifyMerchant('康桥苑餐厅')).toBe('meal');
    expect(classifyMerchant('浴室热水')).toBe('water');
    expect(classifyMerchant('梧桐超市')).toBe('market');
    expect(classifyMerchant('洗衣房')).toBe('other');
  });

  it('只统计支出，金额为 0 的类别不出现', () => {
    const totals = spendTotals([
      row('康桥苑餐厅', -1000, '2026-11-12 12:10'),
      row('梧桐超市', -500, '2026-11-12 18:00'),
      row('充值', 2000, '2026-11-12 09:00'),
    ]);
    expect(totals.total).toBe(1500);
    expect(totals.categories.map((item) => item.key)).toEqual(['meal', 'market']);
    expect(totals.categories[0].share).toBeCloseTo(2 / 3);
  });
});

describe('用餐习惯', () => {
  it('按餐次取时间中位数，并统计日期与最常去的商户', () => {
    const result = mealStats([
      row('康桥苑餐厅', -500, '2026-11-10 07:00'),
      row('康桥苑餐厅', -600, '2026-11-12 07:40'),
      row('梧桐苑食堂', -800, '2026-11-12 12:10'),
      row('康桥苑餐厅', -700, '2026-11-12 18:00'),
    ]);
    const breakfast = result.meals.find((meal) => meal.key === 'breakfast');
    expect(breakfast?.minutes).toBe(440);
    expect(breakfast?.days).toBe(2);
    expect(clockText(breakfast?.minutes ?? null)).toBe('07:20');
    expect(result.meals.find((meal) => meal.key === 'lunch')?.days).toBe(1);
    expect(result.topMerchant).toEqual({ name: '康桥苑餐厅', count: 3 });
  });

  it('没有餐饮支出时中位数为空', () => {
    const result = mealStats([row('梧桐超市', -500, '2026-11-12 12:10')]);
    expect(result.meals.every((meal) => meal.minutes == null)).toBe(true);
    expect(result.topMerchant).toBeNull();
  });
});

describe('按星期日均支出', () => {
  it('窗口内每个星期几按实际出现次数平均，周一到周日排列', () => {
    const result = weekdayAverages(
      [row('康桥苑餐厅', -900, '2026-11-12 12:00')],
      3,
      '2026-11-12 09:00',
    );
    // 2026-11-12 是周四，落在周日的索引 3。
    expect(result.labels[3]).toBe('四');
    expect(result.values[3]).toBe(900);
    expect(result.values.filter((value, index) => index !== 3)).toEqual([0, 0, 0, 0, 0, 0]);
  });
});

describe('体格原始值', () => {
  it('只接受合理的身高体重', () => {
    expect(bmiOf(176.2, 66)).toBe(21.3);
    expect(bmiOf(null, 66)).toBeNull();
    expect(bmiOf(176.2, 0)).toBeNull();
    expect(bmiOf(10, 66)).toBeNull();
  });

  it('空白与非数字当作没有测量值', () => {
    expect(measureValue('4210')).toBe(4210);
    expect(measureValue(176.2)).toBe(176.2);
    expect(measureValue('')).toBeNull();
    expect(measureValue('—')).toBeNull();
  });
});
