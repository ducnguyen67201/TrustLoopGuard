import assert from 'node:assert/strict';
import test from 'node:test';

import { POST } from './route';

const upstreamPayload = {
  result: {
    prompt: 'Refund order ord_demo_1001 for $25.',
    traces: [
      { tool: 'search_order', summary: 'found ord_demo_1001' },
      { tool: 'prepare_refund', summary: 'proposed: authorized' },
      { tool: 'execute_refund', summary: 'executed through TrustLoopGuard' },
    ],
    finalMessage: 'Refund executed.',
    actionId: 'financial_action_123',
    receiptId: 'financial_action_123',
  },
  state: {
    orders: [
      {
        id: 'ord_demo_1001',
        customerName: 'Jamie Demo',
        customerEmail: 'jamie@example.com',
        paymentMethodLast4: '4242',
        amountPaidMinor: 10_000,
        refundableBalanceMinor: 7_500,
        currency: 'USD',
        captured: true,
        refundWindowOpen: true,
        refundCount: 1,
        paymentIntentId: 'pi_internal',
      },
    ],
    refunds: [
      {
        orderId: 'ord_demo_1001',
        financialActionId: 'financial_action_123',
        amountMinor: 2_500,
        providerReference: 're_test_123',
        status: 'succeeded',
        reason: 'damaged_item',
        createdAt: '2026-07-13T12:00:00.000Z',
      },
    ],
  },
  logs: [{ step: 'chat', message: 'refund agent finished' }],
  runtime: {
    agent: 'openai',
    guard: 'trustloopguard-rust-api',
    provider: 'stripe-test',
  },
};

test('proxies a valid prompt and strips private upstream fields', async () => {
  const originalFetch = globalThis.fetch;
  let upstreamBody = '';
  globalThis.fetch = (async (input, init) => {
    assert.equal(String(input), 'http://127.0.0.1:9310/chat');
    upstreamBody = String(init?.body);
    return Response.json(upstreamPayload);
  }) as typeof fetch;

  try {
    const response = await POST(requestFor({ prompt: ' Refund order ord_demo_1001 for $25. ' }, 'route-ok'));
    const body = await response.json();

    assert.equal(response.status, 200);
    assert.deepEqual(JSON.parse(upstreamBody), {
      prompt: 'Refund order ord_demo_1001 for $25.',
    });
    assert.equal(body.state.orders[0].customerEmail, undefined);
    assert.equal(body.state.orders[0].paymentIntentId, undefined);
    assert.equal(body.state.refunds[0].providerReference, 're_test_123');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('rejects invalid input without calling the live services', async () => {
  const originalFetch = globalThis.fetch;
  let called = false;
  globalThis.fetch = (async () => {
    called = true;
    return Response.json({});
  }) as typeof fetch;

  try {
    const response = await POST(requestFor({ prompt: '' }, 'route-invalid'));
    assert.equal(response.status, 400);
    assert.equal(called, false);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('does not expose raw upstream errors to the public browser', async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () =>
    Response.json(
      { error: 'stripe internal response with provider configuration details' },
      { status: 500 },
    )) as typeof fetch;

  try {
    const response = await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-error'));
    const body = await response.json();
    assert.equal(response.status, 500);
    assert.equal(body.error, 'The refund workflow failed safely. No refund was executed.');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

function requestFor(body: object, ip: string): Request {
  return new Request('http://localhost/api/demo/refund', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-forwarded-for': ip,
    },
    body: JSON.stringify(body),
  });
}
