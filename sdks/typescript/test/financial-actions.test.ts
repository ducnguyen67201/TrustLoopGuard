import { describe, expect, it } from 'vitest';

import {
  Client,
  type CreateFinancialActionRequest,
  type CreateFinancialMandateRequest,
  type CreateFinancialPolicyRequest,
} from '../src';
import { jsonResponse, mockFetch } from './test-utils';

const REQUEST: CreateFinancialActionRequest = {
  idempotency_key: 'idem-refund-75',
  execute: false,
  action: {
    kind: 'refund',
    operation: 'issue_refund',
    principal_id: 'refund-bot',
    amount: { amount_minor: 7500n, currency: 'USD' },
    counterparty: {
      id: 'cust_456',
      display_name: 'Casey Customer',
      kind: 'customer',
      country: 'US',
      metadata: {},
    },
    rail: 'card',
    memo: 'refund damaged item',
    metadata: { order_id: 'order_123' },
  },
  evidence: [],
};

const ACTION = {
  id: '018f3333-3333-7333-8333-333333333333',
  workspace_id: 'ws_finance',
  status: 'proposed',
  action: {
    ...REQUEST.action,
    id: '018f3333-3333-7333-8333-333333333333',
    amount: { amount_minor: 7500, currency: 'USD' },
  },
  evidence: [],
  created_at: '2026-07-05T00:00:00Z',
  updated_at: '2026-07-05T00:00:00Z',
};

const MANDATE_REQUEST: CreateFinancialMandateRequest = {
  id: 'mandate_refund_bot',
  version: 1,
  principal_id: 'refund-bot',
  scope: { action_kinds: ['refund'], max_amount_minor: 10000, currency: 'USD' },
  metadata: { source: 'sdk_test' },
  expires_at: '2026-08-05T19:00:00Z',
};

const MANDATE = {
  id: 'mandate_refund_bot',
  workspace_id: 'ws_finance',
  version: 1,
  status: 'active',
  principal_id: 'refund-bot',
  scope: MANDATE_REQUEST.scope,
  metadata: MANDATE_REQUEST.metadata,
  expires_at: MANDATE_REQUEST.expires_at,
  created_at: '2026-07-05T00:00:00Z',
  updated_at: '2026-07-05T00:00:00Z',
};

const RECEIPT = {
  id: ACTION.id,
  action_id: ACTION.id,
  trace_id: '018f4444-4444-7444-8444-444444444444',
  ledger_event_ids: ['ledger_execute_1'],
  proof: { action_status: 'executed', provider_reference: 'refund_123' },
  created_at: '2026-07-05T00:00:00Z',
};

const DECISION_RECEIPT = {
  schema: 'financial_action_decision_receipt.v1',
  action_id: ACTION.id,
  decision: 'hold',
  status: 'held',
  reason: 'valid refund, but above threshold so human approval required',
  amount: { amount_minor: 7500, currency: 'USD' },
  operation: 'issue_refund',
  principal_id: 'refund-bot',
  counterparty: ACTION.action.counterparty,
  authorization_scope: {
    checked: true,
    result: 'passed',
    scope_ref: { id: 'mandate_refund_bot', version: 1 },
    source: 'financial_authorization_service',
    reason: 'refund-bot may spend up to USD 100.00',
  },
  evidence: [],
  risks: [
    {
      code: 'amount_above_auto_approve_threshold',
      severity: 'high',
      reason: 'amount at or above hold threshold',
      policy_id: 'refund-controls',
      source: 'financial_policy',
    },
  ],
  approval: undefined,
  execution: { status: 'not_started', ledger_event_ids: [] },
  created_at: ACTION.created_at,
  updated_at: ACTION.updated_at,
};

const OUTCOME = {
  action_id: ACTION.id,
  status: 'succeeded',
  reversal_capability: 'manual_recovery',
  recovery_status: 'manual_required',
  provider_status: 'provider_status',
  provider_reference: 'provider_ref_123',
  occurred_at: '2026-07-05T20:00:00Z',
  metadata: { source: 'ts_sdk_test' },
};

const FINANCIAL_POLICY_REQUEST: CreateFinancialPolicyRequest = {
  id: 'refund-controls',
  description: 'Refund controls',
  severity: 'high',
  when: {
    agents: ['refund-bot'],
    action_kinds: ['refund'],
    operations: ['issue_refund'],
    currencies: ['USD'],
    rails: ['payment_http'],
  },
  meter: 'actions',
  per_transaction_minor: 10000n,
  hold_above_minor: 5000n,
  daily_minor: 50000n,
  monthly_minor: 500000n,
  allowed_counterparty_ids: [],
  denied_counterparty_ids: [],
  hold_new_counterparty: false,
  mandate_required: false,
  approver_roles: [],
  refund_original_method_only: false,
  required_preconditions: ['amount_lte_refundable_balance'],
  missing_evidence_action: 'escalate',
  failed_precondition_action: 'block',
  on_breach: 'block',
};

const FINANCIAL_POLICY = {
  ...FINANCIAL_POLICY_REQUEST,
  per_transaction_minor: 10000,
  hold_above_minor: 5000,
  daily_minor: 50000,
  monthly_minor: 500000,
  enabled: true,
};

describe('Client financial action methods', () => {
  it('verifyAction posts typed financial actions', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(ACTION, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const action = await client.verifyAction(REQUEST);

    expect(action.id).toBe(ACTION.id);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/financial/actions');
    expect((init as RequestInit).method).toBe('POST');
    expect(JSON.parse(String((init as RequestInit).body))).toEqual({
      ...REQUEST,
      action: {
        ...REQUEST.action,
        amount: { amount_minor: 7500, currency: 'USD' },
      },
    });
  });

  it('guardPayment aliases verifyAction for payment/refund ergonomics', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(ACTION, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const action = await client.guardPayment(REQUEST);

    expect(action.status).toBe('proposed');
    expect(fetchSpy).toHaveBeenCalledOnce();
  });

  it('builds financial operation requests with first-class operation identity', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(ACTION, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });
    const issueRefund = client.financialOperation<
      { orderId: string; amountMinor: bigint; reason: string },
      { customerId: string; customerName: string }
    >({
      operation: 'issue_refund',
      kind: 'refund',
      principalId: 'refund-bot',
      rail: 'payment_http',
      amount: (input) => ({ amount_minor: input.amountMinor, currency: 'USD' }),
      idempotencyKey: (input) => `issue_refund:${input.orderId}:${input.amountMinor}`,
      counterparty: (_input, facts) => ({
        id: facts.customerId,
        display_name: facts.customerName,
        kind: 'customer',
        country: 'US',
        metadata: {},
      }),
      memo: (input) => `refund ${input.orderId}: ${input.reason}`,
      metadata: (input) => ({ order_id: input.orderId, reason: input.reason }),
      evidence: (input) => [
        {
          source: 'customer_backend',
          source_id: `eligibility:${input.orderId}`,
          kind: 'refund_eligibility',
          metadata: { order_exists: true },
        },
      ],
    });

    const request = issueRefund.buildRequest(
      { orderId: 'order_123', amountMinor: 7500n, reason: 'damaged_item' },
      { customerId: 'cust_456', customerName: 'Casey Customer' },
      { execute: true },
    );
    expect(request.action.operation).toBe('issue_refund');
    expect(request.execute).toBe(true);

    await issueRefund.verify(
      { orderId: 'order_123', amountMinor: 7500n, reason: 'damaged_item' },
      { customerId: 'cust_456', customerName: 'Casey Customer' },
    );

    const [, init] = fetchSpy.mock.calls[0]!;
    expect(JSON.parse(String((init as RequestInit).body))).toMatchObject({
      idempotency_key: 'issue_refund:order_123:7500',
      action: {
        kind: 'refund',
        operation: 'issue_refund',
        principal_id: 'refund-bot',
        amount: { amount_minor: 7500, currency: 'USD' },
        counterparty: { id: 'cust_456', display_name: 'Casey Customer' },
        rail: 'payment_http',
        memo: 'refund order_123: damaged_item',
        metadata: { order_id: 'order_123', reason: 'damaged_item' },
      },
      evidence: [{ source: 'customer_backend', source_id: 'eligibility:order_123' }],
    });
  });

  it('can read and transition financial actions', async () => {
    const fetchSpy = mockFetch(async (input) => {
      const url = String(input);
      if (url.endsWith('/approve')) return jsonResponse({ ...ACTION, status: 'authorized' });
      if (url.endsWith('/execute')) return jsonResponse({ ...ACTION, status: 'executed' });
      return jsonResponse(ACTION);
    });
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    await expect(client.getFinancialAction(ACTION.id)).resolves.toMatchObject({ id: ACTION.id });
    await expect(client.approveAction(ACTION.id)).resolves.toMatchObject({ status: 'authorized' });
    await expect(client.executeAction(ACTION.id)).resolves.toMatchObject({ status: 'executed' });

    expect(fetchSpy.mock.calls.map(([url]) => String(url))).toEqual([
      `http://server.test/v1/financial/actions/${ACTION.id}`,
      `http://server.test/v1/financial/actions/${ACTION.id}/approve`,
      `http://server.test/v1/financial/actions/${ACTION.id}/execute`,
    ]);
  });

  it('can list financial actions', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse({ actions: [ACTION] }));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const actions = await client.listFinancialActions();

    expect(actions.actions).toHaveLength(1);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/financial/actions');
    expect((init as RequestInit).method).toBe('GET');
  });

  it('can create and list financial spending controls', async () => {
    const fetchSpy = mockFetch(async (input, init) => {
      const url = String(input);
      if (url.endsWith('/policies') && init?.method === 'POST') {
        return jsonResponse(FINANCIAL_POLICY, 201);
      }
      return jsonResponse({ policies: [FINANCIAL_POLICY] });
    });
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    await expect(client.createFinancialPolicy(FINANCIAL_POLICY_REQUEST)).resolves.toMatchObject({
      id: 'refund-controls',
    });
    await expect(client.listFinancialPolicies()).resolves.toMatchObject({
      policies: [FINANCIAL_POLICY],
    });

    const [, createInit] = fetchSpy.mock.calls[0]!;
    expect(JSON.parse(String((createInit as RequestInit).body))).toMatchObject({
      id: 'refund-controls',
      per_transaction_minor: 10000,
      required_preconditions: ['amount_lte_refundable_balance'],
    });
    expect(fetchSpy.mock.calls.map(([url]) => String(url))).toEqual([
      'http://server.test/v1/financial/policies',
      'http://server.test/v1/financial/policies',
    ]);
  });

  it('can create list and revoke financial mandates', async () => {
    const fetchSpy = mockFetch(async (input, init) => {
      const url = String(input);
      if (url.endsWith('/revoke')) return jsonResponse({ ...MANDATE, status: 'revoked' });
      if (url.endsWith('/mandates') && init?.method === 'POST') {
        return jsonResponse(MANDATE, 201);
      }
      return jsonResponse({ mandates: [MANDATE] });
    });
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    await expect(client.createMandate(MANDATE_REQUEST)).resolves.toMatchObject({ id: MANDATE.id });
    await expect(client.listMandates()).resolves.toMatchObject({ mandates: [MANDATE] });
    await expect(client.revokeMandate(MANDATE.id)).resolves.toMatchObject({ status: 'revoked' });

    expect(fetchSpy.mock.calls.map(([url]) => String(url))).toEqual([
      'http://server.test/v1/financial/mandates',
      'http://server.test/v1/financial/mandates',
      `http://server.test/v1/financial/mandates/${MANDATE.id}/revoke`,
    ]);
  });

  it('can list financial approval requests', async () => {
    const fetchSpy = mockFetch(async () =>
      jsonResponse({
        approval_requests: [
          {
            id: 'approval_1',
            workspace_id: 'ws_finance',
            action_id: ACTION.id,
            status: 'pending',
            reason: 'above threshold',
            approver_roles: ['finance'],
            metadata: {},
            created_at: '2026-07-05T00:00:00Z',
            updated_at: '2026-07-05T00:00:00Z',
          },
        ],
      }),
    );
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const approvals = await client.listApprovalRequests();

    expect(approvals.approval_requests).toHaveLength(1);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/financial/approval-requests');
    expect((init as RequestInit).method).toBe('GET');
  });

  it('can fetch financial decision receipts', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(DECISION_RECEIPT));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    await expect(client.getFinancialDecisionReceipt('action/one')).resolves.toMatchObject({
      decision: 'hold',
      risks: [{ code: 'amount_above_auto_approve_threshold' }],
    });

    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/financial/actions/action%2Fone/decision-receipt');
    expect((init as RequestInit).method).toBe('GET');
  });

  it('can fetch financial receipts', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(RECEIPT));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const receipt = await client.getReceipt(ACTION.id);

    expect(receipt.id).toBe(ACTION.id);
    expect(receipt.proof).toMatchObject({ action_status: 'executed' });
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe(`http://server.test/v1/financial/receipts/${ACTION.id}`);
    expect((init as RequestInit).method).toBe('GET');
  });

  it('can record and list financial action outcomes', async () => {
    const fetchSpy = mockFetch(async (input, init) => {
      const url = String(input);
      if (url.endsWith('/outcomes') && init?.method === 'POST') return jsonResponse(OUTCOME, 201);
      return jsonResponse({ outcomes: [OUTCOME] });
    });
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    await expect(client.recordActionOutcome(ACTION.id, OUTCOME)).resolves.toMatchObject({
      status: 'succeeded',
    });
    await expect(client.listActionOutcomes(ACTION.id)).resolves.toMatchObject({
      outcomes: [OUTCOME],
    });

    expect(fetchSpy.mock.calls.map(([url]) => String(url))).toEqual([
      `http://server.test/v1/financial/actions/${ACTION.id}/outcomes`,
      `http://server.test/v1/financial/actions/${ACTION.id}/outcomes`,
    ]);
  });
});
