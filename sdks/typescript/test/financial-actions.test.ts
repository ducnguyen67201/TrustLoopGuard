import { describe, expect, it } from 'vitest';

import { Client, type CreateFinancialActionRequest } from '../src';
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
});
