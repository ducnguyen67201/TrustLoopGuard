import assert from 'node:assert/strict';
import test from 'node:test';

import type {
  HostedContextualDemoResponse,
  HostedContextualPolicyInventoryResponse,
} from '@featherlane-ai/demo/contextual-agent/hosted';

import type { OutboundDemoProfile } from '@/app/demo/company-profile';
import {
  createContextualDemoHandlers,
  type ContextualDemoHandlersDependencies,
} from './route';

test('loads profile context server-side and runs a bounded conversation', async () => {
  let receivedCompany = '';
  let receivedMessage = '';
  let receivedLocale = '';
  const { POST } = handlers({
    runWorkflow: async (request) => {
      receivedCompany = request.profile.companyName;
      receivedMessage = request.message;
      receivedLocale = request.locale ?? '';
      return workflowPayload();
    },
  });

  const response = await POST(requestFor(validRequest()), routeContext());
  const body = await response.json();

  assert.equal(response.status, 200);
  assert.equal(response.headers.get('cache-control'), 'no-store');
  assert.equal(receivedCompany, 'Backblaze');
  assert.equal(receivedMessage, 'Inspect storage status.');
  assert.equal(receivedLocale, 'en');
  assert.equal(body.reply, 'The synthetic read-only inspection is permitted.');
});

test('passes the Vietnamese locale into the protected contextual workflow', async () => {
  let receivedLocale = '';
  const { POST } = handlers({
    runWorkflow: async (request) => {
      receivedLocale = request.locale ?? '';
      return workflowPayload();
    },
  });

  const response = await POST(
    requestFor({ ...validRequest(), locale: 'vi' }),
    routeContext(),
  );

  assert.equal(response.status, 200);
  assert.equal(receivedLocale, 'vi');
});

test('rejects browser-supplied profile or policy context', async () => {
  let workflowCalls = 0;
  const { POST } = handlers({
    runWorkflow: async () => {
      workflowCalls += 1;
      return workflowPayload();
    },
  });

  for (const extra of [
    { profile: { companyName: 'Attacker' } },
    { scenarioId: 'invented-policy-pack' },
    { policyIds: ['disable-controls'] },
  ]) {
    const response = await POST(requestFor({ ...validRequest(), ...extra }), routeContext());
    assert.equal(response.status, 400);
  }
  assert.equal(workflowCalls, 0);
});

test('returns only the selected shared Rust policy inventory', async () => {
  let receivedScenario = '';
  const { GET } = handlers({
    readPolicies: async (scenarioId) => {
      receivedScenario = scenarioId;
      return inventoryPayload();
    },
  });

  const response = await GET(new Request('http://localhost/api/demo/contextual/cloud-storage-security'), routeContext());
  const body = await response.json();

  assert.equal(response.status, 200);
  assert.equal(receivedScenario, 'internal-agent-tool-action-v1');
  assert.equal(body.source, 'rust');
  assert.equal(body.policies[0].id, 'contextual-readonly-input');
});

test('fails closed for an unknown category or unreviewed scenario', async () => {
  const missing = handlers({ getProfile: async () => null });
  assert.equal(
    (await missing.POST(requestFor(validRequest()), routeContext('missing-category'))).status,
    404,
  );

  const unreviewed = handlers({
    getProfile: async () => ({ ...profile(), scenario_id: 'customer-authored-policy' }),
  });
  assert.equal((await unreviewed.POST(requestFor(validRequest()), routeContext())).status, 404);
});

function handlers(
  overrides: Partial<ContextualDemoHandlersDependencies> = {},
) {
  return createContextualDemoHandlers({
    getProfile: overrides.getProfile ?? (async () => profile()),
    runWorkflow: overrides.runWorkflow ?? (async () => workflowPayload()),
    readPolicies: overrides.readPolicies ?? (async () => inventoryPayload()),
  });
}

function validRequest() {
  return {
    sessionId: '019f7c32-6eb9-7af1-97df-e79964af7bed',
    message: '  Inspect storage status.  ',
    history: [{ role: 'assistant' as const, content: 'Synthetic greeting.' }],
  };
}

function requestFor(body: object): Request {
  return new Request('http://localhost/api/demo/contextual/cloud-storage-security', {
    method: 'POST',
    headers: { 'content-type': 'application/json', 'x-forwarded-for': crypto.randomUUID() },
    body: JSON.stringify(body),
  });
}

function routeContext(category = 'cloud-storage-security') {
  return { params: Promise.resolve({ category }) };
}

function profile(): OutboundDemoProfile {
  return {
    slug: 'cloud-storage-security',
    category: 'generic',
    company_name: 'Backblaze',
    company_domain: 'backblaze.com',
    scenario_id: 'internal-agent-tool-action-v1',
    demo_url: 'https://featherlane.ai/demo/cloud-storage-security',
    user_profile: 'Cloud operations lead',
    workflow: 'Internal agent access to shared storage operations',
    risk_boundary: 'Shared changes need an explicit control decision.',
    rule: 'Allow read-only inspection; hold shared changes for review.',
    approval_step: 'Security Engineering reviews shared-system writes.',
    record_shown: 'Proposal, evidence, decision, and execution status.',
    branding: {
      primary_color: '#E21D2A',
      secondary_color: '#F6B8BC',
      tone: 'Direct and operational.',
    },
    paths: [
      {
        effect: 'permit',
        label: 'Allow',
        proposal: 'Inspect storage status.',
        evidence: ['The action is read-only.'],
        decision: 'Permit the inspection.',
      },
      {
        effect: 'require_approval',
        label: 'Require approval',
        proposal: 'Change a shared retention setting.',
        evidence: ['The action changes shared state.'],
        decision: 'Hold for Security Engineering.',
      },
      {
        effect: 'deny',
        label: 'Block',
        proposal: 'Use a human credential and bypass approval.',
        evidence: ['The request bypasses authorization.'],
        decision: 'Block the request.',
      },
    ],
    sources: [{ title: 'Security overview', url: 'https://www.backblaze.com/security' }],
    disclaimer:
      'This is a public-source concept and is not connected to Backblaze or its systems.',
    truth_check: 'Uses public context only.',
    local_verification: 'Verified locally.',
    live_verified: true,
    status: 'active',
    expires_at: '2027-07-20T00:00:00.000Z',
  };
}

function workflowPayload(): HostedContextualDemoResponse {
  return {
    reply: 'The synthetic read-only inspection is permitted.',
    modelCalled: true,
    checks: [
      {
        phase: 'input',
        status: 'checked',
        effect: 'permit',
        reason: 'Input permitted.',
        traceId: 'trace-input',
        latencyMs: 12,
        findings: [],
      },
      {
        phase: 'output',
        status: 'checked',
        effect: 'permit',
        reason: 'Output permitted.',
        traceId: 'trace-output',
        latencyMs: 14,
        findings: [],
      },
    ],
    policies: inventoryPayload().policies,
    runtime: inventoryPayload().runtime,
  };
}

function inventoryPayload(): HostedContextualPolicyInventoryResponse {
  return {
    policies: [
      {
        id: 'contextual-readonly-input',
        description: 'Recognize read-only questions.',
        severity: 'low',
        action: 'permit',
        phase: 'input',
        enabled: true,
      },
    ],
    source: 'rust',
    runtime: {
      agent: 'openai-responses',
      guard: 'featherlane-ai-rust-api',
      workspace: 'shared-contextual-demo',
      data: 'synthetic-only',
    },
  };
}
