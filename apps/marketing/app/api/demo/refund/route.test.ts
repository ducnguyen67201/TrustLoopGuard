import assert from 'node:assert/strict';
import test from 'node:test';

import { createRefundDemoHandlers } from './route';

const mutableEnv = process.env as Record<string, string | undefined>;
mutableEnv['NODE_ENV'] = 'production';

const workflowPayload = {
  result: {
    prompt: 'Refund order ord_demo_1001 for $25.',
    traces: [
      { tool: 'search_order', summary: 'found ord_demo_1001' },
      { tool: 'prepare_refund', summary: 'proposed: authorized' },
      { tool: 'execute_refund', summary: 'executed through TrustLoopGuard' },
    ],
    finalMessage: 'Refund executed.',
    actionId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
    receiptId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
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
        financialActionId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
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

test('runs a valid prompt in-process and strips private workflow fields', async () => {
  let receivedPrompt = '';
  const { POST } = createRefundDemoHandlers({
    runWorkflow: async (prompt) => {
      receivedPrompt = prompt;
      return workflowPayload;
    },
    readStatus: async () => statusPayload(),
  });

  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () => {
    throw new Error('the consolidated Marketing route must not fetch a refund service');
  }) as typeof fetch;
  try {
    const response = await POST(requestFor({ prompt: ' Refund order ord_demo_1001 for $25. ' }, 'route-ok'));
    const body = await response.json();

    assert.equal(response.status, 200);
    assert.equal(receivedPrompt, 'Refund order ord_demo_1001 for $25.');
    assert.equal(body.state.orders[0].customerEmail, undefined);
    assert.equal(body.state.orders[0].paymentIntentId, undefined);
    assert.equal(body.state.refunds[0].providerReference, 're_test_123');
    assert.equal(body.logs, undefined);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('rejects invalid input without running the workflow', async () => {
  let called = false;
  const { POST } = createRefundDemoHandlers({
    runWorkflow: async () => {
      called = true;
      return workflowPayload;
    },
    readStatus: async () => statusPayload(),
  });

  const response = await POST(requestFor({ prompt: '' }, 'route-invalid'));
  assert.equal(response.status, 400);
  assert.equal(called, false);
});

test('does not expose internal workflow errors to the browser', async () => {
  const { POST } = createRefundDemoHandlers({
    runWorkflow: async () => {
      throw new Error('stripe internal response with provider configuration details');
    },
    readStatus: async () => statusPayload(),
  });

  const response = await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-error'));
  const body = await response.json();
  assert.equal(response.status, 503);
  assert.equal(body.error, 'The live demo is temporarily unavailable. No refund was executed.');
});

test('maps the central launch budget to a public 429', async () => {
  const { POST } = createRefundDemoHandlers({
    runWorkflow: async () => {
      const error = new Error('budget reached');
      error.name = 'RefundDemoBudgetExceededError';
      throw error;
    },
    readStatus: async () => statusPayload(),
  });

  const response = await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'budget'));
  assert.equal(response.status, 429);
  assert.deepEqual(await response.json(), { error: 'Demo budget reached. Try again later.' });
});

test('allows ten requests per visitor in a rolling 24-hour window', async () => {
  const originalDateNow = Date.now;
  let now = Date.parse('2026-07-13T12:00:00.000Z');
  let workflowCalls = 0;
  Date.now = () => now;
  const { POST } = createRefundDemoHandlers({
    runWorkflow: async () => {
      workflowCalls += 1;
      return workflowPayload;
    },
    readStatus: async () => statusPayload(),
  });

  try {
    const responses = [];
    for (let attempt = 0; attempt < 11; attempt += 1) {
      responses.push(await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-limited')));
    }
    assert.deepEqual(
      responses.map((response) => response.status),
      [200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 429],
    );
    assert.equal(workflowCalls, 10);

    now += 24 * 60 * 60 * 1_000;
    assert.equal(
      (await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-limited'))).status,
      200,
    );
    assert.equal(workflowCalls, 11);
  } finally {
    Date.now = originalDateNow;
  }
});

test('reads status in-process and redacts internal fields', async () => {
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  let receivedActionId = '';
  const { GET } = createRefundDemoHandlers({
    runWorkflow: async () => workflowPayload,
    readStatus: async (id) => {
      receivedActionId = id;
      return { ...statusPayload(), internalProof: { paymentIntentId: 'pi_private' } };
    },
  });

  const response = await GET(new Request(`http://localhost/api/demo/refund?actionId=${actionId}`));
  const body = await response.json();
  assert.equal(response.status, 200);
  assert.equal(receivedActionId, actionId);
  assert.equal(body.executionStatus, 'succeeded');
  assert.equal(body.internalProof, undefined);
});

function statusPayload() {
  return {
    actionId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
    authorizationEffect: 'permit',
    executionStatus: 'succeeded',
    orderId: 'ord_demo_1001',
    amountMinor: 7_500,
    currency: 'USD',
    receiptId: '019f5d63-f8ca-77c3-ae7f-07b122daa7b3',
    providerReference: 're_test_status_123',
    updatedAt: '2026-07-13T21:31:00.000Z',
  };
}

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
