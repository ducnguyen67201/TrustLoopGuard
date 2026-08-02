import assert from 'node:assert/strict';
import test from 'node:test';

import { createHealthcareRuntimeClient } from './runtime-client';
import {
  ensureHealthcareRuntimeKey,
  ensureHealthcareWorkspace,
  HEALTHCARE_RUNTIME_KEY_NAME,
  HEALTHCARE_WORKSPACE_NAME,
  resolveHealthcareEnvironment,
  type HealthcareWorkspaceAdminConfig,
} from './workspace';

const ADMIN_USER_ID = '019f7c32-6eb9-7af1-97df-e79964af7bed';

test('reuses the existing dedicated healthcare workspace', async () => {
  const requests: Request[] = [];
  const workspace = workspaceRecord();
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    return Response.json({ workspaces: [workspace] });
  });

  assert.deepEqual(await ensureHealthcareWorkspace(config), workspace);
  assert.equal(requests.length, 1);
  assert.equal(requests[0]?.method, 'GET');
  assert.equal(requests[0]?.headers.get('authorization'), 'Bearer internal-key');
  assert.equal(requests[0]?.headers.get('x-featherlane-ai-user-id'), ADMIN_USER_ID);
});

test('creates the dedicated healthcare workspace only when it is missing', async () => {
  const requests: Request[] = [];
  const workspace = workspaceRecord();
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    if (request.method === 'GET') return Response.json({ workspaces: [] });
    assert.deepEqual(await request.json(), { name: HEALTHCARE_WORKSPACE_NAME });
    return Response.json(workspace, { status: 201 });
  });

  assert.deepEqual(await ensureHealthcareWorkspace(config), workspace);
  assert.deepEqual(
    requests.map((request) => `${request.method} ${new URL(request.url).pathname}`),
    ['GET /v1/team/my-workspaces', 'POST /v1/team/my-workspaces'],
  );
});

test('rejects ambiguous duplicate healthcare workspaces instead of selecting one silently', async () => {
  const config = adminConfig(async () =>
    Response.json({
      workspaces: [
        workspaceRecord(),
        { ...workspaceRecord(), id: 'ws_healthcare_demo_duplicate', slug: 'healthcare-demo-2' },
      ],
    }),
  );

  await assert.rejects(
    () => ensureHealthcareWorkspace(config),
    /multiple workspaces named "Healthcare Demo"/i,
  );
});

test('resolves the Rust-owned default environment for policy and key scope', async () => {
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    assert.equal(request.headers.get('x-featherlane-ai-workspace-id'), 'ws_healthcare_demo');
    return Response.json({
      environments: [
        environmentRecord('staging', false),
        environmentRecord('production', true),
      ],
    });
  });

  assert.deepEqual(
    await resolveHealthcareEnvironment('ws_healthcare_demo', config),
    environmentRecord('production', true),
  );
});

test('creates one agent-bound runtime key when the workspace has none', async () => {
  const requests: Request[] = [];
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    requests.push(request);
    if (request.method === 'GET') return Response.json({ api_keys: [] });
    assert.deepEqual(await request.json(), {
      name: HEALTHCARE_RUNTIME_KEY_NAME,
      environment_id: 'production',
      principal_id: 'healthcare-demo-agent',
    });
    return Response.json(
      {
        api_key: runtimeKeyRecord(),
        plaintext_key: 'tl_live_healthcare-secret',
      },
      { status: 201 },
    );
  });

  const result = await ensureHealthcareRuntimeKey('ws_healthcare_demo', 'production', config);

  assert.deepEqual(result, {
    status: 'created',
    apiKey: runtimeKeyRecord(),
    plaintextKey: 'tl_live_healthcare-secret',
  });
  assert.equal(requests[1]?.headers.get('x-featherlane-ai-workspace-id'), 'ws_healthcare_demo');
});

test('reuses runtime-key metadata without rotating or exposing another secret', async () => {
  let requestCount = 0;
  const config = adminConfig(async () => {
    requestCount += 1;
    return Response.json({ api_keys: [runtimeKeyRecord()] });
  });

  const result = await ensureHealthcareRuntimeKey('ws_healthcare_demo', 'production', config);

  assert.deepEqual(result, { status: 'existing', apiKey: runtimeKeyRecord() });
  assert.equal(requestCount, 1);
});

test('rejects a configured runtime secret that belongs to another key', async () => {
  const config = {
    ...adminConfig(async () => Response.json({ api_keys: [runtimeKeyRecord()] })),
    runtimeApiKey: 'tl_live_other-workspace-secret',
  };

  await assert.rejects(
    () => ensureHealthcareRuntimeKey('ws_healthcare_demo', 'production', config),
    /does not match the active Healthcare demo runtime key/,
  );
});

test('does not reuse a same-name key from another environment', async () => {
  let requestCount = 0;
  const config = adminConfig(async (input, init) => {
    const request = new Request(input, init);
    requestCount += 1;
    if (request.method === 'GET') {
      return Response.json({
        api_keys: [{ ...runtimeKeyRecord(), environment_id: 'staging' }],
      });
    }
    assert.deepEqual(await request.json(), {
      name: HEALTHCARE_RUNTIME_KEY_NAME,
      environment_id: 'production',
      principal_id: 'healthcare-demo-agent',
    });
    return Response.json({
      api_key: runtimeKeyRecord(),
      plaintext_key: 'tl_live_healthcare-secret',
    });
  });

  const result = await ensureHealthcareRuntimeKey('ws_healthcare_demo', 'production', config);

  assert.equal(result.status, 'created');
  assert.equal(requestCount, 2);
});

test('hosted client uses only the workspace runtime key and sends no workspace override', async () => {
  let request: Request | undefined;
  const client = createHealthcareRuntimeClient({
    serverUrl: 'http://rust.test',
    runtimeApiKey: 'tl_live_healthcare-secret',
    fetchImpl: async (input, init) => {
      request = new Request(input, init);
      return Response.json({ policies: [] });
    },
  });

  await client.listPolicies({ family: 'content' });

  assert.equal(request?.headers.get('authorization'), 'Bearer tl_live_healthcare-secret');
  assert.equal(request?.headers.get('x-featherlane-ai-workspace-id'), null);
  assert.equal(request?.headers.get('x-featherlane-ai-user-id'), null);
});

test('hosted client rejects missing and non-runtime credentials', () => {
  assert.throws(
    () => createHealthcareRuntimeClient({ serverUrl: 'http://rust.test', runtimeApiKey: '' }),
    /TL_HEALTHCARE_DEMO_API_KEY is required/,
  );
  assert.throws(
    () =>
      createHealthcareRuntimeClient({
        serverUrl: 'http://rust.test',
        runtimeApiKey: 'internal-key',
      }),
    /must be a workspace runtime key/,
  );
});

function adminConfig(fetchImpl: typeof fetch): HealthcareWorkspaceAdminConfig {
  return {
    serverUrl: 'http://rust.test',
    internalApiKey: 'internal-key',
    adminUserId: ADMIN_USER_ID,
    fetchImpl,
  };
}

function workspaceRecord() {
  return {
    id: 'ws_healthcare_demo',
    slug: 'healthcare-demo',
    name: HEALTHCARE_WORKSPACE_NAME,
  };
}

function runtimeKeyRecord() {
  return {
    id: 'apk_healthcare',
    name: HEALTHCARE_RUNTIME_KEY_NAME,
    prefix: 'tl_live_healthcare-',
    environment_id: 'production',
    environment: 'Production',
    status: 'active',
    created_at: '2026-07-19T12:00:00Z',
    last_used_at: null,
    created_by: ADMIN_USER_ID,
    principal_id: 'healthcare-demo-agent',
  };
}

function environmentRecord(id: string, isDefault: boolean) {
  return {
    id,
    slug: id,
    name: id === 'production' ? 'Production' : 'Staging',
    is_default: isDefault,
  };
}
