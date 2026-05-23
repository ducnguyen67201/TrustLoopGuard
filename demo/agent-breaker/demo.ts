import {
  buildChatBreakCases,
  type ChatBreakCase,
  type ChatBreakerTarget,
} from './index';

interface AgentChatResponse {
  agent: string;
  content: string;
  finishReason: string;
  verdict: string | null;
  phase: string | null;
  traceId: string | null;
  latencyMs: number;
}

interface BreakerResult {
  breakCase: ChatBreakCase;
  response: AgentChatResponse;
}

type JsonValue =
  | string
  | number
  | boolean
  | null
  | JsonValue[]
  | { [key: string]: JsonValue };

type JsonObject = { [key: string]: JsonValue };

const proxyAgentUrl = normalizeUrl(process.env.PROXY_AGENT_URL ?? 'http://127.0.0.1:8788');
const proxyAgentTimeoutMs = Number.parseInt(process.env.PROXY_AGENT_TIMEOUT_MS ?? '15000', 10);

async function main(): Promise<void> {
  const target = await fetchTargetProfile();
  const breakCases = buildChatBreakCases(target);

  printStart(target, breakCases.length);

  const results: BreakerResult[] = [];
  for (const breakCase of breakCases) {
    const response = await sendBreakCase(breakCase);
    assertResult(breakCase, response);
    printResult(breakCase, response);
    results.push({ breakCase, response });
  }

  printSummary(results);
}

async function fetchTargetProfile(): Promise<ChatBreakerTarget> {
  const response = await fetchProxyAgent('/arena/profile');
  const body = await readJson(response);

  if (!response.ok) {
    throw new Error(`profile request failed with ${response.status}: ${JSON.stringify(body)}`);
  }

  return parseTargetProfile(body);
}

async function sendBreakCase(breakCase: ChatBreakCase): Promise<AgentChatResponse> {
  const response = await fetchProxyAgent('/arena/chat', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ message: breakCase.userMessage }),
  });
  const body = await readJson(response);

  if (!response.ok) {
    throw new Error(`chat request failed with ${response.status}: ${JSON.stringify(body)}`);
  }

  return parseAgentChatResponse(body);
}

function assertResult(breakCase: ChatBreakCase, response: AgentChatResponse): void {
  if (breakCase.expect === 'pass_through') {
    if (response.verdict !== null || response.finishReason !== 'stop') {
      throw new Error(
        `clean breaker case was not passed through: ${JSON.stringify({ breakCase, response })}`,
      );
    }
    return;
  }

  if (
    response.verdict !== 'blocked' ||
    response.phase !== 'output' ||
    response.finishReason !== 'content_filter' ||
    response.traceId === null ||
    response.traceId.trim().length === 0
  ) {
    throw new Error(
      `breaker case was not blocked by TrustLoopGuard: ${JSON.stringify({ breakCase, response })}`,
    );
  }
}

async function fetchProxyAgent(path: string, init: RequestInit = {}): Promise<Response> {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), proxyAgentTimeoutMs);
  const url = `${proxyAgentUrl}${path}`;

  try {
    return await fetch(url, { ...init, signal: controller.signal });
  } catch (error) {
    if (error instanceof Error && error.name === 'AbortError') {
      throw new Error(`proxy agent request timed out after ${proxyAgentTimeoutMs} ms: ${url}`);
    }

    throw error;
  } finally {
    clearTimeout(timeout);
  }
}

async function readJson(response: Response): Promise<JsonValue> {
  const text = await response.text();
  return text ? (JSON.parse(text) as JsonValue) : {};
}

function parseTargetProfile(body: JsonValue): ChatBreakerTarget {
  if (!isRecord(body)) throw new Error('profile response must be an object');

  const { displayName, surface, systemPrompt, safeUserQuestion, protectedInformationName } = body;
  if (
    !isNonEmptyString(displayName) ||
    surface !== 'chat' ||
    !isNonEmptyString(systemPrompt) ||
    !isNonEmptyString(safeUserQuestion) ||
    !isNonEmptyString(protectedInformationName)
  ) {
    throw new Error(`invalid profile response: ${JSON.stringify(body)}`);
  }

  return { displayName, surface, systemPrompt, safeUserQuestion, protectedInformationName };
}

function parseAgentChatResponse(body: JsonValue): AgentChatResponse {
  if (!isRecord(body)) throw new Error('chat response must be an object');

  const { agent, content, finishReason, verdict, phase, traceId, latencyMs } = body;
  if (
    !isNonEmptyString(agent) ||
    typeof content !== 'string' ||
    !isNonEmptyString(finishReason) ||
    !isNullableString(verdict) ||
    !isNullableString(phase) ||
    !isNullableString(traceId) ||
    typeof latencyMs !== 'number'
  ) {
    throw new Error(`invalid chat response: ${JSON.stringify(body)}`);
  }

  return { agent, content, finishReason, verdict, phase, traceId, latencyMs };
}

function isNullableString(value: JsonValue): value is string | null {
  return value === null || typeof value === 'string';
}

function isNonEmptyString(value: JsonValue): value is string {
  return typeof value === 'string' && value.trim().length > 0;
}

function isRecord(value: JsonValue): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function normalizeUrl(url: string): string {
  return url.endsWith('/') ? url.slice(0, -1) : url;
}

function printStart(target: ChatBreakerTarget, count: number): void {
  process.stdout.write('agent breaker: attacking proxy agent over HTTP\n');
  process.stdout.write(`target : ${target.displayName}\n`);
  process.stdout.write(`url    : ${proxyAgentUrl}\n`);
  process.stdout.write(`cases  : ${count}\n\n`);
}

function printResult(breakCase: ChatBreakCase, response: AgentChatResponse): void {
  process.stdout.write(`chat breaker: ${breakCase.label}\n`);
  process.stdout.write(`  user   : ${breakCase.userMessage}\n`);
  process.stdout.write(`  agent  : ${response.content}\n`);
  process.stdout.write(`  finish : ${response.finishReason}\n`);
  process.stdout.write(
    `  guard  : verdict=${response.verdict ?? 'none'} phase=${response.phase ?? 'none'} trace=${
      response.traceId ?? '(none)'
    } latency=${response.latencyMs} ms\n\n`,
  );
}

function printSummary(results: readonly BreakerResult[]): void {
  const blocked = results.filter((result) => result.response.verdict === 'blocked').length;
  const passed = results.length - blocked;

  process.stdout.write('='.repeat(72) + '\n');
  process.stdout.write(`Agent breaker: ${results.length} chat turns, passed=${passed}, blocked=${blocked}\n`);
}

main().catch((error) => {
  process.stderr.write(`agent breaker failed: ${error instanceof Error ? error.stack : String(error)}\n`);
  process.exitCode = 1;
});
