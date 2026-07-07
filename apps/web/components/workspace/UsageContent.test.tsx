import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { UsageContent } from './UsageContent';
import { formatTokens, periodRange, readUsagePeriod } from './usage-utils';

// Recharts' ResponsiveContainer observes its parent element; jsdom has no
// ResizeObserver, so stub a no-op implementation for the chart render.
beforeAll(() => {
  vi.stubGlobal(
    'ResizeObserver',
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});

afterEach(() => {
  cleanup();
});

describe('UsageContent', () => {
  it('renders summary tiles and grouped tables from buckets', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        period="week"
        dayBuckets={[bucket('2026-07-01'), bucket('2026-07-02')]}
        principalBuckets={[
          bucket('refund-bot', { prompt: 1_000_000n, completion: 234_567n, cost: 1234n }),
          bucket('support-bot', { prompt: 500n, completion: 250n, cost: 89n }),
        ]}
        modelBuckets={[bucket('gpt-5.2', { prompt: 1_000_500n, completion: 234_817n, cost: 1323n })]}
      />,
    );

    // Tiles: 1234 + 89 minor units, 1_000_000 + 234_567 + 500 + 250 tokens.
    // Totals also appear in the model table row (same window), so allow >1.
    expect(screen.getByText('Total spend')).toBeInTheDocument();
    expect(screen.getAllByText('$13.23').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Total tokens')).toBeInTheDocument();
    expect(screen.getAllByText('1.2M').length).toBeGreaterThanOrEqual(1);
    expect(screen.getByText('Active principals')).toBeInTheDocument();
    expect(screen.getByText('2')).toBeInTheDocument();

    // Tables.
    expect(screen.getByText('refund-bot')).toBeInTheDocument();
    expect(screen.getByText('support-bot')).toBeInTheDocument();
    expect(screen.getByText('gpt-5.2')).toBeInTheDocument();
    expect(screen.getByText('By principal')).toBeInTheDocument();
    expect(screen.getByText('By model')).toBeInTheDocument();
    expect(screen.getByText('Spend over time')).toBeInTheDocument();
    expect(screen.queryByText('No usage yet')).not.toBeInTheDocument();
  });

  it('renders an empty state with zeroed tiles when there is no usage', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        period="week"
        dayBuckets={[]}
        principalBuckets={[]}
        modelBuckets={[]}
      />,
    );

    expect(screen.getByText('No usage yet')).toBeInTheDocument();
    expect(screen.getByText('$0.00')).toBeInTheDocument();
    expect(screen.getAllByText('0').length).toBeGreaterThanOrEqual(2);
    expect(screen.queryByText('Spend over time')).not.toBeInTheDocument();
    expect(screen.queryByText('By principal')).not.toBeInTheDocument();
  });

  it('period links preserve workspace and environment params', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        period="week"
        dayBuckets={[]}
        principalBuckets={[]}
        modelBuckets={[]}
      />,
    );

    for (const period of ['day', 'week', 'month'] as const) {
      const link = screen.getByRole('link', { name: new RegExp(`^${period}$`, 'i') });
      expect(link).toHaveAttribute(
        'href',
        `/usage?workspace=demo&environment=production&period=${period}`,
      );
    }
    expect(screen.getByRole('link', { name: /week/i })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('link', { name: /day/i })).not.toHaveAttribute('aria-current');
  });
});

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

function bucket(
  key: string,
  totals: { prompt?: bigint; completion?: bigint; cost?: bigint; calls?: bigint } = {},
): LlmUsageBucket {
  return {
    key,
    prompt_tokens: totals.prompt ?? 100n,
    completion_tokens: totals.completion ?? 50n,
    cost_minor: totals.cost ?? 25n,
    calls: totals.calls ?? 3n,
  };
}
