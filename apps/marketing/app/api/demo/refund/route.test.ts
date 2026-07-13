import assert from 'node:assert/strict';
import test from 'node:test';

import { GET, POST } from './route';

const PROXY_SECRET = 'refund-demo-proxy-secret-32-bytes-minimum';
const mutableEnv = process.env as Record<string, string | undefined>;
process.env['REFUND_DEMO_PROXY_SECRET'] = PROXY_SECRET;
mutableEnv['NODE_ENV'] = 'production';

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
    assert.equal(new Headers(init?.headers).get('authorization'), `Bearer ${PROXY_SECRET}`);
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
    assert.equal(body.logs, undefined);
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

test('treats an invalid upstream success payload as a service failure', async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () => Response.json({ internal: 'unexpected response' })) as typeof fetch;

  try {
    const response = await POST(
      requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-invalid-upstream'),
    );
    const body = await response.json();
    assert.equal(response.status, 502);
    assert.equal(body.error, 'The refund workflow returned an invalid response. No refund was executed.');
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('limits repeated expensive requests for one visitor', async () => {
  const originalFetch = globalThis.fetch;
  let upstreamCalls = 0;
  globalThis.fetch = (async () => {
    upstreamCalls += 1;
    return Response.json(upstreamPayload);
  }) as typeof fetch;

  try {
    const responses = [];
    for (let attempt = 0; attempt < 5; attempt += 1) {
      responses.push(
        await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-limited')),
      );
    }
    assert.deepEqual(
      responses.map((response) => response.status),
      [200, 200, 200, 200, 429],
    );
    assert.equal(upstreamCalls, 4);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('allows repeated localhost runs during development', async () => {
  const originalFetch = globalThis.fetch;
  const originalNodeEnv = process.env['NODE_ENV'];
  let upstreamCalls = 0;
  mutableEnv['NODE_ENV'] = 'development';
  globalThis.fetch = (async () => {
    upstreamCalls += 1;
    return Response.json(upstreamPayload);
  }) as typeof fetch;

  try {
    const responses = [];
    for (let attempt = 0; attempt < 6; attempt += 1) {
      responses.push(
        await POST(requestFor({ prompt: 'Refund order ord_demo_1001 for $25.' }, 'route-local-dev')),
      );
    }
    assert.deepEqual(
      responses.map((response) => response.status),
      [200, 200, 200, 200, 200, 200],
    );
    assert.equal(upstreamCalls, 6);
  } finally {
    globalThis.fetch = originalFetch;
    if (originalNodeEnv === undefined) delete mutableEnv['NODE_ENV'];
    else mutableEnv['NODE_ENV'] = originalNodeEnv;
  }
});

test('uses the platform-owned client address instead of a spoofable forwarded value', async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = (async () => Response.json(upstreamPayload)) as typeof fetch;

  try {
    const responses = [];
    for (let attempt = 0; attempt < 5; attempt += 1) {
      responses.push(
        await POST(
          new Request('http://localhost/api/demo/refund', {
            method: 'POST',
            headers: {
              'content-type': 'application/json',
              'x-vercel-forwarded-for': 'trusted-platform-client',
              'x-forwarded-for': `spoofed-${attempt}`,
            },
            body: JSON.stringify({ prompt: 'Refund order ord_demo_1001 for $25.' }),
          }),
        ),
      );
    }

    assert.deepEqual(
      responses.map((response) => response.status),
      [200, 200, 200, 200, 429],
    );
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('proxies a demo action status without exposing upstream fields', async () => {
  const originalFetch = globalThis.fetch;
  const actionId = '019f5d63-f8ca-77c3-ae7f-07b122daa7b3';
  globalThis.fetch = (async (input, init) => {
    assert.equal(String(input), `http://127.0.0.1:9310/status/${actionId}`);
    assert.equal(new Headers(init?.headers).get('authorization'), `Bearer ${PROXY_SECRET}`);
    return Response.json({
      actionId,
      status: 'executed',
      orderId: 'ord_demo_1001',
      amountMinor: 7_500,
      currency: 'USD',
      receiptId: actionId,
      providerReference: 're_test_status_123',
      updatedAt: '2026-07-13T21:31:00.000Z',
      internalProof: { paymentIntentId: 'pi_private' },
    });
  }) as typeof fetch;

  try {
    const response = await GET(
      new Request(`http://localhost/api/demo/refund?actionId=${encodeURIComponent(actionId)}`),
    );
    const body = await response.json();
    assert.equal(response.status, 200);
    assert.equal(body.status, 'executed');
    assert.equal(body.providerReference, 're_test_status_123');
    assert.equal(body.internalProof, undefined);
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
