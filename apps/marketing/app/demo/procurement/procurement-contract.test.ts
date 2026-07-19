import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

import {
  parseProcurementDemoRequest,
  sanitizeProcurementDemoResponse,
  PROCUREMENT_POLICY_IDS,
} from './contract';

test('defaults to all policies and normalizes custom policy order', () => {
  assert.deepEqual(parseProcurementDemoRequest({ prompt: '  Order chairs.  ' }), {
    prompt: 'Order chairs.',
    activePolicyIds: [...PROCUREMENT_POLICY_IDS],
  });
  assert.deepEqual(
    parseProcurementDemoRequest({
      prompt: 'Order chairs.',
      activePolicyIds: [
        'procurement-restricted-categories',
        'procurement-approved-suppliers',
        'procurement-approved-suppliers',
      ],
    }).activePolicyIds,
    ['procurement-approved-suppliers', 'procurement-restricted-categories'],
  );
});

test('rejects invalid public procurement inputs', () => {
  assert.throws(() => parseProcurementDemoRequest({ prompt: '   ' }), /prompt/i);
  assert.throws(() => parseProcurementDemoRequest({ prompt: 'x'.repeat(501) }), /500/);
  assert.throws(
    () =>
      parseProcurementDemoRequest({
        prompt: 'Order chairs.',
        activePolicyIds: ['not-a-policy'],
      }),
    /policy|option/i,
  );
  assert.throws(
    () => parseProcurementDemoRequest({ prompt: 'Order chairs.', agentId: 'attacker' }),
    /unrecognized|key/i,
  );
});

test('exposes only bounded public decision and purchase-order fields', () => {
  const response = sanitizeProcurementDemoResponse({
    result: {
      finalMessage: 'This purchase requires approval.',
      traces: [
        {
          tool: 'submit_purchase_order',
          summary: 'Held by TrustLoopGuard.',
          rawToolOutput: 'private',
        },
      ],
      decision: {
        traceId: 'trace-public',
        effect: 'require_approval',
        reason: 'High-value purchase orders require review.',
        latencyMs: 7,
        findings: [
          {
            id: 'finding-public',
            effect: 'require_approval',
            reason: 'High-value purchase orders require review.',
            severity: 'high',
            policyId: 'procurement-high-value-review',
            evidence: { supplierPrivateContact: 'hidden' },
          },
        ],
        approvalId: 'approval-public',
        lease: { id: 'lease-private' },
        grant: { id: 'grant-private' },
      },
      modelUsage: { tokens: 100 },
    },
    state: {
      purchaseOrders: [],
      supplierPrivateContacts: ['hidden'],
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
    runtime: {
      agent: 'openai-agents-js',
      guard: 'trustloopguard-rust-api',
      provider: 'simulated-procurement-api',
    },
    logs: [{ step: 'authorization_requested' }],
    apiKey: 'private',
  });

  assert.equal(response.result.decision?.effect, 'require_approval');
  assert.equal('rawToolOutput' in response.result.traces[0]!, false);
  assert.equal('modelUsage' in response.result, false);
  assert.equal('lease' in response.result.decision!, false);
  assert.equal('grant' in response.result.decision!, false);
  assert.equal('evidence' in response.result.decision!.findings[0]!, false);
  assert.equal('logs' in response, false);
  assert.equal('apiKey' in response, false);
});

test('the page presents live OpenAI, Rust policies, all outcomes, and accessible controls', () => {
  const page = readFileSync(new URL('./page.tsx', import.meta.url), 'utf8');
  const demo = readFileSync(new URL('./procurement-demo.tsx', import.meta.url), 'utf8');
  const source = `${page}\n${demo}`;

  assert.match(source, /OpenAI proposes/i);
  assert.match(source, /TrustLoopGuard decides/i);
  assert.match(source, /Demo catalog only/i);
  assert.match(source, /Approved suppliers only/i);
  assert.match(source, /Review high-value orders/i);
  assert.match(source, /Block restricted categories/i);
  assert.match(source, /role="switch"/);
  assert.match(source, /aria-live="polite"/);
  assert.match(source, /require_approval/);
  assert.match(source, /effect === 'deny'/);
  assert.match(source, /effect === 'permit'/);
});

test('analytics events omit prompts and runtime identifiers', () => {
  const demo = readFileSync(new URL('./procurement-demo.tsx', import.meta.url), 'utf8');

  assert.match(demo, /trackMarketingEvent\('demo_started'/);
  assert.match(demo, /trackMarketingEvent\('demo_decision_shown'/);
  assert.match(demo, /trackMarketingEvent\('demo_policy_changed'/);

  const analyticsCalls = demo.match(/trackMarketingEvent\([\s\S]*?\n\s*\}\);/g) ?? [];
  assert.equal(analyticsCalls.length, 4);
  for (const analyticsCall of analyticsCalls) {
    assert.doesNotMatch(analyticsCall, /prompt\s*:/);
    assert.doesNotMatch(analyticsCall, /traceId|approvalId|purchaseOrderId/);
  }
});
