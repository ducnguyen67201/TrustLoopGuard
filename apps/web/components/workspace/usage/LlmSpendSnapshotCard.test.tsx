import { cleanup, render, screen, within } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { LlmUsageBucket } from '@trustloopguard/sdk';

import { LlmSpendSnapshotCard } from './LlmSpendSnapshotCard';

afterEach(() => {
  cleanup();
});

describe('LlmSpendSnapshotCard', () => {
  it('shows this-week total, top 3 spenders heaviest-first, and a detail link', () => {
    render(
      <LlmSpendSnapshotCard
        detailHref="/usage?workspace=demo&environment=production"
        principalBuckets={[
          bucket('light-bot', 100n),
          bucket('heavy-bot', 5000n),
          bucket('mid-bot', 2000n),
          bucket('tiny-bot', 5n),
        ]}
      />,
    );

    // 100 + 5000 + 2000 + 5 minor units = $71.05 total.
    expect(screen.getByText('$71.05')).toBeInTheDocument();

    const list = screen.getByRole('list');
    const items = within(list).getAllByRole('listitem');
    // Only the top 3 render, heaviest first.
    expect(items).toHaveLength(3);
    expect(items[0]).toHaveTextContent('heavy-bot');
    expect(items[1]).toHaveTextContent('mid-bot');
    expect(items[2]).toHaveTextContent('light-bot');
    expect(screen.queryByText('tiny-bot')).not.toBeInTheDocument();

    expect(screen.getByRole('link', { name: /view detail/i })).toHaveAttribute(
      'href',
      '/usage?workspace=demo&environment=production',
    );
  });

  it('teaches instead of showing a bare zero when there is no spend', () => {
    render(<LlmSpendSnapshotCard detailHref="/usage" principalBuckets={[]} />);

    expect(screen.getByText('$0.00')).toBeInTheDocument();
    expect(screen.getByText(/no spend yet this week/i)).toBeInTheDocument();
    expect(screen.queryByRole('link', { name: /view detail/i })).not.toBeInTheDocument();
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
