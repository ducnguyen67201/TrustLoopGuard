// Formatting and period math shared by the /usage page and UsageContent.

export type UsagePeriod = 'day' | 'week' | 'month';

export const USAGE_PERIODS: readonly UsagePeriod[] = ['day', 'week', 'month'];

export function readUsagePeriod(value: string | null): UsagePeriod {
  return (USAGE_PERIODS as readonly string[]).includes(value ?? '')
    ? (value as UsagePeriod)
    : 'week';
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
