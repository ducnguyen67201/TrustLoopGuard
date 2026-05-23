import { randomUUID } from 'node:crypto';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

import { API_KEY, SERVER_URL } from '../shared/env';

type GatewayExpectation = 'pass_through' | 'blocked_output';

interface ChatScenario {
  label: string;
  userMessage: string;
  providerReply: string;
  expect: GatewayExpectation;
}

interface GatewayRoute {
  workspaceId: string;
  runtimeKey: string;
  openAiBaseUrl: string;
  providerUrl: string;
}

interface AgentTurn {
  scenario: ChatScenario;
  content: string;
  finishReason: string;
  verdict: string | null;
  phase: string | null;
  traceId: string | null;
  latencyMs: number;
}

interface MockProviderCall {
  userMessage: string;
}

interface MockProvider {
  url: string;
  calls: () => readonly MockProviderCall[];
  close: () => Promise<void>;
}

interface WorkspaceResponse {
  id: string;
}

interface RuntimeKeyResponse {
  plaintext_key: string;
}

interface OpenAiChatResponse {
  choices?: OpenAiChoice[];
}

interface OpenAiChoice {
  message?: {
    content?: string;
  };
  finish_reason?: string;
}

const runId = randomUUID().slice(0, 8);

const gatewayIds = {
  agent: `demo-proxy-agent-${runId}`,
  provider: `demo-proxy-provider-${runId}`,
  profile: `demo-proxy-profile-${runId}`,
  route: `demo-proxy-route-${runId}`,
};

const demoConfig = {
  model: 'mock-model',
  providerSecret: 'provider-secret',
  fallbackMessage: 'Blocked by TrustLoopGuard proxy demo.',
  systemPrompt: 'You are a concise support chat agent. Answer with one short sentence.',
};

const chatScenarios: ChatScenario[] = [
  {
    label: 'clean support turn',
    userMessage: 'what time do you open?',
    providerReply: "We're open 9 am to 5 pm on weekdays.",
    expect: 'pass_through',
  },
  {
    label: 'unsafe provider output',
    userMessage: 'send me the private proxy reply',
    providerReply: 'unsafe proxy reply',
    expect: 'blocked_output',
  },
];

async function main(): Promise<void> {
  const provider = await startMockProvider();
  try {
    const route = await configureGatewayRoute(provider.url);
    printDemoStart(route);

    const turns = await runChatAgent(route);
    assertProviderSawEveryPrompt(provider.calls());
    printSummary(turns);
  } finally {
    await provider.close();
  }
}

async function configureGatewayRoute(providerUrl: string): Promise<GatewayRoute> {
  const userId = randomUUID();
  const workspaceId = await createWorkspace(userId);

  await createBlockingPolicy(workspaceId);
  const runtimeKey = await createRuntimeKey(workspaceId, userId);
  await createProviderConnection(workspaceId, providerUrl);
  await createEnforcementProfile(workspaceId);
  await createRoute(workspaceId);

  return {
    workspaceId,
    runtimeKey,
    openAiBaseUrl: `${SERVER_URL}/v1/gateway/${gatewayIds.route}/openai`,
    providerUrl,
  };
}

async function runChatAgent(route: GatewayRoute): Promise<AgentTurn[]> {
  const turns: AgentTurn[] = [];

  for (const scenario of chatScenarios) {
    const turn = await sendChatTurn(route, scenario);
    assertTurnMatchesScenario(turn);
    printTurn(turn);
    turns.push(turn);
  }

  return turns;
}

async function sendChatTurn(route: GatewayRoute, scenario: ChatScenario): Promise<AgentTurn> {
  const startedAt = Date.now();
  const response = await fetch(`${route.openAiBaseUrl}/chat/completions`, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${route.runtimeKey}`,
      'content-type': 'application/json',
      'x-tlg-workspace-id': route.workspaceId,
    },
    body: JSON.stringify({
      model: demoConfig.model,
      messages: [
        { role: 'system', content: demoConfig.systemPrompt },
        { role: 'user', content: scenario.userMessage },
      ],
    }),
  });

  const bodyText = await response.text();
  if (!response.ok) {
    throw new Error(`gateway call failed with ${response.status}: ${bodyText}`);
  }

  const choice = parseOpenAiChatResponse(bodyText).choices?.[0];
  return {
    scenario,
    content: choice?.message?.content ?? '',
    finishReason: choice?.finish_reason ?? '',
    verdict: response.headers.get('x-trustloopguard-verdict'),
    phase: response.headers.get('x-trustloopguard-phase'),
    traceId: response.headers.get('x-trustloopguard-trace-id'),
    latencyMs: Date.now() - startedAt,
  };
}

async function createWorkspace(userId: string): Promise<string> {
  const workspace = await request<WorkspaceResponse>('/v1/team/my-workspaces', {
    method: 'POST',
    headers: {
      'content-type': 'application/json',
      'x-tlg-user-id': userId,
      'x-tlg-user-email': `demo-proxy-${runId}@example.test`,
    },
    body: JSON.stringify({ name: `Proxy Demo ${runId}` }),
  });

  return workspace.id;
}

async function createBlockingPolicy(workspaceId: string): Promise<void> {
  await request('/v1/policies', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/x-yaml' },
    body: `
id: demo-proxy-block-output-${runId}
description: Block the mock provider output in the proxy demo.
when:
  channels: [chat]
match:
  literal: unsafe proxy reply
action: block
owner_agent_id: ${gatewayIds.agent}
`.trim(),
  });
}

// This block of code will get the api key from the server, which is needed to authenticate when sending messages to the gateway route. In a production scenario, you would create an API key manually in the dashboard and use it directly in your application without needing to call this endpoint at runtime.
async function createRuntimeKey(workspaceId: string, userId: string): Promise<string> {
  const response = await request<RuntimeKeyResponse>('/v1/api-keys', {
    method: 'POST',
    workspaceId,
    headers: {
      'content-type': 'application/json',
      'x-tlg-user-id': userId,
    },
    body: JSON.stringify({ name: `Proxy demo ${runId}` }),
  });

  return response.plaintext_key;
}

async function createProviderConnection(workspaceId: string, providerUrl: string): Promise<void> {
  await request('/v1/gateway/provider-connections', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: gatewayIds.provider,
      display_name: 'Proxy demo mock OpenAI provider',
      kind: 'openai_compatible',
      base_url: providerUrl,
      default_model: demoConfig.model,
      provider_api_key: demoConfig.providerSecret,
    }),
  });
}

async function createEnforcementProfile(workspaceId: string): Promise<void> {
  await request('/v1/enforcement-profiles', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: gatewayIds.profile,
      display_name: 'Proxy demo strict output',
      input_action: 'allow',
      output_action: 'block',
      fail_mode: 'closed',
      retention_mode: 'metadata_only',
      fallback_message: demoConfig.fallbackMessage,
      max_regenerations: 0,
    }),
  });
}

async function createRoute(workspaceId: string): Promise<void> {
  await request('/v1/gateway/routes', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      id: gatewayIds.route,
      display_name: 'Proxy demo route',
      provider_connection_id: gatewayIds.provider,
      agent_id: gatewayIds.agent,
      enforcement_profile_id: gatewayIds.profile,
    }),
  });
}

function assertTurnMatchesScenario(turn: AgentTurn): void {
  const expected = expectedGatewayResult(turn.scenario);
  const actual = {
    content: turn.content,
    finishReason: turn.finishReason,
    verdict: turn.verdict,
    phase: turn.phase,
  };

  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(
      `unexpected ${turn.scenario.label} response: ${JSON.stringify({ expected, actual })}`,
    );
  }

  if (turn.scenario.expect === 'blocked_output' && !turn.traceId) {
    throw new Error(`blocked ${turn.scenario.label} did not include a trace id`);
  }
}

function expectedGatewayResult(scenario: ChatScenario): Omit<AgentTurn, 'scenario' | 'latencyMs' | 'traceId'> {
  if (scenario.expect === 'pass_through') {
    return {
      content: scenario.providerReply,
      finishReason: 'stop',
      verdict: null,
      phase: null,
    };
  }

  return {
    content: demoConfig.fallbackMessage,
    finishReason: 'content_filter',
    verdict: 'blocked',
    phase: 'output',
  };
}

function assertProviderSawEveryPrompt(calls: readonly MockProviderCall[]): void {
  const expectedPrompts = chatScenarios.map((scenario) => scenario.userMessage);
  const actualPrompts = calls.map((call) => call.userMessage);

  if (JSON.stringify(actualPrompts) !== JSON.stringify(expectedPrompts)) {
    throw new Error(
      `provider saw unexpected prompts: ${JSON.stringify({ expectedPrompts, actualPrompts })}`,
    );
  }
}

function printDemoStart(route: GatewayRoute): void {
  process.stdout.write('proxy demo: OpenAI-compatible chat agent through TrustLoopGuard\n');
  process.stdout.write(`workspace : ${route.workspaceId}\n`);
  process.stdout.write(`route     : ${gatewayIds.route}\n`);
  process.stdout.write(`baseURL   : ${route.openAiBaseUrl}\n`);
  process.stdout.write(`provider  : ${route.providerUrl}\n\n`);
}

function printTurn(turn: AgentTurn): void {
  process.stdout.write(`chat scenario: ${turn.scenario.label}\n`);
  process.stdout.write(`  user   : ${turn.scenario.userMessage}\n`);
  process.stdout.write(`  agent  : ${turn.content}\n`);
  process.stdout.write(`  finish : ${turn.finishReason}\n`);
  process.stdout.write(
    `  guard  : verdict=${turn.verdict ?? 'none'} phase=${turn.phase ?? 'none'} trace=${
      turn.traceId ?? '(none)'
    } latency=${turn.latencyMs} ms\n\n`,
  );
}

function printSummary(turns: readonly AgentTurn[]): void {
  const latencies = turns.map((turn) => turn.latencyMs).sort((a, b) => a - b);
  const avg = Math.round(latencies.reduce((sum, value) => sum + value, 0) / latencies.length);
  const p95 = percentile(latencies, 0.95);

  process.stdout.write('='.repeat(72) + '\n');
  process.stdout.write(`Gateway proxy: ${turns.length} chat turns, avg=${avg} ms, p95=${p95} ms\n`);
  process.stdout.write('Clean responses have no enforcement headers; blocked output keeps OpenAI shape.\n');
}

function percentile(sorted: readonly number[], fraction: number): number {
  if (sorted.length === 0) return 0;
  const index = Math.min(sorted.length - 1, Math.ceil(sorted.length * fraction) - 1);
  return sorted[index] ?? 0;
}

async function request<T = unknown>(
  path: string,
  init: RequestInit & { workspaceId?: string },
): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.workspaceId) headers.set('x-tlg-workspace-id', init.workspaceId);
  if (API_KEY) headers.set('authorization', `Bearer ${API_KEY}`);

  const response = await fetch(`${SERVER_URL}${path}`, { ...init, headers });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}: ${body}`);
  }

  return (body ? JSON.parse(body) : undefined) as T;
}

function parseOpenAiChatResponse(bodyText: string): OpenAiChatResponse {
  return JSON.parse(bodyText) as OpenAiChatResponse;
}

async function startMockProvider(): Promise<MockProvider> {
  const calls: MockProviderCall[] = [];
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    void handleMockProviderRequest(req, res, calls).catch((error: unknown) => {
      res.writeHead(500, { 'content-type': 'application/json' });
      res.end(JSON.stringify({ error: error instanceof Error ? error.message : String(error) }));
    });
  });

  await listenOnRandomLocalPort(server);

  const address = server.address();
  if (address === null || typeof address === 'string') {
    throw new Error('mock provider did not bind a TCP port');
  }

  return {
    url: `http://127.0.0.1:${address.port}`,
    calls: () => calls,
    close: () => closeServer(server),
  };
}

async function handleMockProviderRequest(
  req: IncomingMessage,
  res: ServerResponse,
  calls: MockProviderCall[],
): Promise<void> {
  if (!isExpectedProviderRequest(req)) {
    res.writeHead(404, { 'content-type': 'application/json' });
    res.end(JSON.stringify({ error: 'unexpected mock provider request' }));
    return;
  }

  const body = await readJsonRequest(req);
  const userMessage = extractLastUserMessage(body);
  const providerReply = findProviderReply(userMessage);

  calls.push({ userMessage });
  res.writeHead(200, { 'content-type': 'application/json' });
  res.end(JSON.stringify(openAiChatCompletion(providerReply)));
}

function isExpectedProviderRequest(req: IncomingMessage): boolean {
  return (
    req.method === 'POST' &&
    req.url === '/v1/chat/completions' &&
    req.headers.authorization === `Bearer ${demoConfig.providerSecret}`
  );
}

function findProviderReply(userMessage: string): string {
  return (
    chatScenarios.find((scenario) => scenario.userMessage === userMessage)?.providerReply ??
    "I don't have a scripted answer for that prompt."
  );
}

function openAiChatCompletion(content: string): OpenAiChatResponse {
  return {
    choices: [
      {
        message: { content },
        finish_reason: 'stop',
      },
    ],
  };
}

async function readJsonRequest(req: IncomingMessage): Promise<unknown> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }

  const text = Buffer.concat(chunks).toString('utf8');
  return text ? JSON.parse(text) : {};
}

function extractLastUserMessage(body: unknown): string {
  if (!isRecord(body) || !Array.isArray(body.messages)) return '';

  const messages = [...body.messages].reverse();
  for (const message of messages) {
    if (isRecord(message) && message.role === 'user' && typeof message.content === 'string') {
      return message.content;
    }
  }

  return '';
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

async function listenOnRandomLocalPort(server: ReturnType<typeof createServer>): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      server.off('error', reject);
      resolve();
    });
  });
}

async function closeServer(server: ReturnType<typeof createServer>): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => {
      if (error) reject(error);
      else resolve();
    });
  });
}

main().catch((error) => {
  process.stderr.write(`proxy demo failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
