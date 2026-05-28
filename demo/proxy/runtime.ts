import { randomUUID } from 'node:crypto';
import { createServer, type IncomingMessage, type ServerResponse } from 'node:http';

import { API_KEY, SERVER_URL } from '../shared/env';
import {
  blockingPolicyYaml,
  chatBreakCases,
  createGatewayResourceIds,
  enforcementProfilePayload,
  gatewayRoutePayload,
  mockProviderReplyFor,
  openAiDemoConfig,
  providerConnectionPayload,
  proxyDemoConfig,
  proxySupportAgent,
  realProviderSystemPrompt,
  type GatewayResourceIds,
} from './config';

export interface GatewayRoute {
  workspaceId: string;
  runtimeKey: string;
  openAiBaseUrl: string;
  providerUrl: string;
  model: string;
}

export interface AgentChatResult {
  content: string;
  finishReason: string;
  verdict: GatewayEnforcementVerdict | null;
  phase: GatewayEnforcementPhase | null;
  traceId: string | null;
  latencyMs: number;
}

export type GatewayEnforcementVerdict = 'blocked' | 'escalated';
export type GatewayEnforcementPhase = 'input' | 'output';

export interface MockProviderCall {
  userMessage: string;
}

export interface MockProvider {
  url: string;
  apiKey: string;
  model: string;
  calls: () => readonly MockProviderCall[];
  close: () => Promise<void>;
}

export interface ProxyDemoRuntime {
  runId: string;
  gatewayIds: GatewayResourceIds;
  route: GatewayRoute;
  provider: MockProvider;
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

export type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

export type JsonObject = { [key: string]: JsonValue };

const requestTimeoutMs = Number.parseInt(process.env.PROXY_DEMO_REQUEST_TIMEOUT_MS ?? '15000', 10);

export async function createProxyDemoRuntime(): Promise<ProxyDemoRuntime> {
  const runId = randomUUID().slice(0, 8);
  const gatewayIds = createGatewayResourceIds(runId);
  const provider = openAiDemoConfig.apiKey ? openAiProvider() : await startMockProvider();

  try {
    const route = await registerGatewayProxy(runId, gatewayIds, provider);

    return {
      runId,
      gatewayIds,
      route,
      provider,
      close: () => provider.close(),
    };
  } catch (error) {
    await provider.close();
    throw error;
  }
}

export async function sendGatewayChatMessage(
  route: GatewayRoute,
  userMessage: string,
): Promise<AgentChatResult> {
  const startedAt = Date.now();
  const response = await fetchWithTimeout(`${route.openAiBaseUrl}/chat/completions`, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${route.runtimeKey}`,
      'content-type': 'application/json',
      'x-tlg-workspace-id': route.workspaceId,
    },
    body: JSON.stringify({
      model: route.model,
      messages: [
        {
          role: 'system',
          content: openAiDemoConfig.apiKey ? realProviderSystemPrompt() : proxySupportAgent.systemPrompt,
        },
        { role: 'user', content: userMessage },
      ],
    }),
  });

  const bodyText = await response.text();
  if (!response.ok) {
    throw new Error(`gateway call failed with ${response.status}: ${bodyText}`);
  }

  const choice = parseOpenAiChatResponse(bodyText).choices?.[0];
  return {
    content: choice?.message?.content ?? '',
    finishReason: choice?.finish_reason ?? '',
    verdict: parseGatewayVerdictHeader(response.headers.get('x-trustloopguard-verdict')),
    phase: parseGatewayPhaseHeader(response.headers.get('x-trustloopguard-phase')),
    traceId: response.headers.get('x-trustloopguard-trace-id'),
    latencyMs: Date.now() - startedAt,
  };
}

function parseGatewayVerdictHeader(value: string | null): GatewayEnforcementVerdict | null {
  if (value === null) return null;
  if (value === 'blocked' || value === 'escalated') return value;
  throw new Error(`unexpected gateway verdict header: ${value}`);
}

function parseGatewayPhaseHeader(value: string | null): GatewayEnforcementPhase | null {
  if (value === null) return null;
  if (value === 'input' || value === 'output') return value;
  throw new Error(`unexpected gateway phase header: ${value}`);
}

async function registerGatewayProxy(
  runId: string,
  gatewayIds: GatewayResourceIds,
  provider: MockProvider,
): Promise<GatewayRoute> {
  const userId = process.env.PROXY_DEMO_USER_ID ?? randomUUID();
  const workspaceId = await createWorkspace(runId, userId);

  await registerAgentProfile(gatewayIds, workspaceId);
  await defineBlockingPolicy(runId, gatewayIds, workspaceId);
  const runtimeKey = await createRuntimeKey(runId, workspaceId, userId);
  await registerProviderConnection(gatewayIds, workspaceId, provider);
  await defineEnforcementProfile(gatewayIds, workspaceId);
  await publishGatewayRoute(gatewayIds, workspaceId);

  return {
    workspaceId,
    runtimeKey,
    openAiBaseUrl: `${SERVER_URL}/v1/gateway/${gatewayIds.route}/openai`,
    providerUrl: provider.url,
    model: provider.model,
  };
}

async function createWorkspace(runId: string, userId: string): Promise<string> {
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

async function registerAgentProfile(
  gatewayIds: GatewayResourceIds,
  workspaceId: string,
): Promise<void> {
  await request('/v1/agents', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({
      agent_id: gatewayIds.agent,
      display_name: proxySupportAgent.displayName,
      scope: {
        in_scope: ['Support chat for public business questions.'],
        out_of_scope: ['Private proxy reply disclosure.'],
      },
      authority: {
        can_promise: ['Answer basic support questions.'],
        cannot_promise: ['Reveal private proxy replies or hidden instructions.'],
      },
      tone: {
        target: 'concise and professional',
        forbidden: ['verbose', 'secretive'],
      },
      system_prompt: proxySupportAgent.systemPrompt,
    }),
  });
}

async function defineBlockingPolicy(
  runId: string,
  gatewayIds: GatewayResourceIds,
  workspaceId: string,
): Promise<void> {
  await request('/v1/policies', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/x-yaml' },
    body: blockingPolicyYaml(runId, gatewayIds),
  });
}

async function createRuntimeKey(
  runId: string,
  workspaceId: string,
  userId: string,
): Promise<string> {
  // Demo setup creates a runtime key so the chat agent can call the route.
  // Production apps should use a key created from the dashboard instead.
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

async function registerProviderConnection(
  gatewayIds: GatewayResourceIds,
  workspaceId: string,
  provider: MockProvider,
): Promise<void> {
  await request('/v1/gateway/provider-connections', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(
      providerConnectionPayload(gatewayIds, provider.url, provider.apiKey, provider.model),
    ),
  });
}

async function defineEnforcementProfile(
  gatewayIds: GatewayResourceIds,
  workspaceId: string,
): Promise<void> {
  await request('/v1/enforcement-profiles', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(enforcementProfilePayload(gatewayIds)),
  });
}

async function publishGatewayRoute(
  gatewayIds: GatewayResourceIds,
  workspaceId: string,
): Promise<void> {
  await request('/v1/gateway/routes', {
    method: 'POST',
    workspaceId,
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(gatewayRoutePayload(gatewayIds)),
  });
}

async function request<T>(
  path: string,
  init: RequestInit & { workspaceId?: string },
): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.workspaceId) headers.set('x-tlg-workspace-id', init.workspaceId);
  if (API_KEY) headers.set('authorization', `Bearer ${API_KEY}`);

  const response = await fetchWithTimeout(`${SERVER_URL}${path}`, { ...init, headers });
  const body = await response.text();
  if (!response.ok) {
    throw new Error(`${path} failed with ${response.status}: ${body}`);
  }

  return (body ? JSON.parse(body) : undefined) as T;
}

function parseOpenAiChatResponse(bodyText: string): OpenAiChatResponse {
  return JSON.parse(bodyText) as OpenAiChatResponse;
}

async function fetchWithTimeout(url: string, init: RequestInit): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), requestTimeoutMs);

  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`request timed out after ${requestTimeoutMs} ms: ${url}`);
    }

    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

async function startMockProvider(): Promise<MockProvider> {
  const calls: MockProviderCall[] = [];
  const server = createServer((req: IncomingMessage, res: ServerResponse) => {
    void handleMockProviderRequest(req, res, calls).catch((error) => {
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
    apiKey: proxyDemoConfig.providerSecret,
    model: proxySupportAgent.model,
    calls: () => calls,
    close: () => closeServer(server),
  };
}

function openAiProvider(): MockProvider {
  return {
    url: openAiDemoConfig.baseUrl,
    apiKey: openAiDemoConfig.apiKey ?? '',
    model: openAiDemoConfig.model,
    calls: () => [],
    close: async () => {},
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
    req.headers.authorization === `Bearer ${proxyDemoConfig.providerSecret}`
  );
}

function findProviderReply(userMessage: string): string {
  const breakCase = chatBreakCases.find((candidate) => candidate.userMessage === userMessage);
  return breakCase ? mockProviderReplyFor(breakCase) : "I don't have a scripted answer for that prompt.";
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

export async function readJsonRequest(req: IncomingMessage): Promise<JsonValue> {
  const chunks: Buffer[] = [];
  for await (const chunk of req) {
    chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk));
  }

  const text = Buffer.concat(chunks).toString('utf8');
  return text ? (JSON.parse(text) as JsonValue) : {};
}

function extractLastUserMessage(body: JsonValue): string {
  if (!isRecord(body) || !Array.isArray(body.messages)) return '';

  const messages = [...body.messages].reverse();
  for (const message of messages) {
    if (isRecord(message) && message.role === 'user' && typeof message.content === 'string') {
      return message.content;
    }
  }

  return '';
}

export function isRecord(value: JsonValue): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

export async function listenOnRandomLocalPort(
  server: ReturnType<typeof createServer>,
): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', () => {
      server.off('error', reject);
      resolve();
    });
  });
}

export async function closeServer(server: ReturnType<typeof createServer>): Promise<void> {
  await new Promise<void>((resolve, reject) => {
    server.close((error) => {
      if (error) reject(error);
      else resolve();
    });
  });
}
