import { createHash, randomUUID } from 'node:crypto';
import { readFile, realpath } from 'node:fs/promises';
import { isAbsolute, join, relative } from 'node:path';

import type {
  HostId,
  JsonObject,
  JsonValue,
  RuntimeEnvironment,
  RuntimeRegistration,
} from './runtime-types.js';
import {
  claimLeaseState,
  claimSessionLeaseStates,
  finishClaim,
  releaseClaim,
  storeLeaseState,
  type ClaimedLease,
} from './state.js';
import {
  isObject,
  parseApprovalStatus,
  parseDecision,
  parseRuntimeRegistration,
  type GuardDecision,
  type GuardEvent,
  type HostToolCall,
} from './wire.js';

const REQUEST_TIMEOUT_MS = 3_000;
const APPROVAL_TIMEOUT_MS = 300_000;
const APPROVAL_POLL_MS = 1_000;
const MAX_TOOL_INPUT_BYTES = 1_000_000;
const RUNTIME_SCHEMA_VERSION = 'featherlane-ai-tool-gate-v1';

const FILE_TOOLS = new Set(['Write', 'Edit', 'NotebookEdit', 'apply_patch', 'write', 'edit']);
const READ_TOOLS = new Set(['Read', 'Glob', 'Grep', 'read', 'glob', 'grep', 'read_file']);
const NETWORK_TOOLS = new Set([
  'WebFetch',
  'WebSearch',
  'webfetch',
  'websearch',
  'web_fetch',
  'web_search',
]);
const SHELL_TOOLS = new Set(['Bash', 'bash', 'shell', 'shell_command', 'exec_command']);

export interface BridgeOptions {
  configRoot: string;
  env: RuntimeEnvironment;
  fetchImpl?: typeof fetch;
  now?: () => number;
  sleep?: (milliseconds: number) => Promise<void>;
}

interface AuthorizationResult {
  managed: boolean;
  allowed: boolean;
  reason: string;
}

interface CompletionResult {
  completed: number;
  retained: number;
  errors: string[];
}

export async function authorizeToolCall(
  call: HostToolCall,
  options: BridgeOptions,
): Promise<AuthorizationResult> {
  let registration: RuntimeRegistration | undefined;
  try {
    registration = await registrationForCall(call, options.configRoot);
  } catch (error) {
    return {
      managed: true,
      allowed: false,
      reason: `Featherlane AI could not authorize this tool: ${
        error instanceof Error ? error.message : 'project registry unavailable'
      }.`,
    };
  }
  if (registration === undefined) return { managed: false, allowed: false, reason: '' };
  try {
    if (call.callId === '') throw new Error('tool-use id is missing');
    const apiKey = options.env.FEATHERLANE_AI_API_KEY?.trim();
    if (!apiKey) throw new Error('FEATHERLANE_AI_API_KEY is not set in the host environment');
    const event = buildGuardEvent(call, registration);
    if (Buffer.byteLength(JSON.stringify(event.action.parameters)) > MAX_TOOL_INPUT_BYTES) {
      throw new Error('tool input exceeds the 1 MB gate limit');
    }
    let decision = await submitEvent(event, registration.url, apiKey, options);
    let resumedAfterApproval = false;
    if (decision.effect === 'require_approval') {
      if (decision.approval === undefined) throw new Error('approval decision is missing its id');
      const grantId = await awaitApproval(
        registration.url,
        apiKey,
        decision.approval.id,
        decision.approval.pollAfterMs,
        options,
      );
      event.action.authorization = { grant_id: grantId, attempt_id: randomUUID() };
      decision = await submitEvent(event, registration.url, apiKey, options);
      resumedAfterApproval = true;
    }
    if (decision.effect !== 'permit') {
      return { managed: true, allowed: false, reason: describeDecision(decision) };
    }
    if (resumedAfterApproval && decision.lease === undefined) {
      throw new Error('approved execution did not return a lease');
    }
    if (decision.lease !== undefined) {
      await storeLeaseState(join(options.configRoot, 'state'), {
        leaseId: decision.lease.id,
        host: call.host,
        sessionId: call.sessionId,
        callId: call.callId,
        url: registration.url,
      });
    }
    return { managed: true, allowed: true, reason: describeDecision(decision) };
  } catch (error) {
    const message = error instanceof Error ? error.message : 'guard unavailable';
    return {
      managed: true,
      allowed: false,
      reason: `Featherlane AI could not authorize this tool: ${message}.`,
    };
  }
}

export async function completeToolCall(
  call: HostToolCall,
  status: 'canceled' | 'consumed',
  options: BridgeOptions,
): Promise<CompletionResult> {
  if (call.callId === '') return { completed: 0, retained: 0, errors: [] };
  const claim = await claimLeaseState(
    join(options.configRoot, 'state'),
    call.host,
    call.sessionId,
    call.callId,
  );
  if (claim === undefined) return { completed: 0, retained: 0, errors: [] };
  return completeClaims([claim], status, call.event, options);
}

export async function cancelSessionLeases(
  call: HostToolCall,
  options: BridgeOptions,
): Promise<CompletionResult> {
  if (call.sessionId === '') return { completed: 0, retained: 0, errors: [] };
  const claims = await claimSessionLeaseStates(
    join(options.configRoot, 'state'),
    call.host,
    call.sessionId,
  );
  return completeClaims(claims, 'canceled', call.event, options);
}

export async function isManagedProject(
  host: HostId,
  cwd: string,
  configRoot: string,
): Promise<boolean> {
  return (await findRegistration(configRoot, cwd, host)) !== undefined;
}

export function buildGuardEvent(call: HostToolCall, registration: RuntimeRegistration): GuardEvent {
  const kind = SHELL_TOOLS.has(call.toolName)
    ? 'shell.action.proposed'
    : FILE_TOOLS.has(call.toolName)
      ? 'file.action.proposed'
      : NETWORK_TOOLS.has(call.toolName)
        ? 'network.request.proposed'
        : 'tool.call.proposed';
  const sideEffect = SHELL_TOOLS.has(call.toolName)
    ? 'shell_exec'
    : FILE_TOOLS.has(call.toolName)
      ? 'file_write'
      : READ_TOOLS.has(call.toolName)
        ? 'read'
        : NETWORK_TOOLS.has(call.toolName)
          ? 'network_call'
          : 'api_mutation';
  const parameters = SHELL_TOOLS.has(call.toolName)
    ? shellParameters(call)
    : sortObject(call.input);
  const normalizedFields = Object.keys(parameters).sort();
  const serverId =
    call.host === 'claude' ? 'claude-code' : call.host === 'codex' ? 'codex' : 'opencode';
  const source = {
    id: 'conversation',
    origin: 'user',
    labels: { trust: 'untrusted', confidentiality: 'unknown', integrity: 'unknown' },
  };
  return {
    kind,
    principal: {
      workspace_id: '',
      environment_id: '',
      agent_id: registration.agentId,
      ...(call.sessionId === '' ? {} : { session_id: call.sessionId }),
    },
    action: {
      operation: call.toolName,
      parameters,
      side_effect: sideEffect,
      invocation_id: call.callId,
      tool_identity: {
        server_id: serverId,
        tool_name: call.toolName,
        schema_hash: `sha256:${sha256(
          JSON.stringify([
            RUNTIME_SCHEMA_VERSION,
            call.host,
            call.hostVersion,
            call.toolName,
            kind,
            normalizedFields,
          ]),
        )}`,
      },
    },
    sources: [source],
    provenance: Object.fromEntries(Object.keys(parameters).map((key) => [key, [source.id]])),
    context: { channel: serverId },
  };
}

function shellParameters(call: HostToolCall): JsonObject {
  const commandValue =
    call.input['command'] ?? call.input['cmd'] ?? call.input['script'] ?? call.input['description'];
  const timeout = call.input['timeout'] ?? call.input['timeout_ms'];
  return {
    command: typeof commandValue === 'string' ? commandValue : '',
    shell: 'bash',
    cwd: call.cwd,
    workspace_root: call.projectRoot,
    ...(typeof timeout === 'number' && Number.isSafeInteger(timeout) && timeout > 0
      ? { timeout_ms: timeout }
      : {}),
    run_in_background:
      call.input['run_in_background'] === true || call.input['runInBackground'] === true,
  };
}

function sortObject(value: JsonObject): JsonObject {
  return Object.fromEntries(
    Object.entries(value).sort(([left], [right]) => left.localeCompare(right)),
  );
}

async function submitEvent(
  event: GuardEvent,
  baseUrl: string,
  apiKey: string,
  options: BridgeOptions,
): Promise<GuardDecision> {
  const value = await requestJson(
    baseUrl,
    '/v1/events',
    apiKey,
    { method: 'POST', body: JSON.stringify(event) },
    options,
  );
  return parseDecision(value);
}

async function awaitApproval(
  baseUrl: string,
  apiKey: string,
  approvalId: string,
  serverPollMs: number,
  options: BridgeOptions,
): Promise<string> {
  const now = options.now ?? Date.now;
  const sleep =
    options.sleep ??
    ((milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds)));
  const timeoutMs = positiveInt(options.env.FEATHERLANE_AI_APPROVAL_TIMEOUT_MS, APPROVAL_TIMEOUT_MS);
  const configuredPollMs = positiveInt(
    options.env.FEATHERLANE_AI_APPROVAL_POLL_MS,
    serverPollMs || APPROVAL_POLL_MS,
  );
  const deadline = now() + timeoutMs;
  while (now() < deadline) {
    const approval = parseApprovalStatus(
      await requestJson(
        baseUrl,
        `/v1/authorization/approvals/${encodeURIComponent(approvalId)}`,
        apiKey,
        { method: 'GET' },
        options,
      ),
    );
    if (approval.status === 'approved' && approval.grantId !== undefined) return approval.grantId;
    if (approval.status !== 'pending') throw new Error(`approval ${approval.status}`);
    await sleep(configuredPollMs);
  }
  throw new Error('approval timed out');
}

async function completeClaims(
  claims: ClaimedLease[],
  status: 'canceled' | 'consumed',
  hookEvent: string,
  options: BridgeOptions,
): Promise<CompletionResult> {
  const apiKey = options.env.FEATHERLANE_AI_API_KEY?.trim();
  const result: CompletionResult = { completed: 0, retained: 0, errors: [] };
  for (const claim of claims) {
    try {
      if (!apiKey) throw new Error('FEATHERLANE_AI_API_KEY is not set in the host environment');
      let lastError: Error | undefined;
      for (let attempt = 0; attempt < 3; attempt += 1) {
        try {
          await requestJson(
            claim.state.url,
            `/v1/authorization/leases/${encodeURIComponent(claim.state.leaseId)}/complete`,
            apiKey,
            {
              method: 'POST',
              body: JSON.stringify({ status, outcome: { hook_event_name: hookEvent } }),
            },
            options,
          );
          lastError = undefined;
          break;
        } catch (error) {
          lastError = error instanceof Error ? error : new Error('lease completion failed');
        }
      }
      if (lastError !== undefined) throw lastError;
      await finishClaim(claim);
      result.completed += 1;
    } catch (error) {
      await releaseClaim(claim);
      result.retained += 1;
      result.errors.push(error instanceof Error ? error.message : 'lease completion failed');
    }
  }
  return result;
}

async function requestJson(
  baseUrl: string,
  pathname: string,
  apiKey: string,
  init: RequestInit,
  options: BridgeOptions,
): Promise<JsonValue> {
  const fetchImpl = options.fetchImpl ?? fetch;
  const timeout = positiveInt(options.env.FEATHERLANE_AI_REQUEST_TIMEOUT_MS, REQUEST_TIMEOUT_MS);
  const response = await fetchImpl(`${baseUrl.replace(/\/$/, '')}${pathname}`, {
    ...init,
    headers: {
      'content-type': 'application/json',
      authorization: `Bearer ${apiKey}`,
      ...init.headers,
    },
    signal: AbortSignal.timeout(timeout),
  });
  if (!response.ok) throw new Error(`Featherlane AI returned HTTP ${response.status}`);
  const text = await response.text();
  if (Buffer.byteLength(text) > MAX_TOOL_INPUT_BYTES) {
    throw new Error('Featherlane AI returned an oversized response');
  }
  try {
    return JSON.parse(text) as JsonValue;
  } catch {
    throw new Error('Featherlane AI returned malformed JSON');
  }
}

async function registrationForCall(
  call: HostToolCall,
  configRoot: string,
): Promise<RuntimeRegistration | undefined> {
  return findRegistration(configRoot, call.cwd, call.host);
}

async function findRegistration(
  configRoot: string,
  cwd: string,
  host: HostId,
): Promise<RuntimeRegistration | undefined> {
  let raw: JsonValue;
  try {
    raw = JSON.parse(await readFile(join(configRoot, 'registry.json'), 'utf8')) as JsonValue;
  } catch (error) {
    if (error instanceof Error && 'code' in error && error.code === 'ENOENT') return undefined;
    throw new Error('Featherlane AI project registry is unreadable');
  }
  if (!isObject(raw) || raw['version'] !== 1 || !Array.isArray(raw['projects'])) {
    throw new Error('Featherlane AI project registry is malformed');
  }
  let candidatePath = cwd;
  try {
    candidatePath = await realpath(cwd);
  } catch {
    // Missing post-hook paths still use their original absolute value for lease reconciliation.
  }
  return raw['projects']
    .map((project, index) => parseRuntimeRegistration(project, `registry project ${index}`))
    .filter(
      (project) => project.targets.includes(host) && containsPath(project.root, candidatePath),
    )
    .sort((left, right) => right.root.length - left.root.length)[0];
}

function containsPath(root: string, candidate: string): boolean {
  const child = relative(root, candidate);
  return (
    child === '' ||
    (child !== '..' && !child.startsWith(`..${pathSeparator(child)}`) && !isAbsolute(child))
  );
}

function pathSeparator(value: string): '/' | '\\' {
  return value.includes('\\') ? '\\' : '/';
}

function positiveInt(value: string | undefined, fallback: number): number {
  const parsed = Number.parseInt(value ?? '', 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function sha256(value: string): string {
  return createHash('sha256').update(value).digest('hex');
}

function describeDecision(decision: GuardDecision): string {
  return `Featherlane AI ${decision.effect}: ${decision.reason} (trace ${decision.traceId})`;
}
