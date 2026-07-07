import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { UsageContent } from './UsageContent';
import type { UsageBuckets } from './usage';

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

describe('UsageContent (drill-in composition)', () => {
  it('renders the full board with summary tiles and every widget', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        buckets={buckets({
          day: [bucket('2026-07-01'), bucket('2026-07-02')],
          principal: [
            bucket('refund-bot', { prompt: 1_000_000n, completion: 234_567n, cost: 1234n }),
            bucket('support-bot', { prompt: 500n, completion: 250n, cost: 89n }),
          ],
          model: [bucket('gpt-5.2', { prompt: 1_000_500n, completion: 234_817n, cost: 1323n })],
        })}
      />,
    );

    expect(screen.getByText('Total spend')).toBeInTheDocument();
    expect(screen.getByText('Active principals')).toBeInTheDocument();
    expect(screen.getByText('Spend over time')).toBeInTheDocument();
    expect(screen.getByText('Spend by principal')).toBeInTheDocument();
    expect(screen.getByText('Spend by model')).toBeInTheDocument();
    expect(screen.getByText('gpt-5.2')).toBeInTheDocument();
    expect(screen.queryByText('No usage yet')).not.toBeInTheDocument();
  });

  it('sorts principals by spend so the heaviest spender is the first row', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        buckets={buckets({
          principal: [
            bucket('light-bot', { cost: 89n }),
            bucket('heavy-bot', { cost: 9999n }),
          ],
        })}
      />,
    );

    const rows = screen.getAllByRole('row');
    // Header row is index 0; the heaviest spender leads the body.
    expect(rows[1]).toHaveTextContent('heavy-bot');
  });

  it('links principal rows to the focused drill-in with ?principal=', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        buckets={buckets({ principal: [bucket('refund-bot', { cost: 500n })] })}
      />,
    );

    const link = screen.getByRole('link', { name: 'refund-bot' });
    expect(link).toHaveAttribute(
      'href',
      '/usage?workspace=demo&environment=production&principal=refund-bot',
    );
  });

  it('focuses one principal: hides the by-principal table and shows a back link', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        buckets={buckets({
          principalId: 'refund-bot',
          day: [bucket('2026-07-01', { cost: 500n })],
          principal: [bucket('refund-bot', { cost: 500n })],
          model: [bucket('gpt-5.2', { cost: 500n })],
        })}
      />,
    );

    expect(screen.getByRole('heading', { name: 'refund-bot' })).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /all principals/i })).toBeInTheDocument();
    expect(screen.getByText('Spend by model')).toBeInTheDocument();
    expect(screen.queryByText('Spend by principal')).not.toBeInTheDocument();
    expect(screen.queryByText('Active principals')).not.toBeInTheDocument();
  });

  it('renders the teaching empty state when there is no usage', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        buckets={buckets({})}
      />,
    );

    expect(screen.getByText('No usage yet')).toBeInTheDocument();
    expect(screen.getByText(/point an agent at the gateway/i)).toBeInTheDocument();
  });

  it('period links preserve workspace, environment, and focus', () => {
    render(
      <UsageContent
        workspaceSlug="demo"
        environmentId="production"
        buckets={buckets({ principalId: 'refund-bot', principal: [bucket('refund-bot')] })}
      />,
    );

    const monthLink = screen.getByRole('link', { name: /^month$/i });
    expect(monthLink).toHaveAttribute(
      'href',
      '/usage?workspace=demo&environment=production&principal=refund-bot&period=month',
    );
  });
});

function buckets(partial: Partial<UsageBuckets>): UsageBuckets {
  return {
    period: 'week',
    principalId: null,
    day: [],
    principal: [],
    model: [],
    ...partial,
  };
}

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
