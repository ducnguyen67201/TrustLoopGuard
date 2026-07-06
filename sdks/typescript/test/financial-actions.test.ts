import { describe, expect, it } from 'vitest';

import {
  Client,
  type CreateFinancialActionRequest,
  type CreateFinancialMandateRequest,
} from '../src';
import { jsonResponse, mockFetch } from './test-utils';

const REQUEST: CreateFinancialActionRequest = {
  idempotency_key: 'idem-refund-75',
  execute: false,
  action: {
    kind: 'refund',
    principal_id: 'refund-bot',
    amount: { amount_minor: 7500, currency: 'USD' },
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
  action: { ...REQUEST.action, id: '018f3333-3333-7333-8333-333333333333' },
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

describe('Client financial action methods', () => {
  it('verifyAction posts typed financial actions', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(ACTION, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const action = await client.verifyAction(REQUEST);

    expect(action.id).toBe(ACTION.id);
    const [url, init] = fetchSpy.mock.calls[0]!;
    expect(url).toBe('http://server.test/v1/financial/actions');
    expect((init as RequestInit).method).toBe('POST');
    expect(JSON.parse(String((init as RequestInit).body))).toEqual(REQUEST);
  });

  it('guardPayment aliases verifyAction for payment/refund ergonomics', async () => {
    const fetchSpy = mockFetch(async () => jsonResponse(ACTION, 201));
    const client = new Client({ baseUrl: 'http://server.test', fetchImpl: fetchSpy });

    const action = await client.guardPayment(REQUEST);

    expect(action.status).toBe('proposed');
    expect(fetchSpy).toHaveBeenCalledOnce();
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
