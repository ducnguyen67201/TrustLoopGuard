import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { FinancialActionRecord, FinancialActionStatus } from '@trustloopguard/sdk';

import { FinancialActionsContent } from './FinancialActionsContent';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
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
        approvals={[]}
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
            id: 'pay-alice-financial',
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
    expect(screen.queryByRole('link', { name: /approvals/i })).not.toBeInTheDocument();
  });

  it('approves held actions inline from the ledger', async () => {
    const fetchMock = vi.fn<typeof fetch>(async (input) => {
      const url = typeof input === 'string' ? input : input.toString();
      if (url.includes('/approve')) {
        return new Response(JSON.stringify(apiAction('act_held', 'authorized', 7_500)), {
          status: 200,
        });
      }
      if (url.includes('/execute')) {
        return new Response(JSON.stringify(apiAction('act_held', 'executed', 7_500)), {
          status: 200,
        });
      }
      return new Response(JSON.stringify({ error: 'unexpected' }), { status: 500 });
    });
    vi.stubGlobal('fetch', fetchMock);

    render(
      <FinancialActionsContent
        workspaceSlug="demo"
        environmentId="production"
        actions={[action('act_held', 'held', 7_500)]}
        approvals={[
          {
            id: 'approval_1',
            workspace_id: 'ws_1',
            action_id: 'act_held',
            status: 'pending',
            reason: 'above threshold',
            approver_roles: [],
            metadata: {},
            created_at: '2026-07-05T20:00:00Z',
            updated_at: '2026-07-05T20:00:00Z',
          },
        ]}
        outcomesByActionId={{}}
        familyPolicies={[]}
        providerConnections={[]}
      />,
    );

    await userEvent.click(
      screen.getByRole('button', { name: /approve financial action act_held/i }),
    );

    expect(fetchMock).toHaveBeenCalledWith(
      '/api/financial/actions/act_held/approve?workspace=demo&environment=production',
      { method: 'POST' },
    );
    expect(fetchMock).toHaveBeenCalledWith(
      '/api/financial/actions/act_held/execute?workspace=demo&environment=production',
      { method: 'POST' },
    );
    await waitFor(() => {
      expect(screen.getByRole('link', { name: /receipt/i })).toHaveAttribute(
        'href',
        '/financial/receipts/act_held?workspace=demo&environment=production',
      );
    });
  });

  it('links financial controls to the policy registry', () => {
    render(
      <FinancialActionsContent
        workspaceSlug="demo"
        environmentId="production"
        actions={[]}
        approvals={[]}
        outcomesByActionId={{}}
        familyPolicies={[
          {
            id: 'refund-bot-refund-controls',
            description: 'Refund controls for support agents',
            severity: 'high',
            when: {
              agents: ['refund-bot'],
              action_kinds: ['refund'],
              operations: ['issue_refund'],
              currencies: ['USD'],
              rails: ['payment_http'],
            },
            per_transaction_minor: 10000,
            hold_above_minor: 5000,
            daily_minor: 50000,
            monthly_minor: 500000,
            required_preconditions: ['order_exists', 'amount_lte_refundable_balance'],
            missing_evidence_action: 'escalate',
            failed_precondition_action: 'block',
            on_breach: 'block',
            enabled: true,
          },
        ]}
        providerConnections={[]}
      />,
    );

    expect(screen.getByText('Policy controls')).toBeInTheDocument();
    expect(screen.getByText('1 active financial policy')).toBeInTheDocument();
    expect(screen.getByText('Refund controls for support agents')).toBeInTheDocument();
    expect(screen.getByRole('link', { name: /policies/i })).toHaveAttribute(
      'href',
      '/policies?workspace=demo&environment=production',
    );
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

function apiAction(id: string, status: FinancialActionStatus, amountMinor: number) {
  return {
    ...action(id, status, amountMinor),
    action: {
      ...action(id, status, amountMinor).action,
      amount: { amount_minor: amountMinor, currency: 'USD' },
    },
  };
}
