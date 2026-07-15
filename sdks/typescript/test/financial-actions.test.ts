import { describe, expect, it } from 'vitest';

import { Client, type CreateFinancialActionRequest } from '../src';
import { jsonResponse, mockFetch } from './test-utils';

const request: CreateFinancialActionRequest = {
  idempotency_key: 'idem-1',
  execute: false,
  action: {
    kind: 'payment',
    operation: 'pay',
    principal_id: 'agent-1',
    amount: { amount_minor: 100n, currency: 'USD' },
    rail: 'internal',
    metadata: {},
  },
  evidence: [],
};

const action = {
  id: 'action-1',
  workspace_id: 'workspace-1',
  environment_id: 'production',
  authorization_intent_id: 'intent-1',
  authorization_receipt_id: 'receipt-1',
  authorization_effect: 'permit',
  authorization_status: 'authorized',
  execution_status: 'not_started',
  state: 'authorized',
  action: {
    ...request.action,
    id: 'action-1',
    amount: { amount_minor: 100, currency: 'USD' },
  },
  evidence: [],
  created_at: '2026-07-14T00:00:00Z',
  updated_at: '2026-07-14T00:00:00Z',
};

describe('financial actions', () => {
  it('decodes the unified authorization and execution projection', async () => {
    const fetch = mockFetch(async () => jsonResponse(action, 201));
    const result = await new Client({ baseUrl: 'https://api.test', fetchImpl: fetch }).verifyAction(
      request,
    );

    expect(result.authorization_effect).toBe('permit');
    expect(result.authorization_status).toBe('authorized');
    expect(result.execution_status).toBe('not_started');
    expect(result.state).toBe('authorized');
    expect(fetch).toHaveBeenCalledTimes(1);
  });

  it('builds financial requests with the common authorization claim', () => {
    const client = new Client({ baseUrl: 'https://api.test', fetchImpl: mockFetch() });
    const operation = client.financialOperation<number>({
      operation: 'pay',
      kind: 'payment',
      principalId: 'agent-1',
      rail: 'internal',
      amount: (amount) => ({ amount_minor: BigInt(amount), currency: 'USD' }),
      idempotencyKey: () => 'idem-2',
      authorization: () => ({ grant_id: 'grant-1', attempt_id: 'attempt-1' }),
    });

    const built = operation.buildRequest(100);
    expect(built.authorization).toEqual({ grant_id: 'grant-1', attempt_id: 'attempt-1' });
    expect(built.action).not.toHaveProperty('mandate');
  });
});
