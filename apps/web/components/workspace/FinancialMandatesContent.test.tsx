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
  it('creates mandates through the same-origin financial route', async () => {
    const fetchMock = vi.fn<typeof fetch>(async () =>
      new Response(JSON.stringify(mandate('mandate_new', 'active')), { status: 201 }),
    );
    vi.stubGlobal('fetch', fetchMock);
    const user = userEvent.setup();
    render(
      <FinancialMandatesContent
        workspaceSlug="demo"
        environmentId="production"
        mandates={[]}
      />,
    );

    await user.type(screen.getByLabelText('Principal'), 'refund-bot');
    await user.click(screen.getByRole('button', { name: /create/i }));

    await waitFor(() => {
      expect(fetchMock).toHaveBeenCalledWith(
        '/api/financial/mandates?workspace=demo&environment=production',
        {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: expect.stringContaining('"principal_id":"refund-bot"'),
        },
      );
    });
    expect(screen.getByText('mandate_new')).toBeInTheDocument();
  });

  it('revokes active mandates through the same-origin financial route', async () => {
    const fetchMock = vi.fn<typeof fetch>(async () =>
      new Response(JSON.stringify(mandate('mandate_1', 'revoked')), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);
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
});

function mandate(id: string, status: FinancialMandate['status']): FinancialMandate {
  return {
    id,
    workspace_id: 'ws_1',
    version: 1,
    status,
    principal_id: 'refund-bot',
    scope: { action_kinds: ['refund'], currency: 'USD' },
    metadata: {},
    created_at: '2026-07-05T20:00:00Z',
    updated_at: '2026-07-05T20:00:00Z',
  };
}
