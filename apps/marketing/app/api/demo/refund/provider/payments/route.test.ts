import assert from 'node:assert/strict';
import test from 'node:test';

import { createProviderPaymentsHandler } from './route';

const validRequest = {
  action_id: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
  kind: 'refund',
  amount_minor: 2_500,
  currency: 'USD',
  metadata: {
    payment_intent_id: 'pi_test_123',
    order_id: 'ord_demo_1001',
    reason: 'damaged_item',
  },
};

test('rejects an unauthenticated provider callback before execution', async () => {
  let called = false;
  const POST = createProviderPaymentsHandler({
    handlePayment: async () => {
      called = true;
      return { statusCode: 200, body: { status: 'succeeded' } as never };
    },
  });

  const response = await POST(providerRequest(validRequest));
  assert.equal(response.status, 401);
  assert.equal(called, false);
});

test('rejects malformed provider callbacks without exposing validation details', async () => {
  const POST = createProviderPaymentsHandler({
    authorize: () => true,
    handlePayment: async () => ({ statusCode: 200, body: { status: 'succeeded' } as never }),
  });

  const response = await POST(providerRequest({ ...validRequest, amount_minor: -1 }, 'Bearer valid'));
  assert.equal(response.status, 400);
  assert.deepEqual(await response.json(), { error: 'invalid provider request' });
});

test('executes a valid authenticated callback through the refund provider adapter', async () => {
  let receivedAuthorization: string | undefined;
  let receivedBody: unknown;
  const POST = createProviderPaymentsHandler({
    authorize: (authorization) => authorization === 'Bearer provider-secret',
    handlePayment: async (authorization, body) => {
      receivedAuthorization = authorization;
      receivedBody = body;
      return {
        statusCode: 200,
        body: {
          status: 'succeeded',
          provider_status: 'succeeded',
          provider_reference: 're_test_123',
          reversal_capability: 'manual_recovery',
          recovery_status: 'manual_required',
          mode: 'stripe-test',
        },
      };
    },
  });

  const response = await POST(providerRequest(validRequest, 'Bearer provider-secret'));
  assert.equal(response.status, 200);
  assert.equal(receivedAuthorization, 'Bearer provider-secret');
  assert.deepEqual(receivedBody, validRequest);
  assert.equal((await response.json()).provider_reference, 're_test_123');
});

test('fails closed without leaking provider or Stripe errors', async () => {
  const POST = createProviderPaymentsHandler({
    authorize: () => true,
    handlePayment: async () => {
      throw new Error('Stripe secret and upstream response');
    },
  });

  const response = await POST(providerRequest(validRequest, 'Bearer valid'));
  assert.equal(response.status, 500);
  assert.deepEqual(await response.json(), { error: 'provider request failed' });
});

function providerRequest(body: unknown, authorization?: string): Request {
  const headers = new Headers({ 'content-type': 'application/json' });
  if (authorization !== undefined) headers.set('authorization', authorization);
  return new Request('http://localhost/api/demo/refund/provider/payments', {
    method: 'POST',
    headers,
    body: JSON.stringify(body),
  });
}
