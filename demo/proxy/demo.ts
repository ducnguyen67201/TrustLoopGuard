import { createHash, randomUUID } from 'node:crypto';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

import { API_KEY, SERVER_URL } from '../shared/env';

interface AuthResponse {
  user_id: string;
}

interface WorkspaceResponse {
  id: string;
}

interface RuntimeKeyResponse {
  plaintext_key: string;
}

interface OpenAiChatResponse {
  choices?: Array<{
    message?: {
      content?: string;
    };
    finish_reason?: string;
  }>;
}

const suffix = randomUUID().slice(0, 8);
const agentId = `demo-proxy-agent-${suffix}`;
const providerId = `demo-proxy-provider-${suffix}`;
const profileId = `demo-proxy-profile-${suffix}`;
const routeId = `demo-proxy-route-${suffix}`;

async function main(): Promise<void> {
  const provider = await startMockProvider();
  try {
    const workspaceId = await createWorkspace();
    await createPolicy(workspaceId);
    const runtimeKey = await createRuntimeKey(workspaceId);
    await createProviderConnection(workspaceId, provider.url);
    await createEnforcementProfile(workspaceId);
    await createGatewayRoute(workspaceId);
    await callGateway(workspaceId, runtimeKey);
    if (provider.calls() !== 1) {
      throw new Error(`expected exactly one provider call, saw ${provider.calls()}`);
    }

    process.stdout.write('proxy demo: gateway returned provider-shaped blocked response\n');
    process.stdout.write(`workspace : ${workspaceId}\n`);
    process.stdout.write(`route     : ${routeId}\n`);
    process.stdout.write(`provider  : ${provider.url}\n`);
  } finally {
    await provider.close();
  }
}

async function createWorkspace(): Promise<string> {
  const username = `demo-proxy-${suffix}`;
  const password = createHash('sha256').update(`demo-proxy-${suffix}`).digest('hex');
  const signup = await request<AuthResponse>('/v1/auth/signup', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ username, password }),
    auth: false,
  });

  const workspace = await request<WorkspaceResponse>('/v1/team/my-workspaces', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-tlg-user-id': signup.user_id,
      'x-tlg-user-email': `${username}@example.test`,
    },
    body: JSON.stringify({ name: `Proxy Demo ${suffix}` }),
  });

  return workspace.id;
}

async function createPolicy(workspaceId: string): Promise<void> {
  await request('/v1/policies', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/x-yaml' },
    body: `
id: demo-proxy-block-output-${suffix}
description: Block the mock provider output in the proxy demo.
when:
  channels: [chat]
match:
  literal: unsafe proxy reply
action: block
owner_agent_id: ${agentId}
`.trim(),
  });
}

async function createRuntimeKey(workspaceId: string): Promise<string> {
  const response = await request<RuntimeKeyResponse>('/v1/api-keys', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ name: `Proxy demo ${suffix}` }),
  });
  return response.plaintext_key;
}

async function createProviderConnection(workspaceId: string, providerUrl: string): Promise<void> {
  await request('/v1/gateway/provider-connections', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: providerId,
      display_name: 'Proxy demo mock OpenAI provider',
      kind: 'openai_compatible',
      base_url: providerUrl,
      default_model: 'mock-model',
      provider_api_key: 'provider-secret',
    }),
  });
}

async function createEnforcementProfile(workspaceId: string): Promise<void> {
  await request('/v1/enforcement-profiles', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: profileId,
      display_name: 'Proxy demo strict output',
      input_action: 'allow',
      output_action: 'block',
      fail_mode: 'closed',
      retention_mode: 'metadata_only',
      fallback_message: 'Blocked by TrustLoopGuard proxy demo.',
      max_regenerations: 0,
    }),
  });
}

async function createGatewayRoute(workspaceId: string): Promise<void> {
  await request('/v1/gateway/routes', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: routeId,
      display_name: 'Proxy demo route',
      provider_connection_id: providerId,
      agent_id: agentId,
      enforcement_profile_id: profileId,
    }),
  });
}

async function callGateway(workspaceId: string, runtimeKey: string): Promise<void> {
  const response = await fetch(
    `${SERVER_URL}/v1/gateway/${routeId}/openai/chat/completions`,
    {
      method: 'POST',
      headers: {
        authorization: `Bearer ${runtimeKey}`,
        'content-type': 'application/json',
        'x-tlg-workspace-id': workspaceId,
      },
      body: JSON.stringify({
        model: 'mock-model',
        messages: [{ role: 'user', content: 'hello from the proxy demo' }],
      }),
    },
  );

  const bodyText = await response.text();
  if (!response.ok) {
    throw new Error(`gateway call failed with ${response.status}: ${bodyText}`);
  }

  const body = JSON.parse(bodyText) as OpenAiChatResponse;
  const choice = body.choices?.[0];
  const verdict = response.headers.get('x-trustloopguard-verdict');
  const phase = response.headers.get('x-trustloopguard-phase');
  if (
    choice?.finish_reason !== 'content_filter' ||
    choice.message?.content !== 'Blocked by TrustLoopGuard proxy demo.' ||
    verdict !== 'blocked' ||
    phase !== 'output'
  ) {
    throw new Error(`unexpected gateway response: ${bodyText}`);
  }
}

async function request<T = unknown>(
  path: string,
  init: RequestInit & { auth?: boolean; workspaceId?: string },
): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.workspaceId) headers.set('x-tlg-workspace-id', init.workspaceId);
  if (init.auth !== false && API_KEY) headers.set('authorization', `Bearer ${API_KEY}`);

  const response = await fetch(`${SERVER_URL}${path}`, {
    ...init,
    headers,
  });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}: ${body}`);
  }
  return (body ? JSON.parse(body) : undefined) as T;
}

async function startMockProvider(): Promise<{
  url: string;
  calls: () => number;
  close: () => Promise<void>;
}> {
  let calls = 0;
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    if (
      req.method !== 'POST' ||
      req.url !== '/v1/chat/completions' ||
      req.headers.authorization !== 'Bearer provider-secret'
    ) {
      res.writeHead(404, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ error: 'unexpected mock provider request' }));
      return;
    }

    calls += 1;
    res.writeHead(200, { 'content-type': 'application/json' });
    res.end(
      JSON.stringify({
        id: 'chatcmpl_demo_proxy',
        object: 'chat.completion',
        created: Math.floor(Date.now() / 1000),
        model: 'mock-model',
        choices: [
          {
            index: 0,
            message: { role: 'assistant', content: 'unsafe proxy reply' },
            finish_reason: 'stop',
          },
        ],
        usage: { prompt_tokens: 1, completion_tokens: 3, total_tokens: 4 },
      }),
    );
  });

  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      server.off('error', reject);
      resolve();
    });
  });

  const address = server.address();
  if (address === null || typeof address === 'string') {
    throw new Error('mock provider did not bind a TCP port');
  }

  return {
    url: `http://127.0.0.1:${address.port}`,
    calls: () => calls,
    close: async () => {
      await new Promise<void>((resolve, reject) => {
        server.close((error) => {
          if (error) reject(error);
          else resolve();
        });
      });
    },
  };
}

main().catch((error) => {
  process.stderr.write(`proxy demo failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
