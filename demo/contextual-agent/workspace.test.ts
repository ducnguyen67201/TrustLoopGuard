import assert from 'node:assert/strict';
import test from 'node:test';

import {
  CONTEXTUAL_RUNTIME_KEY_NAME,
  disableContextualStarterPolicies,
  ensureContextualRuntimeKey,
  ensureContextualWorkspace,
  type ContextualWorkspaceAdminConfig,
} from './workspace';
import { CONTEXTUAL_DEMO_WORKSPACE_NAME } from './config';

const ADMIN_USER_ID = '019f7c32-6eb9-7af1-97df-e79964af7bed';

test('reuses the existing shared contextual workspace', async () => {
  const requests: Request[] = [];
  const workspace = workspaceRecord();
  const config = adminConfig(async (input, init) => {
    requests.push(new Request(input, init));
    return Response.json({ workspaces: [workspace] });
  });

  assert.deepEqual(await ensureContextualWorkspace(config), workspace);
  assert.equal(requests.length, 1);
  assert.equal(requests[0]?.method, 'GET');
});

test('creates the shared contextual workspace only when missing', async () => {
  const requests: Request[] = [];
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    if (request.method === 'GET') return Response.json({ workspaces: [] });
    assert.deepEqual(await request.json(), { name: CONTEXTUAL_DEMO_WORKSPACE_NAME });
    return Response.json(workspaceRecord(), { status: 201 });
  });

  await ensureContextualWorkspace(config);
  assert.deepEqual(
    requests.map((request) => `${request.method} ${new URL(request.url).pathname}`),
    ['GET /v1/team/my-workspaces', 'POST /v1/team/my-workspaces'],
  );
});

test('creates one agent-bound runtime key for the shared workspace', async () => {
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    if (request.method === 'GET') return Response.json({ api_keys: [] });
    assert.deepEqual(await request.json(), {
      name: CONTEXTUAL_RUNTIME_KEY_NAME,
      environment_id: 'production',
      principal_id: 'contextual-demo-agent',
    });
    return Response.json({
      api_key: runtimeKeyRecord(),
      plaintext_key: 'tl_live_contextual-secret',
    });
  });

  const result = await ensureContextualRuntimeKey('ws_contextual_demo', 'production', config);
  assert.equal(result.status, 'created');
});

test('disables the default starter policies in the dedicated shared workspace', async () => {
  const updates: { ids: string[]; enabled: boolean }[] = [];
  const disabled = await disableContextualStarterPolicies({
    listPolicies: async () => ({
      policies: [
        policySummary('starter-pii-email', true),
        policySummary('starter-prompt-injection', true),
        policySummary('contextual-readonly-input', true),
      ],
    }),
    batchSetPolicyEnabled: async (ids, enabled) => {
      updates.push({ ids, enabled });
      return { policies: ids.map((id) => policySummary(id, enabled)) };
    },
  });

  assert.deepEqual(disabled, ['starter-pii-email', 'starter-prompt-injection']);
  assert.deepEqual(updates, [
    { ids: ['starter-pii-email', 'starter-prompt-injection'], enabled: false },
  ]);
});

function adminConfig(fetchImpl: typeof fetch): ContextualWorkspaceAdminConfig {
  return {
    serverUrl: 'http://rust.test',
    internalApiKey: 'internal-key',
    adminUserId: ADMIN_USER_ID,
    fetchImpl,
  };
}

function workspaceRecord() {
  return { id: 'ws_contextual_demo', slug: 'contextual-demo', name: CONTEXTUAL_DEMO_WORKSPACE_NAME };
}

function runtimeKeyRecord() {
  return {
    id: 'apk_contextual',
    name: CONTEXTUAL_RUNTIME_KEY_NAME,
    prefix: 'tl_live_contextual-',
    environment_id: 'production',
    environment: 'Production',
    status: 'active',
    created_at: '2026-07-20T12:00:00Z',
    last_used_at: null,
    created_by: ADMIN_USER_ID,
    principal_id: 'contextual-demo-agent',
  };
}

function policySummary(id: string, enabled: boolean) {
  return {
    id,
    family: 'content' as const,
    severity: 'high' as const,
    action: 'deny',
    enabled,
  };
}
