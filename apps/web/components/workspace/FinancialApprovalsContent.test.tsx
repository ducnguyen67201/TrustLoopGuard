import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { FinancialActionRecord } from '@trustloopguard/sdk';

import { FinancialApprovalsContent } from './FinancialApprovalsContent';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('FinancialApprovalsContent', () => {
  it('approves and resumes execution through same-origin financial routes', async () => {
    const fetchMock = vi.fn<typeof fetch>(async (input) => {
      const url = typeof input === 'string' ? input : input.toString();
      if (url.includes('/approve')) {
        return new Response(JSON.stringify(apiAction('authorized')), {
          status: 200,
        });
      }
      if (url.includes('/execute')) {
        return new Response(JSON.stringify(apiAction('executed')), {
          status: 200,
        });
      }
      return new Response(JSON.stringify({ error: 'unexpected' }), { status: 500 });
    });
    vi.stubGlobal('fetch', fetchMock);

    render(
      <FinancialApprovalsContent
        workspaceSlug="demo"
        environmentId="production"
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
        actions={[heldAction()]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /approve financial action act_held/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/financial/actions/act_held/approve?workspace=demo&environment=production',
        { method: 'POST' },
      );
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/financial/actions/act_held/execute?workspace=demo&environment=production',
        { method: 'POST' },
      );
    });
    expect(screen.queryByText('above threshold')).not.toBeInTheDocument();
  });

  it('denies through the same-origin financial route', async () => {
    const fetchMock = vi.fn<typeof fetch>(async () =>
      new Response(JSON.stringify(apiAction('denied')), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);

    render(
      <FinancialApprovalsContent
        workspaceSlug="demo"
        environmentId="production"
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
        actions={[heldAction()]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /deny financial action act_held/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/financial/actions/act_held/deny?workspace=demo&environment=production',
        { method: 'POST' },
      );
    });
  });
});

function heldAction(): FinancialActionRecord {
  return {
    id: 'act_held',
    workspace_id: 'ws_1',
    status: 'held',
    action: {
      id: 'act_held',
      kind: 'refund',
      operation: 'issue_refund',
      principal_id: 'refund-bot',
      amount: { amount_minor: 7_500n, currency: 'USD' },
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

function apiAction(status: FinancialActionRecord['status']) {
  return {
    ...heldAction(),
    status,
    action: {
      ...heldAction().action,
      amount: { amount_minor: 7_500, currency: 'USD' },
    },
  };
}
