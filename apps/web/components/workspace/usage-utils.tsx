// Formatting and period math shared by the /usage page and UsageContent.
//
// Buckets from GET /v1/llm-usage carry no currency: gateway pricing is USD
// minor units (see crates/tl-server/src/llm_pricing.rs), so callers pass the
// currency explicitly and default to USD.

export type UsagePeriod = 'day' | 'week' | 'month';

export const DEFAULT_USAGE_PERIOD: UsagePeriod = 'week';
export const USAGE_PERIODS: readonly UsagePeriod[] = ['day', 'week', 'month'];

export function readUsagePeriod(value: string | null): UsagePeriod {
  return value === 'day' || value === 'week' || value === 'month'
    ? value
    : DEFAULT_USAGE_PERIOD;
}

/**
 * `[start, end)` window for the period containing `now`, in UTC. Matches the
 * gateway budget engine's windows (crates/tl-server/src/gateway/budget.rs):
 * day from 00:00 UTC, week from Monday 00:00 UTC, month from the 1st.
 */
export function periodRange(period: UsagePeriod, now: Date): { start: Date; end: Date } {
  const dayStart = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  if (period === 'day') {
    return { start: new Date(dayStart), end: new Date(addUtcDays(dayStart, 1)) };
  }
  if (period === 'week') {
    const daysFromMonday = (new Date(dayStart).getUTCDay() + 6) % 7;
    const weekStart = addUtcDays(dayStart, -daysFromMonday);
    return { start: new Date(weekStart), end: new Date(addUtcDays(weekStart, 7)) };
  }
  return {
    start: new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), 1)),
    end: new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() + 1, 1)),
  };
}

const MS_PER_DAY = 24 * 60 * 60 * 1000;

function addUtcDays(epochMs: number, days: number): number {
  return epochMs + days * MS_PER_DAY;
}

/**
 * Currency minor units → display string, e.g. `1234` → `"$12.34"`. Locale is
 * pinned to en-US so server render, client hydration, and tests all agree.
 */
export function formatMinor(costMinor: number | bigint, currency: string): string {
  return (Number(costMinor) / 100).toLocaleString('en-US', {
    style: 'currency',
    currency,
  });
}

const TOKEN_UNITS: readonly { value: number; suffix: string }[] = [
  { value: 1e12, suffix: 'T' },
  { value: 1e9, suffix: 'B' },
  { value: 1e6, suffix: 'M' },
  { value: 1e3, suffix: 'K' },
];

/** Compact token count, e.g. `1_234_567` → `"1.2M"`, `3_200_000_000` → `"3.2B"`. */
export function formatTokens(value: number | bigint): string {
  const count = Number(value);
  const magnitude = Math.abs(count);
  const unit = TOKEN_UNITS.find((candidate) => magnitude >= candidate.value);
  if (unit === undefined) {
    return count.toLocaleString('en-US');
  }
  const scaled = count / unit.value;
  const text =
    Math.abs(scaled) >= 100
      ? Math.round(scaled).toLocaleString('en-US')
      : scaled.toFixed(1).replace(/\.0$/, '');
  return `${text}${unit.suffix}`;
}
