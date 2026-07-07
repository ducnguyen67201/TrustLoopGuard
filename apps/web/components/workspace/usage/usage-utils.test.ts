import { describe, expect, it } from 'vitest';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import {
  bySpendDescending,
  formatTokens,
  periodRange,
  readUsagePeriod,
  spendIntensity,
} from './usage-utils';

describe('formatTokens', () => {
  it('formats big numbers compactly', () => {
    expect(formatTokens(0)).toBe('0');
    expect(formatTokens(999)).toBe('999');
    expect(formatTokens(1_234)).toBe('1.2K');
    expect(formatTokens(1_234_567)).toBe('1.2M');
    expect(formatTokens(3_200_000_000)).toBe('3.2B');
    expect(formatTokens(3_200_000_000n)).toBe('3.2B');
    expect(formatTokens(1_000_000)).toBe('1M');
    expect(formatTokens(2_500_000_000_000)).toBe('2.5T');
  });
});

describe('readUsagePeriod', () => {
  it('defaults to week', () => {
    expect(readUsagePeriod(null)).toBe('week');
    expect(readUsagePeriod('bogus')).toBe('week');
    expect(readUsagePeriod('day')).toBe('day');
    expect(readUsagePeriod('month')).toBe('month');
  });
});

describe('periodRange', () => {
  it('day covers today 00:00 UTC to tomorrow', () => {
    const { start, end } = periodRange('day', new Date('2026-07-06T15:30:00Z'));
    expect(start.toISOString()).toBe('2026-07-06T00:00:00.000Z');
    expect(end.toISOString()).toBe('2026-07-07T00:00:00.000Z');
  });

  it('week starts Monday 00:00 UTC, matching the budget engine window', () => {
    // 2026-07-06 is a Monday.
    const monday = periodRange('week', new Date('2026-07-06T01:00:00Z'));
    expect(monday.start.toISOString()).toBe('2026-07-06T00:00:00.000Z');
    expect(monday.end.toISOString()).toBe('2026-07-13T00:00:00.000Z');

    // 2026-07-05 is a Sunday → the week began the previous Monday.
    const sunday = periodRange('week', new Date('2026-07-05T23:59:59Z'));
    expect(sunday.start.toISOString()).toBe('2026-06-29T00:00:00.000Z');
    expect(sunday.end.toISOString()).toBe('2026-07-06T00:00:00.000Z');
  });

  it('month covers the 1st to the 1st of the next month', () => {
    const { start, end } = periodRange('month', new Date('2026-12-15T12:00:00Z'));
    expect(start.toISOString()).toBe('2026-12-01T00:00:00.000Z');
    expect(end.toISOString()).toBe('2027-01-01T00:00:00.000Z');
  });
});

describe('bySpendDescending', () => {
  it('orders heaviest spend first without mutating the input', () => {
    const input = [bucket('a', 10n), bucket('b', 900n), bucket('c', 50n)];
    const sorted = bySpendDescending(input);
    expect(sorted.map((row) => row.key)).toEqual(['b', 'c', 'a']);
    // Input untouched (immutability).
    expect(input.map((row) => row.key)).toEqual(['a', 'b', 'c']);
  });
});

describe('spendIntensity', () => {
  it('is 1 for the max, a fraction below it, and 0 when there is no max', () => {
    expect(spendIntensity(bucket('a', 500n), 1000)).toBe(0.5);
    expect(spendIntensity(bucket('a', 1000n), 1000)).toBe(1);
    expect(spendIntensity(bucket('a', 0n), 0)).toBe(0);
    // Never exceeds 1 even if a stale max is passed.
    expect(spendIntensity(bucket('a', 2000n), 1000)).toBe(1);
  });
});

function bucket(key: string, cost: bigint): LlmUsageBucket {
  return {
    key,
    prompt_tokens: 100n,
    completion_tokens: 50n,
    cost_minor: cost,
    calls: 3n,
  };
}
