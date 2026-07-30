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
  it('explains every visible financial-action field in plain language', () => {
    render(
      <FinancialPolicyCreateDialog
        open
        onOpenChange={vi.fn()}
        contextQuery="?workspace=demo&environment=production"
      />,
    );

    const expectedGuidance = [
      'Choose whether this policy governs money-moving actions or Gateway LLM spend.',
      'Use a stable lowercase identifier that will appear in policy lists, logs, and API responses.',
      'Only actions from this agent id match. Use the same id your SDK sends.',
      'Explain what this control protects so teammates can recognize it later.',
      'Optional operation name sent by the integration, such as issue_refund. It must match exactly.',
      'Use a three-letter currency code, such as USD. The amount fields use this currency.',
      'Select the typed action this policy evaluates: refund, payment, or payout.',
      'Select how the money moves. The action must report the same rail to match.',
      'Threshold checked against each action. Cap breach decides what happens when it is exceeded.',
      'Actions above this amount require approval. A hard cap can still deny the action.',
      'Cumulative threshold per UTC day across matching actions. Leave blank to skip this window.',
      'Cumulative threshold per UTC week across matching actions. Leave blank to skip this window.',
      'Cumulative threshold per UTC month across matching actions. Leave blank to skip this window.',
      'Effect returned when an action exceeds one of the configured caps.',
      'Effect returned when required evidence was not provided.',
      'Effect returned when supplied evidence says a required precondition is false.',
    ];

    for (const guidance of expectedGuidance) {
      expect(screen.getByText(guidance)).toBeInTheDocument();
    }
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
    expect(
      screen.getByText(
        'Limit this budget to one runtime principal. Leave blank to meter every principal separately.',
      ),
    ).toBeInTheDocument();
    expect(
      screen.getByText(
        'Estimated LLM spend threshold per principal per UTC week. Leave blank to skip this window.',
      ),
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

  it('explains the evidence choices shown for refund policies', () => {
    const policy = x402MandatePolicy();

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

    expect(
      screen.getByText(
        'Select the facts the caller must provide and satisfy before a refund can be authorized.',
      ),
    ).toBeInTheDocument();
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
