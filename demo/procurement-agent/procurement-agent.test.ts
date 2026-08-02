import assert from 'node:assert/strict';
import test from 'node:test';

import type { AuthorizationDecision, PolicyListResponse } from '@featherlane-ai/sdk';

import {
  runProcurementAgent,
  submitProcurementPurchaseOrder,
  type ProcurementAuthorizationClient,
  type ProcurementRunContext,
} from './agent';
import {
  findProcurementQuote,
  normalizeProcurementPolicyIds,
  procurementAgentId,
  procurementPolicyDocuments,
  PROCUREMENT_POLICY_IDS,
  type ProcurementPolicyId,
} from './fixtures';
import {
  ProcurementDemoRequestBudget,
  readHostedProcurementDemoPolicies,
  readHostedProcurementDemoPolicyPreview,
  runHostedProcurementDemo,
} from './hosted';

test('maps every policy subset to one stable three-bit principal', () => {
  for (let profile = 0; profile < 8; profile += 1) {
    const selected = PROCUREMENT_POLICY_IDS.filter((_, index) => {
      const bit = 1 << (2 - index);
      return (profile & bit) === bit;
    });
    assert.equal(
      procurementAgentId([...selected].reverse()),
      `procurement-demo-${profile.toString(2).padStart(3, '0')}`,
    );
    assert.deepEqual(normalizeProcurementPolicyIds([...selected, ...selected]), selected);
  }
});

test('generates one fully scoped Rust tool policy for each control', () => {
  const documents = procurementPolicyDocuments();
  assert.equal(documents.length, 3);
  for (const document of documents) {
    assert.equal(document.family, 'tool');
    assert.equal(document.when.agents.length, 4);
    assert.deepEqual(document.when.operations, ['submit_purchase_order']);
    assert.deepEqual(document.when.side_effects, ['api_mutation']);
    assert.deepEqual(document.when.tools, [
      { server_id: 'openai-agents', tool_name: 'submit_purchase_order' },
    ]);
  }

  const approval = documents.find((document) => document.id === 'procurement-high-value-review');
  assert.equal(approval?.action, 'require_approval');
  assert.deepEqual(approval?.approver_roles, ['owner', 'admin']);
  assert.equal(approval?.max_grant_ttl_seconds, 900);
});

test('returns copies of canonical catalog facts', () => {
  const quote = findProcurementQuote('quote-high-value-laptops');
  assert.equal(quote.totalMinor, 4_200_000);
  quote.totalMinor = 1;
  assert.equal(findProcurementQuote('quote-high-value-laptops').totalMinor, 4_200_000);
});

test('executes one purchase order only after a permit decision with canonical parameters', async () => {
  let capturedParameters = '';
  let callbackCalls = 0;
  const client: ProcurementAuthorizationClient = {
    async withAuthorizedAction(options, execute) {
      capturedParameters = JSON.stringify(options.parameters);
      callbackCalls += 1;
      const value = await execute(Object.freeze({ ...(options.parameters ?? {}) }));
      return { decision: authorizationDecision('permit'), executed: true, value };
    },
  };
  const context = runContext(client);

  const result = await submitProcurementPurchaseOrder('quote-approved-chairs', context);

  assert.equal(result.status, 'submitted');
  assert.equal(callbackCalls, 1);
  assert.equal(context.purchaseOrders.length, 1);
  assert.deepEqual(JSON.parse(capturedParameters), {
    quote_id: 'quote-approved-chairs',
    supplier_id: 'supplier-northstar-office',
    supplier_status: 'approved',
    category: 'office_furniture',
    item_name: 'Ergonomic office chairs',
    quantity: 20,
    unit_price_minor: 12_000,
    total_minor: 240_000,
    currency: 'USD',
    review_tier: 'standard',
  });

  const repeated = await submitProcurementPurchaseOrder('quote-approved-chairs', context);
  assert.equal(repeated.status, 'stopped');
  assert.equal(callbackCalls, 1);
});

test('deny and approval decisions never execute the provider callback', async () => {
  for (const effect of ['deny', 'require_approval'] as const) {
    let callbackCalls = 0;
    const client: ProcurementAuthorizationClient = {
      async withAuthorizedAction() {
        callbackCalls += 0;
        return { decision: authorizationDecision(effect), executed: false };
      },
    };
    const context = runContext(client);
    const result = await submitProcurementPurchaseOrder('quote-high-value-laptops', context);

    assert.equal(result.status, 'stopped');
    assert.equal(result.effect, effect);
    assert.equal(callbackCalls, 0);
    assert.equal(context.purchaseOrders.length, 0);
    assert.equal(context.decision?.effect, effect);
  }
});

test('does not retry a permitted provider callback that fails', async () => {
  let callbackCalls = 0;
  const client: ProcurementAuthorizationClient = {
    async withAuthorizedAction(options, execute) {
      try {
        callbackCalls += 1;
        await execute(Object.freeze({ ...(options.parameters ?? {}), total_minor: -1 }));
        assert.fail('invalid approved parameters should fail');
      } catch (error) {
        throw error;
      }
    },
  };

  await assert.rejects(submitProcurementPurchaseOrder('quote-approved-chairs', runContext(client)));
  assert.equal(callbackCalls, 1);
});

test('requires live OpenAI credentials and never falls back to a scripted agent', async () => {
  const context = runContext(nonExecutingClient());
  await assert.rejects(
    runProcurementAgent('Order chairs', context, { apiKey: '' }),
    /OPENAI_API_KEY/,
  );
});

test('enforces the central process-local request budget', () => {
  const budget = new ProcurementDemoRequestBudget({ maxRequests: 2, windowMs: 1_000 });
  assert.equal(budget.tryAcquire(100), true);
  assert.equal(budget.tryAcquire(101), true);
  assert.equal(budget.tryAcquire(102), false);
  assert.equal(budget.tryAcquire(1_100), true);
});

test('projects the enabled procurement policy inventory from the Rust registry', async () => {
  const inventory = await readHostedProcurementDemoPolicies({
    workspaceId: '  ws-procurement-test  ',
    createClient: () => ({
      async listPolicies(): Promise<PolicyListResponse> {
        return {
          policies: [
            {
              id: 'procurement-restricted-categories',
              family: 'tool',
              description: 'Rust-owned restricted category policy.',
              severity: 'critical',
              action: 'deny',
              enabled: true,
            },
            {
              id: 'procurement-high-value-review',
              family: 'tool',
              severity: 'high',
              action: 'require_approval',
              enabled: false,
            },
            {
              id: 'unrelated-tool-policy',
              family: 'tool',
              severity: 'low',
              enabled: true,
            },
          ],
        };
      },
    }),
  });

  assert.equal(inventory.source, 'rust');
  assert.deepEqual(inventory.workspace, {
    id: 'ws-procurement-test',
    source: 'configured',
  });
  assert.deepEqual(inventory.policies, [
    {
      id: 'procurement-restricted-categories',
      description: 'Rust-owned restricted category policy.',
      severity: 'critical',
      action: 'deny',
      enabled: true,
    },
  ]);
});

test('builds a disabled preview of the policies installed by demo setup', () => {
  const preview = readHostedProcurementDemoPolicyPreview('');
  assert.equal(preview.source, 'demo_template');
  assert.deepEqual(preview.workspace, { source: 'server_default' });
  assert.deepEqual(
    preview.policies.map((policy) => policy.id),
    [...PROCUREMENT_POLICY_IDS],
  );
  assert.equal(preview.policies.every((policy) => !policy.enabled), true);
});

test('keeps concurrent hosted runs isolated', async () => {
  const requestIds = ['request-a', 'request-b'];
  const responses = await Promise.all([
    runHostedProcurementDemo(
      'First request',
      PROCUREMENT_POLICY_IDS,
      hostedDependencies(requestIds),
    ),
    runHostedProcurementDemo(
      'Second request',
      ['procurement-approved-suppliers'],
      hostedDependencies(requestIds),
    ),
  ]);

  assert.notEqual(responses[0]?.result.finalMessage, responses[1]?.result.finalMessage);
  assert.notEqual(
    responses[0]?.state.purchaseOrders[0]?.id,
    responses[1]?.state.purchaseOrders[0]?.id,
  );
  assert.equal(responses[0]?.activePolicies.filter((policy) => policy.enabled).length, 3);
  assert.equal(responses[1]?.activePolicies.filter((policy) => policy.enabled).length, 1);
});

function hostedDependencies(requestIds: string[]) {
  return {
    budget: { tryAcquire: () => true },
    createClient: nonExecutingClient,
    createRequestId: () => requestIds.shift() ?? 'unexpected-request',
    runAgent: async (_prompt: string, context: ProcurementRunContext) => ({
      finalMessage: `finished ${context.requestId}`,
      traces: [],
      purchaseOrders: [
        {
          id: `po-${context.requestId}`,
          quoteId: 'quote-approved-chairs' as const,
          supplierName: 'Northstar Office',
          itemName: 'Ergonomic office chairs',
          quantity: 20,
          totalMinor: 240_000,
          currency: 'USD' as const,
          status: 'submitted' as const,
        },
      ],
    }),
  };
}

function runContext(client: ProcurementAuthorizationClient): ProcurementRunContext {
  return {
    client,
    agentId: procurementAgentId(PROCUREMENT_POLICY_IDS),
    requestId: 'request-test',
    logger: { log() {} },
    traces: [],
    purchaseOrders: [],
    authorizationAttempted: false,
    nextInvocationSequence: 0,
  };
}

function nonExecutingClient(): ProcurementAuthorizationClient {
  return {
    async withAuthorizedAction() {
      return { decision: authorizationDecision('deny'), executed: false };
    },
  };
}

function authorizationDecision(
  effect: AuthorizationDecision['effect'],
  policyId: ProcurementPolicyId = 'procurement-approved-suppliers',
): AuthorizationDecision {
  return {
    trace_id: `trace-${effect}`,
    domain: 'tool',
    effect,
    reason: `${effect} from test policy`,
    findings: [
      {
        id: `finding-${effect}`,
        source: 'policy',
        effect,
        reason: `${effect} from test policy`,
        severity: 'high',
        policy_id: policyId,
        evidence: null,
      },
    ],
    ...(effect === 'require_approval'
      ? {
          approval: {
            id: 'approval-test',
            status: 'pending' as const,
            envelope_hash: 'hash',
            expires_at: '2026-07-19T12:00:00Z',
            poll_after_ms: 1_000n,
          },
        }
      : {}),
    latency_ms: 8n,
  };
}
