import { campusDateKey, campusParts, campusTimestamp } from '../../shared/domain';
import type { CardTransaction } from '../../types';

/**
 * 个人画像 → 概览 的本页统计。分类只用于概览的消费结构图：商户到类别是
 * 本地关键词规则，学校没有提供口径，界面须注明“按商户名归类”。金额一律
 * 用分，负数表示支出（校园卡流水的约定）。
 */
export type SpendCategory = 'meal' | 'market' | 'water' | 'other';

export const SPEND_LABELS: Record<SpendCategory, string> = {
  meal: '餐饮',
  market: '超市',
  water: '热水 · 浴室',
  other: '其他',
};

/** Fixed category order; the overview assigns colours by this list, never by rank. */
export const SPEND_ORDER: SpendCategory[] = ['meal', 'market', 'water', 'other'];

/** 商户归类用本地关键词：顺序有意义，热水先于餐饮，避免“热水”被算成餐饮。 */
export function classifyMerchant(merchant: string, type = ''): SpendCategory {
  const text = `${merchant} ${type}`;
  if (/浴室|热水|水控|淋浴|澡堂/.test(text)) return 'water';
  if (/超市|便利|商店|小卖部|水果|文具|书店/.test(text)) return 'market';
  if (/食堂|餐厅|餐|饭|面|粉|饺|包|饼|粥|米线|砂锅|烧烤|麻辣|饮|咖啡|奶茶|馍|菜/.test(text))
    return 'meal';
  return 'other';
}

export interface CategoryTotal {
  key: SpendCategory;
  label: string;
  /** 支出金额，分。 */
  amount: number;
  /** 占总支出的比例，0–1。 */
  share: number;
}

export interface SpendTotals {
  /** 总支出，分。 */
  total: number;
  categories: CategoryTotal[];
}

/** 按类别汇总支出；金额为 0 的类别不出现，避免空扇区。 */
export function spendTotals(rows: CardTransaction[]): SpendTotals {
  const totals = new Map<SpendCategory, number>();
  let total = 0;
  for (const row of rows) {
    if (row.amount >= 0) continue;
    const key = classifyMerchant(row.merchant, row.type);
    const amount = -row.amount;
    totals.set(key, (totals.get(key) ?? 0) + amount);
    total += amount;
  }
  return {
    total,
    categories: SPEND_ORDER.filter((key) => (totals.get(key) ?? 0) > 0).map((key) => {
      const amount = totals.get(key) ?? 0;
      return { key, label: SPEND_LABELS[key], amount, share: total ? amount / total : 0 };
    }),
  };
}

export interface MealStat {
  key: 'breakfast' | 'lunch' | 'dinner';
  label: string;
  /** 刷卡时间中位数，换算成当天第几分钟；没有记录时为 null。 */
  minutes: number | null;
  /** 有记录的日期数。 */
  days: number;
}

const MEAL_WINDOWS: { key: MealStat['key']; label: string; start: number; end: number }[] = [
  { key: 'breakfast', label: '早餐', start: 5 * 60, end: 10 * 60 },
  { key: 'lunch', label: '午餐', start: 10 * 60, end: 16 * 60 },
  { key: 'dinner', label: '晚餐', start: 16 * 60, end: 22 * 60 },
];

function median(values: number[]): number | null {
  if (!values.length) return null;
  const sorted = [...values].sort((a, b) => a - b);
  const middle = Math.floor(sorted.length / 2);
  return sorted.length % 2 ? sorted[middle] : Math.round((sorted[middle - 1] + sorted[middle]) / 2);
}

export function clockText(minutes: number | null): string {
  if (minutes == null) return '—';
  return `${String(Math.floor(minutes / 60)).padStart(2, '0')}:${String(minutes % 60).padStart(2, '0')}`;
}

/**
 * 用餐时间中位数与最常去的餐厅。只统计被归为餐饮的支出；同一餐次一天刷
 * 多次也按多次计入中位数，日期数按去重后的日期算。
 */
export function mealStats(rows: CardTransaction[]): {
  meals: MealStat[];
  topMerchant: { name: string; count: number } | null;
} {
  const buckets = new Map<MealStat['key'], { times: number[]; dates: Set<string> }>(
    MEAL_WINDOWS.map((meal) => [meal.key, { times: [], dates: new Set() }]),
  );
  const merchants = new Map<string, number>();
  for (const row of rows) {
    if (row.amount >= 0) continue;
    if (classifyMerchant(row.merchant, row.type) !== 'meal') continue;
    const parts = campusParts(row.time);
    if (!parts) continue;
    const minutes = parts.hour * 60 + parts.minute;
    const meal = MEAL_WINDOWS.find((item) => minutes >= item.start && minutes < item.end);
    if (!meal) continue;
    const bucket = buckets.get(meal.key)!;
    bucket.times.push(minutes);
    bucket.dates.add(campusDateKey(row.time));
    const name = row.merchant.trim();
    if (name) merchants.set(name, (merchants.get(name) ?? 0) + 1);
  }
  const top = [...merchants.entries()].sort((a, b) => b[1] - a[1])[0];
  return {
    meals: MEAL_WINDOWS.map((meal) => {
      const bucket = buckets.get(meal.key)!;
      return {
        key: meal.key,
        label: meal.label,
        minutes: median(bucket.times),
        days: bucket.dates.size,
      };
    }),
    topMerchant: top ? { name: top[0], count: top[1] } : null,
  };
}

export interface WeekdaySpend {
  /** 周一到周日。 */
  labels: string[];
  /** 每个星期几的日均支出，分。 */
  values: number[];
}

/** 近 `days` 天按星期几的日均支出；窗口内每个星期几都按实际出现次数平均。 */
export function weekdayAverages(
  rows: CardTransaction[],
  days = 30,
  now: number | string | Date = Date.now(),
): WeekdaySpend {
  const labels = ['一', '二', '三', '四', '五', '六', '日'];
  const today = campusDateKey(now);
  const values = new Array(7).fill(0) as number[];
  const counts = new Array(7).fill(0) as number[];
  const start = campusTimestamp(`${today} 00:00`);
  for (let offset = 0; offset < days; offset++) {
    const parts = campusParts(start - offset * 86400000);
    if (parts) counts[(parts.weekday + 6) % 7] += 1;
  }
  for (const row of rows) {
    if (row.amount >= 0) continue;
    const parts = campusParts(row.time);
    if (!parts) continue;
    values[(parts.weekday + 6) % 7] += -row.amount;
  }
  return {
    labels,
    values: values.map((value, index) => (counts[index] ? Math.round(value / counts[index]) : 0)),
  };
}

/** 身高体重换成 BMI；数值不合理时返回 null，不生成假的体格数据。 */
export function bmiOf(heightCm: number | null, weightKg: number | null): number | null {
  if (heightCm == null || weightKg == null) return null;
  if (heightCm < 80 || heightCm > 250 || weightKg < 20 || weightKg > 300) return null;
  return Math.round((weightKg / (heightCm / 100) ** 2) * 10) / 10;
}

/** 解析接口返回的原始值；空白或非数字都当作没有。 */
export function measureValue(value: unknown): number | null {
  const text = typeof value === 'number' ? String(value) : String(value ?? '').trim();
  if (!/^\d+(\.\d+)?$/.test(text)) return null;
  const number = Number(text);
  return Number.isFinite(number) && number > 0 ? number : null;
}
