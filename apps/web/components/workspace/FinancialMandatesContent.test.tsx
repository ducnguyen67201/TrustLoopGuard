import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { FinancialMandate } from '@trustloopguard/sdk';

import { FinancialMandatesContent } from './FinancialMandatesContent';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('FinancialMandatesContent', () => {
  it('creates managed payment mandates through the same-origin financial route', async () => {
    const fetchMock = vi.fn<typeof fetch>(
      async () => new Response(JSON.stringify(mandate('mandate_new', 'active')), { status: 201 }),
    );
    vi.stubGlobal('fetch', fetchMock);
    const user = userEvent.setup();
    render(
      <FinancialMandatesContent workspaceSlug="demo" environmentId="production" mandates={[]} />,
    );

    await user.clear(screen.getByLabelText('Agent principal'));
    await user.type(screen.getByLabelText('Agent principal'), 'spid:commerce-agent');
    await user.click(screen.getByRole('button', { name: /create managed mandate/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/financial/mandates?workspace=demo&environment=production',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: expect.stringContaining('"payment_scope"'),
        },
      );
    });
    const body = JSON.parse(String(fetchMock.mock.calls[0]?.[1]?.body)) as {
      principal_id?: string;
      payment_scope?: { rail?: string; action_kinds?: string[]; allowed_pay_to?: string[] };
      metadata?: { raw_user_request?: string; user_intent?: string };
    };
    expect(body.principal_id).toBe('spid:commerce-agent');
    expect(body.payment_scope?.rail).toBe('x402');
    expect(body.payment_scope?.action_kinds).toEqual(['payment']);
    expect(body.payment_scope?.allowed_pay_to).toEqual([
      '0xabc1230000000000000000000000000000000000',
    ]);
    expect(body.metadata?.raw_user_request).toContain('User asked');
    expect(body.metadata?.user_intent).toContain('spid:commerce-agent');
    expect(screen.getByText('mandate_new')).toBeInTheDocument();
  });

  it('revokes active mandates through the same-origin financial route', async () => {
    const fetchMock = vi.fn<typeof fetch>(
      async () => new Response(JSON.stringify(mandate('mandate_1', 'revoked')), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);
    vi.stubGlobal(
      'confirm',
      vi.fn(() => true),
    );
    render(
      <FinancialMandatesContent
        workspaceSlug="demo"
        environmentId="production"
        mandates={[mandate('mandate_1', 'active')]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /revoke mandate mandate_1/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/financial/mandates/mandate_1/revoke?workspace=demo&environment=production',
        { method: 'POST' },
      );
    });
    expect(screen.getByText('Revoked')).toBeInTheDocument();
  });

  it('labels TrustLoopGuard-managed mandates as internal', () => {
    render(
      <FinancialMandatesContent
        workspaceSlug="demo"
        environmentId="production"
        mandates={[mandate('mandate_1', 'active')]}
      />,
    );

    expect(screen.getByText('Internal')).toBeInTheDocument();
    expect(screen.getAllByText(/User asked:/).length).toBeGreaterThan(0);
  });

  it('does not revoke when confirmation is cancelled', async () => {
    const fetchMock = vi.fn<typeof fetch>();
    vi.stubGlobal('fetch', fetchMock);
    vi.stubGlobal(
      'confirm',
      vi.fn(() => false),
    );
    render(
      <FinancialMandatesContent
        workspaceSlug="demo"
        environmentId="production"
        mandates={[mandate('mandate_1', 'active')]}
      />,
    );

    await userEvent.click(screen.getByRole('button', { name: /revoke mandate mandate_1/i }));

    expect(fetchMock).not.toHaveBeenCalled();
  });
});

function mandate(id: string, status: FinancialMandate['status']): FinancialMandate {
  return {
    id,
    workspace_id: 'ws_1',
    version: 1,
    status,
    principal_id: 'refund-bot',
    scope: {
      action_kinds: ['payment'],
      currency: 'USD',
      max_amount_minor: 500,
      allowed_hosts: ['127.0.0.1:4021'],
      allowed_resources: ['/premium/article/agentic-commerce'],
    },
    metadata: {
      mandate_mode: 'trustloop_managed',
      raw_user_request: 'User asked: "Buy access to the premium article about agentic commerce."',
      user_intent:
        'Allow spid:commerce-agent to pay up to $5.00 for /premium/article/agentic-commerce.',
    },
    created_at: '2026-07-05T20:00:00Z',
    updated_at: '2026-07-05T20:00:00Z',
  };
}
