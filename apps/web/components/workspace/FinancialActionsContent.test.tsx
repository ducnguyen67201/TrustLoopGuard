import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { FinancialActionRecord, FinancialActionStatus } from '@trustloopguard/sdk';

import { FinancialActionsContent } from './FinancialActionsContent';

afterEach(() => {
  cleanup();
});

describe('FinancialActionsContent', () => {
  it('renders held, executed, denied, failed, and reversed action states', () => {
    render(
      <FinancialActionsContent
        workspaceSlug="demo"
        environmentId="production"
        actions={[
          action('act_held', 'held', 7_500),
          action('act_executed', 'executed', 4_000),
          action('act_denied', 'denied', 9_000),
          action('act_failed', 'failed', 6_000),
          action('act_reversed', 'reversed', 3_000),
        ]}
        outcomesByActionId={{
          act_executed: [
            {
              action_id: 'act_executed',
              status: 'succeeded',
              reversal_capability: 'provider_reversal',
              recovery_status: 'not_needed',
              provider_status: 'succeeded',
              provider_reference: 'pay_123',
              occurred_at: '2026-07-05T20:00:00Z',
              metadata: {},
            },
          ],
        }}
        familyPolicies={[
          {
            family: 'payment',
            id: 'pay-alice',
            when: { agents: ['alice'], operations: ['pay'] },
            per_transaction_minor: 10_000,
            hold_above_minor: 5_000,
            daily_minor: 50_000,
            monthly_minor: null,
          },
        ]}
        providerConnections={[
          {
            id: 'provider_1',
            display_name: 'Refund provider',
            kind: 'payment_http',
            base_url: 'https://payments.test',
            default_model: '',
            credential_status: 'configured',
            created_at: '2026-07-05T20:00:00Z',
            updated_at: '2026-07-05T20:00:00Z',
          },
        ]}
      />,
    );

    expect(screen.getAllByText('Held').length).toBeGreaterThan(0);
    expect(screen.getAllByText('Executed').length).toBeGreaterThan(0);
    expect(screen.getByText('Denied')).toBeInTheDocument();
    expect(screen.getByText('Failed')).toBeInTheDocument();
    expect(screen.getByText('Reversed')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /receipt/i })).toHaveAttribute(
      'href',
      '/financial/receipts/act_executed?workspace=demo&environment=production',
    );
    expect(screen.getByText('Refund provider')).toBeInTheDocument();
  });
});

function action(
  id: string,
  status: FinancialActionStatus,
  amountMinor: number,
): FinancialActionRecord {
  return {
    id,
    workspace_id: 'ws_1',
    status,
    action: {
      id,
      kind: 'refund',
      principal_id: 'refund-bot',
      amount: { amount_minor: BigInt(amountMinor), currency: 'USD' },
      counterparty: {
        id: 'cust_1',
        display_name: 'Casey Customer',
        kind: 'customer',
        metadata: {},
      },
      rail: 'payment_http',
      metadata: {},
    },
    evidence: [],
    created_at: '2026-07-05T20:00:00Z',
    updated_at: '2026-07-05T20:00:00Z',
  };
}
