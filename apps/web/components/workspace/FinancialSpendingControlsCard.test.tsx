import { cleanup, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';

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

beforeEach(() => {
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
  vi.unstubAllGlobals();
  vi.clearAllMocks();
});

describe('FinancialPolicyCreateDialog', () => {
  it('keeps financial-action guidance in information tooltips', async () => {
    const user = userEvent.setup();

    render(
      <FinancialPolicyCreateDialog
        open
        onOpenChange={vi.fn()}
        contextQuery="?workspace=demo&environment=production"
      />,
    );

    const fieldsWithGuidance = [
      'Applies to',
      'Control id',
      'Agent',
      'Description',
      'Operation',
      'Currency',
      'Action kind',
      'Rail',
      'Per-action cap',
      'Require approval above',
      'Daily cap',
      'Weekly cap',
      'Monthly cap',
      'Require user intent proof',
      'Cap breach',
      'Missing evidence',
      'Failed evidence',
    ];

    for (const field of fieldsWithGuidance) {
      expect(
        screen.getByRole('button', { name: `More information about ${field}` }),
      ).toBeInTheDocument();
    }
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument();

    const capGuidance =
      'Threshold checked against each action. Cap breach decides what happens when it is exceeded.';
    expect(screen.queryByText(capGuidance)).not.toBeInTheDocument();

    const capHelp = screen.getByRole('button', {
      name: 'More information about Per-action cap',
    });
    await user.hover(capHelp);

    expect(await screen.findByRole('tooltip')).toHaveTextContent(capGuidance);

    const intentDetails = /Where it comes from:/i;
    expect(screen.queryByText(intentDetails)).not.toBeInTheDocument();
    await user.hover(
      screen.getByRole('button', { name: 'More information about Require user intent proof' }),
    );
    await waitFor(() => expect(screen.getByRole('tooltip')).toHaveTextContent(intentDetails));
  });

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
    expect(
      screen.getByText(/requests with max_tokens get strict preflight enforcement/i),
    ).toBeInTheDocument();
    const principalGuidance =
      'Limit this budget to one runtime principal. Leave blank to meter every principal separately.';
    expect(screen.queryByText(principalGuidance)).not.toBeInTheDocument();
    await userEvent.hover(
      screen.getByRole('button', { name: 'More information about Principal (optional)' }),
    );
    expect(await screen.findByRole('tooltip')).toHaveTextContent(principalGuidance);
    expect(
      screen.getByRole('button', { name: 'More information about Weekly cap' }),
    ).toBeInTheDocument();
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
      on_breach: 'deny',
    });
  });

  it('shows x402 grant-required policies as payment policies and preserves authority checks', async () => {
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
      grant_required?: boolean;
      required_preconditions?: string[];
      when?: { rails?: string[]; action_kinds?: string[] };
    };
    expect(body.grant_required).toBe(true);
    expect(body.required_preconditions).toEqual([]);
    expect(body.when?.rails).toEqual(['x402']);
    expect(body.when?.action_kinds).toEqual(['payment']);
  });

  it('explains the evidence choices shown for refund policies in a tooltip', async () => {
    const policy = x402MandatePolicy();
    const user = userEvent.setup();

    render(
      <FinancialPolicyCreateDialog
        open
        onOpenChange={vi.fn()}
        contextQuery="?workspace=demo&environment=production"
        initialPolicy={{
          ...policy,
          id: 'refund-evidence-policy',
          when: {
            ...policy.when,
            action_kinds: ['refund'],
            rails: ['payment_http'],
          },
          required_preconditions: ['order_exists'],
        }}
      />,
    );

    const evidenceGuidance =
      'Select the facts the caller must provide and satisfy before a refund can be authorized.';
    expect(screen.queryByText(evidenceGuidance)).not.toBeInTheDocument();

    await user.hover(
      screen.getByRole('button', {
        name: 'More information about Required refund evidence',
      }),
    );

    expect(await screen.findByRole('tooltip')).toHaveTextContent(evidenceGuidance);
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
    approval_threshold_minor: null,
    daily_minor: 5000,
    weekly_minor: null,
    monthly_minor: null,
    grant_required: true,
    required_preconditions: [],
    missing_evidence_effect: 'require_approval',
    failed_precondition_effect: 'deny',
    on_breach: 'deny',
    enabled: true,
  };
}
