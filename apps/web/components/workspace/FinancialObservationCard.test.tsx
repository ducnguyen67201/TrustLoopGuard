import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';

import { FinancialObservationCard } from './FinancialObservationCard';

afterEach(cleanup);

describe('FinancialObservationCard', () => {
  it('keeps currencies separate and shows reviewed false-positive burden', () => {
    render(
      <FinancialObservationCard
        mode="observe"
        summary={{
          start: '2026-07-01T00:00:00Z',
          end: '2026-07-15T00:00:00Z',
          currencies: [
            currency('USD', 10_000, 4, 1),
            currency('CAD', 7_500, 2, 0),
          ],
          reasons: [],
        }}
      />,
    );

    expect(screen.getByText('Observe')).toBeTruthy();
    expect(screen.getByText('USD exposure')).toBeTruthy();
    expect(screen.getByText('CAD exposure')).toBeTruthy();
    expect(screen.getByText('Reviewed false positives: 1 / 2 (50.0%)')).toBeTruthy();
  });

  it('renders an honest empty state', () => {
    render(
      <FinancialObservationCard
        mode="enforce"
        summary={{ start: '', end: '', currencies: [], reasons: [] }}
      />,
    );
    expect(screen.getByText('No observed financial actions in this reporting window.')).toBeTruthy();
  });
});

function currency(
  currencyCode: string,
  total: number,
  approvalCount: number,
  falsePositiveCount: number,
) {
  return {
    currency: currencyCode,
    total_observed_count: 10n,
    total_observed_amount_minor: BigInt(total),
    would_allow_count: 4n,
    would_allow_amount_minor: 4_000n,
    would_hold_count: BigInt(approvalCount),
    would_hold_amount_minor: 3_000n,
    would_block_count: 2n,
    would_block_amount_minor: 3_000n,
    adverse_count: BigInt(approvalCount + 2),
    adverse_rate_bps: 6_000,
    estimated_approval_count: BigInt(approvalCount),
    estimated_approval_rate_bps: approvalCount * 1_000,
    reviewed_adverse_count: 2n,
    false_positive_count: BigInt(falsePositiveCount),
    false_positive_rate_bps: falsePositiveCount * 5_000,
  };
}
