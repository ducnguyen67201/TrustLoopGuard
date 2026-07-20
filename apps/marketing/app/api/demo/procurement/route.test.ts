import assert from 'node:assert/strict';
import test from 'node:test';

import type {
  HostedProcurementDemoResponse,
  HostedProcurementPolicyInventoryResponse,
} from '@trustloopguard/demo/procurement-agent/hosted';

import { createProcurementDemoHandlers } from './route';

const mutableEnv = process.env as Record<string, string | undefined>;
mutableEnv['NODE_ENV'] = 'production';

test('returns the Rust procurement policy inventory without caching it', async () => {
  const { GET } = createProcurementDemoHandlers({
    runWorkflow: async () => workflowPayload(),
    readPolicies: async () => inventoryPayload(),
  });

  const response = await GET();
  const body = await response.json();

  assert.equal(response.status, 200);
  assert.equal(response.headers.get('cache-control'), 'no-store');
  assert.equal(body.source, 'rust');
  assert.deepEqual(body.workspace, {
    id: 'ws-route-test',
    source: 'configured',
  });
  assert.equal(body.policies[0].id, 'procurement-approved-suppliers');
  assert.equal(body.policies[0].privateMatcher, undefined);
});

test('returns a safe policy pack preview when the Rust inventory is unavailable', async () => {
  const originalWarn = console.warn;
  console.warn = () => {};
  try {
    const { GET } = createProcurementDemoHandlers({
      runWorkflow: async () => workflowPayload(),
      readPolicies: async () => {
        throw new Error('private backend details');
      },
    });

    const response = await GET();
    const body = await response.json();

    assert.equal(response.status, 200);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    assert.equal(body.source, 'demo_template');
    assert.equal(body.policies.length, 3);
    assert.equal(body.policies.every((policy: { enabled: boolean }) => !policy.enabled), true);
  } finally {
    console.warn = originalWarn;
  }
});

test('runs a valid request in-process, normalizes policies, and returns no-store data', async () => {
  let receivedPrompt = '';
  let receivedPolicyIds: readonly string[] = [];
  const { POST } = createProcurementDemoHandlers({
    runWorkflow: async (prompt, activePolicyIds) => {
      receivedPrompt = prompt;
      receivedPolicyIds = activePolicyIds;
      return workflowPayload();
    },
  });

  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => {
    throw new Error('the consolidated Marketing route must not call another demo service');
  };
  try {
    const response = await POST(
      requestFor(
        {
          prompt: '  Order the approved office chairs.  ',
          activePolicyIds: [
            'procurement-restricted-categories',
            'procurement-approved-suppliers',
            'procurement-approved-suppliers',
          ],
        },
        'valid-route',
      ),
    );
    const body = await response.json();

    assert.equal(response.status, 200);
    assert.equal(response.headers.get('cache-control'), 'no-store');
    assert.equal(receivedPrompt, 'Order the approved office chairs.');
    assert.deepEqual(receivedPolicyIds, [
      'procurement-approved-suppliers',
      'procurement-restricted-categories',
    ]);
    assert.equal(body.result.decision.effect, 'permit');
    assert.equal(body.logs, undefined);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test('defaults to all three policies', async () => {
  let receivedPolicyIds: readonly string[] = [];
  const { POST } = createProcurementDemoHandlers({
    runWorkflow: async (_prompt, activePolicyIds) => {
      receivedPolicyIds = activePolicyIds;
      return workflowPayload();
    },
  });

  const response = await POST(requestFor({ prompt: 'Order chairs.' }, 'default-policies'));
  assert.equal(response.status, 200);
  assert.deepEqual(receivedPolicyIds, [
    'procurement-approved-suppliers',
    'procurement-high-value-review',
    'procurement-restricted-categories',
  ]);
});

test('rejects malformed, invalid, and injectable requests before running the workflow', async () => {
  let calls = 0;
  const { POST } = createProcurementDemoHandlers({
    runWorkflow: async () => {
      calls += 1;
      return workflowPayload();
    },
  });

  const invalidRequests = [
    requestFor({ prompt: '' }, 'invalid-empty'),
    requestFor({ prompt: 'x'.repeat(501) }, 'invalid-long'),
    requestFor({ prompt: 'Order chairs.', activePolicyIds: ['unknown-policy'] }, 'invalid-policy'),
    requestFor({ prompt: 'Order chairs.', agentId: 'attacker' }, 'invalid-agent'),
    new Request('http://localhost/api/demo/procurement', {
      method: 'POST',
      headers: { 'content-type': 'application/json', 'x-forwarded-for': 'invalid-json' },
      body: '{',
    }),
  ];

  for (const request of invalidRequests) {
    assert.equal((await POST(request)).status, 400);
  }
  assert.equal(calls, 0);
});

test('maps central budget failures and generic live failures safely', async () => {
  const budgetHandlers = createProcurementDemoHandlers({
    runWorkflow: async () => {
      const error = new Error('budget details');
      error.name = 'ProcurementDemoBudgetExceededError';
      throw error;
    },
  });
  const budgetResponse = await budgetHandlers.POST(
    requestFor({ prompt: 'Order chairs.' }, 'budget-error'),
  );
  assert.equal(budgetResponse.status, 429);

  const failureHandlers = createProcurementDemoHandlers({
    runWorkflow: async () => {
      throw new Error('private provider configuration details');
    },
  });
  const failureResponse = await failureHandlers.POST(
    requestFor({ prompt: 'Order chairs.' }, 'live-error'),
  );
  assert.equal(failureResponse.status, 503);
  assert.deepEqual(await failureResponse.json(), {
    error: 'The live procurement demo is temporarily unavailable. No purchase order was submitted.',
  });
});

test('returns 502 when the hosted workflow violates the public response contract', async () => {
  const { POST } = createProcurementDemoHandlers({
    runWorkflow: async () => ({ result: { finalMessage: 'missing required fields' } }),
  });

  const response = await POST(requestFor({ prompt: 'Order chairs.' }, 'bad-contract'));
  assert.equal(response.status, 502);
  assert.deepEqual(await response.json(), {
    error:
      'The procurement workflow returned an invalid response. No purchase order was submitted.',
  });
});

test('allows ten requests per visitor and resets after 24 hours', async () => {
  const originalDateNow = Date.now;
  let now = Date.parse('2026-07-19T12:00:00.000Z');
  let workflowCalls = 0;
  Date.now = () => now;
  const { POST } = createProcurementDemoHandlers({
    runWorkflow: async () => {
      workflowCalls += 1;
      return workflowPayload();
    },
  });

  try {
    const responses = [];
    for (let attempt = 0; attempt < 11; attempt += 1) {
      responses.push(await POST(requestFor({ prompt: 'Order chairs.' }, 'rate-limited-visitor')));
    }
    assert.deepEqual(
      responses.map((response) => response.status),
      [200, 200, 200, 200, 200, 200, 200, 200, 200, 200, 429],
    );
    assert.equal(workflowCalls, 10);

    now += 24 * 60 * 60 * 1_000;
    assert.equal(
      (await POST(requestFor({ prompt: 'Order chairs.' }, 'rate-limited-visitor'))).status,
      200,
    );
    assert.equal(workflowCalls, 11);
  } finally {
    Date.now = originalDateNow;
  }
});

function workflowPayload(): HostedProcurementDemoResponse {
  return {
    result: {
      finalMessage: 'Purchase order submitted.',
      traces: [
        { tool: 'search_catalog', summary: 'Found one demo quote.' },
        {
          tool: 'submit_purchase_order',
          summary: 'Purchase order submitted after TrustLoopGuard returned permit.',
        },
      ],
      decision: {
        traceId: 'trace-route-test',
        effect: 'permit',
        reason: 'No policy blocked the action.',
        latencyMs: 5,
        findings: [],
      },
    },
    state: {
      purchaseOrders: [
        {
          id: 'po-route-test',
          quoteId: 'quote-approved-chairs',
          supplierName: 'Northstar Office',
          itemName: 'Ergonomic office chairs',
          quantity: 20,
          totalMinor: 240_000,
          currency: 'USD',
          status: 'submitted',
        },
      ],
    },
    activePolicies: [
      {
        id: 'procurement-approved-suppliers',
        title: 'Approved suppliers only',
        description: 'Approved suppliers.',
        effect: 'deny',
        enabled: true,
      },
      {
        id: 'procurement-high-value-review',
        title: 'Review high-value orders',
        description: 'High-value review.',
        effect: 'require_approval',
        enabled: true,
      },
      {
        id: 'procurement-restricted-categories',
        title: 'Block restricted categories',
        description: 'Restricted categories.',
        effect: 'deny',
        enabled: true,
      },
    ],
    logs: [{ step: 'chat_received' }],
    runtime: {
      agent: 'openai-agents-js',
      guard: 'trustloopguard-rust-api',
      provider: 'simulated-procurement-api',
    },
  };
}

function inventoryPayload(): Extract<
  HostedProcurementPolicyInventoryResponse,
  { source: 'rust' }
> {
  return {
    policies: [
      {
        id: 'procurement-approved-suppliers',
        description: 'Blocks purchase orders from unapproved suppliers.',
        severity: 'critical',
        action: 'deny',
        enabled: true,
      },
    ],
    source: 'rust',
    runtime: {
      agent: 'openai-agents-js',
      guard: 'trustloopguard-rust-api',
      provider: 'simulated-procurement-api',
    },
    workspace: {
      id: 'ws-route-test',
      source: 'configured',
    },
  };
}

function requestFor(body: object, ip: string): Request {
  return new Request('http://localhost/api/demo/procurement', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-forwarded-for': ip,
    },
    body: JSON.stringify(body),
  });
}
