import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';

import type { FamilyPolicyRow } from '@/lib/server/dashboard-data';

import { FinancialPolicyCreateDialog } from './FinancialSpendingControlsCard';

vi.mock('sonner', () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

beforeAll(() => {
  Object.defineProperty(HTMLElement.prototype, 'hasPointerCapture', {
    configurable: true,
    value: vi.fn(() => false),
  });
  Object.defineProperty(HTMLElement.prototype, 'scrollIntoView', {
    configurable: true,
    value: vi.fn(),
  });
});

afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('FinancialPolicyCreateDialog', () => {
  it('creates an LLM usage budget without financial-action selectors', async () => {
    const fetchMock = vi.fn<typeof fetch>(
      async () =>
        new Response(
          JSON.stringify({
            id: 'llm-weekly-budget',
            meter: 'llm_usage',
            weekly_minor: 5000,
          }),
          { status: 200 },
        ),
    );
    vi.stubGlobal('fetch', fetchMock);

    render(
      <FinancialPolicyCreateDialog
        open
        onOpenChange={vi.fn()}
        contextQuery="?workspace=demo&environment=production"
      />,
    );

    const meterSelect = screen.getAllByRole('combobox')[0];
    if (meterSelect === undefined) {
      throw new Error('expected meter select');
    }
    await userEvent.click(meterSelect);
    await userEvent.click(await screen.findByText('LLM usage (gateway)'));
    await userEvent.click(screen.getByRole('button', { name: 'Create financial policy' }));

    expect(fetchMock).toHaveBeenCalledTimes(1);
    const call = fetchMock.mock.calls[0];
    if (call === undefined) {
      throw new Error('expected fetch call');
    }
    const [url, init] = call;
    expect(String(url)).toBe('/api/financial/policies?workspace=demo&environment=production');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({
      id: 'llm-weekly-budget',
      description: 'Weekly LLM spend cap per principal',
      severity: 'high',
      meter: 'llm_usage',
      when: {},
      weekly_minor: 5000,
      on_breach: 'block',
    });
  });

  it('shows x402 mandate-required policies as payment policies and preserves mandate checks', async () => {
    const policy = x402MandatePolicy();
    const fetchMock = vi.fn<typeof fetch>(
      async () => new Response(JSON.stringify(policy), { status: 200 }),
    );
    vi.stubGlobal('fetch', fetchMock);

    render(
      <FinancialPolicyCreateDialog
        open
        onOpenChange={vi.fn()}
        contextQuery="?workspace=demo&environment=production"
        initialPolicy={policy}
        existingPolicyIds={[policy.id]}
      />,
    );

    expect(screen.getByText('x402')).toBeInTheDocument();
    expect(screen.getByText('Require user intent proof')).toBeInTheDocument();
    expect(screen.getByRole('checkbox', { name: /require user intent proof/i })).toBeChecked();
    expect(screen.queryByText('Required refund evidence')).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole('button', { name: /save financial policy/i }));

    await waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const [, init] = fetchMock.mock.calls[0] ?? [];
    expect(init).toMatchObject({
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
    });
    const body = JSON.parse(String(init?.body)) as {
      mandate_required?: boolean;
      required_preconditions?: string[];
      when?: { rails?: string[]; action_kinds?: string[] };
    };
    expect(body.mandate_required).toBe(true);
    expect(body.required_preconditions).toEqual([]);
    expect(body.when?.rails).toEqual(['x402']);
    expect(body.when?.action_kinds).toEqual(['payment']);
  });
});

function x402MandatePolicy(): FamilyPolicyRow {
  return {
    id: 'x402-agentic-payment-mandate-required',
    description: 'Sandbox policy: x402 agentic payments must present an active mandate',
    severity: 'high',
    meter: 'actions',
    when: {
      agents: ['spid:commerce-agent'],
      action_kinds: ['payment'],
      operations: ['x402_read_paid_resource'],
      currencies: ['USD'],
      rails: ['x402'],
    },
    per_transaction_minor: 500,
    hold_above_minor: null,
    daily_minor: 5000,
    weekly_minor: null,
    monthly_minor: null,
    mandate_required: true,
    required_preconditions: [],
    missing_evidence_action: 'escalate',
    failed_precondition_action: 'block',
    on_breach: 'block',
    enabled: true,
  };
}
