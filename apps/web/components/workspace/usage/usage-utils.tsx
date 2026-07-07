// Formatting, period math, and rollup helpers shared by every usage widget and
// its server loader. Kept framework-free so it imports cleanly on both the
// server (loader) and the client (widgets).

import type { LlmUsageBucket } from '@trustloopguard/sdk';

export type UsagePeriod = 'day' | 'week' | 'month';

export const USAGE_PERIODS: readonly UsagePeriod[] = ['day', 'week', 'month'];

// Gateway pricing is USD minor units (crates/tl-server/src/llm_pricing.rs);
// usage buckets carry no currency field.
export const USAGE_CURRENCY = 'USD';

export function readUsagePeriod(value: string | null): UsagePeriod {
  return (USAGE_PERIODS as readonly string[]).includes(value ?? '')
    ? (value as UsagePeriod)
    : 'week';
}

/** Human label for a period, used by the segmented selector and snapshot copy. */
export function periodLabel(period: UsagePeriod): string {
  return period === 'day' ? 'Today' : period === 'week' ? 'This week' : 'This month';
}

/**
 * `[start, end)` window for the period containing `now`, in UTC. Matches the
 * gateway budget engine's windows (crates/tl-server/src/gateway/budget.rs):
 * day from 00:00 UTC, week from Monday 00:00 UTC, month from the 1st.
 */
export function periodRange(period: UsagePeriod, now: Date): { start: Date; end: Date } {
  const year = now.getUTCFullYear();
  const month = now.getUTCMonth();
  const day = now.getUTCDate();
  if (period === 'day') {
    return {
      start: new Date(Date.UTC(year, month, day)),
      end: new Date(Date.UTC(year, month, day + 1)),
    };
  }
  if (period === 'week') {
    const monday = day - ((new Date(Date.UTC(year, month, day)).getUTCDay() + 6) % 7);
    return {
      start: new Date(Date.UTC(year, month, monday)),
      end: new Date(Date.UTC(year, month, monday + 7)),
    };
  }
  return {
    start: new Date(Date.UTC(year, month, 1)),
    end: new Date(Date.UTC(year, month + 1, 1)),
  };
}

/** Compact token count, e.g. `1_234_567` → `"1.2M"`, `3_200_000_000` → `"3.2B"`. */
export function formatTokens(value: number | bigint): string {
  return Number(value).toLocaleString('en-US', {
    notation: 'compact',
    maximumFractionDigits: 1,
  });
}

export function totalTokens(bucket: LlmUsageBucket): number {
  return Number(bucket.prompt_tokens) + Number(bucket.completion_tokens);
}

export function sumBy(
  buckets: LlmUsageBucket[],
  pick: (bucket: LlmUsageBucket) => number | bigint,
): number {
  return buckets.reduce((total, bucket) => total + Number(pick(bucket)), 0);
}

/**
 * Buckets sorted by spend, descending — the order every principal/model widget
 * renders in, so the heaviest spender is always the top row.
 */
export function bySpendDescending(buckets: LlmUsageBucket[]): LlmUsageBucket[] {
  return [...buckets].sort((a, b) => Number(b.cost_minor) - Number(a.cost_minor));
}

/**
 * Fraction of the largest spend in the set (0–1). Drives the subtle intensity
 * bar behind a principal row so the heavy spender reads without parsing digits.
 * The max row is always 1; an all-zero set yields 0 everywhere (no bar).
 */
export function spendIntensity(bucket: LlmUsageBucket, maxCostMinor: number): number {
  if (maxCostMinor <= 0) return 0;
  return Math.min(1, Number(bucket.cost_minor) / maxCostMinor);
}
