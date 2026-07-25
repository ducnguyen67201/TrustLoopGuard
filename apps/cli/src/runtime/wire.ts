import type { HostId, JsonObject, JsonValue, RuntimeRegistration } from './runtime-types.js';

export type RuntimeHostId = HostId;
export type RuntimeEvent = 'post-failure' | 'post-success' | 'pre' | 'session-end';

export interface HostToolCall {
  host: RuntimeHostId;
  event: RuntimeEvent;
  toolName: string;
  callId: string;
  sessionId: string;
  cwd: string;
  projectRoot: string;
  input: JsonObject;
  hostVersion: string;
}

export interface GuardEvent {
  kind:
    | 'file.action.proposed'
    | 'network.request.proposed'
    | 'shell.action.proposed'
    | 'tool.call.proposed';
  principal: {
    workspace_id: string;
    environment_id: string;
    agent_id: string;
    session_id?: string;
  };
  action: {
    operation: string;
    parameters: JsonObject;
    side_effect: 'api_mutation' | 'file_write' | 'network_call' | 'read' | 'shell_exec';
    invocation_id: string;
    tool_identity: {
      server_id: 'claude-code' | 'codex' | 'opencode';
      tool_name: string;
      schema_hash: string;
    };
    authorization?: {
      grant_id: string;
      attempt_id: string;
    };
  };
  sources: Array<{
    id: string;
    origin: string;
    labels: {
      trust: string;
      confidentiality: string;
      integrity: string;
    };
  }>;
  provenance: Record<string, string[]>;
  context: {
    channel: 'claude-code' | 'codex' | 'opencode';
  };
}

export type DecisionEffect = 'defer' | 'deny' | 'permit' | 'require_approval' | 'transform';

export interface GuardDecision {
  effect: DecisionEffect;
  reason: string;
  traceId: string;
  approval?: {
    id: string;
    pollAfterMs: number;
  };
  lease?: {
    id: string;
  };
}

interface ApprovalStatus {
  status: 'approved' | 'canceled' | 'denied' | 'expired' | 'pending';
  grantId?: string;
}

export function parseCommandHookPayload(
  value: JsonValue,
  host: RuntimeHostId,
  fallbackCwd: string,
): HostToolCall {
  if (!isObject(value)) throw new Error('hook input must be a JSON object');
  const eventName = stringValue(value, ['hook_event_name', 'hookEventName']);
  const event = eventFromName(eventName);
  const inputValue = value['tool_input'] ?? value['toolInput'] ?? {};
  const input = isObject(inputValue) ? inputValue : {};
  const cwd = stringValue(value, ['cwd']) || fallbackCwd;
  return {
    host,
    event,
    toolName: stringValue(value, ['tool_name', 'toolName']) || 'unknown_tool',
    callId: stringValue(value, ['tool_use_id', 'call_id', 'callID']),
    sessionId: stringValue(value, ['session_id', 'sessionId']),
    cwd,
    projectRoot: stringValue(value, ['project_dir', 'projectDir']) || cwd,
    input,
    hostVersion: stringValue(value, ['host_version', 'hostVersion']),
  };
}

function eventFromName(value: string): RuntimeEvent {
  if (value === 'PreToolUse') return 'pre';
  if (value === 'PostToolUse') return 'post-success';
  if (value === 'PostToolUseFailure') return 'post-failure';
  if (value === 'Stop' || value === 'SessionEnd') return 'session-end';
  throw new Error(`unsupported hook event ${value || '(missing)'}`);
}

export function parseDecision(value: JsonValue): GuardDecision {
  if (!isObject(value)) throw new Error('guard returned a non-object decision');
  const effect = value['effect'];
  if (
    effect !== 'permit' &&
    effect !== 'deny' &&
    effect !== 'transform' &&
    effect !== 'require_approval' &&
    effect !== 'defer'
  ) {
    throw new Error('guard returned an unexpected decision effect');
  }
  const reason =
    typeof value['reason'] === 'string' ? value['reason'] : 'the guard returned no reason';
  const traceId = typeof value['trace_id'] === 'string' ? value['trace_id'] : 'n/a';
  const decision: GuardDecision = { effect, reason, traceId };
  const approval = value['approval'];
  if (isObject(approval) && typeof approval['id'] === 'string') {
    decision.approval = {
      id: approval['id'],
      pollAfterMs:
        typeof approval['poll_after_ms'] === 'number' && approval['poll_after_ms'] > 0
          ? approval['poll_after_ms']
          : 1_000,
    };
  }
  const lease = value['lease'];
  if (isObject(lease) && typeof lease['id'] === 'string') {
    decision.lease = { id: lease['id'] };
  }
  return decision;
}

export function parseApprovalStatus(value: JsonValue): ApprovalStatus {
  if (!isObject(value)) throw new Error('guard returned a non-object approval');
  const status = value['status'];
  if (
    status !== 'approved' &&
    status !== 'canceled' &&
    status !== 'denied' &&
    status !== 'expired' &&
    status !== 'pending'
  ) {
    throw new Error('guard returned an unexpected approval status');
  }
  const grantId = value['grant_id'];
  return typeof grantId === 'string' ? { status, grantId } : { status };
}

export function parseRuntimeRegistration(value: JsonValue, source: string): RuntimeRegistration {
  if (!isObject(value)) throw new Error(`${source} must be an object`);
  const targets = value['targets'];
  if (
    !Array.isArray(targets) ||
    targets.some((target) => target !== 'claude' && target !== 'codex' && target !== 'opencode')
  ) {
    throw new Error(`${source} has invalid targets`);
  }
  return {
    root: requiredString(value, 'root', source),
    url: requiredString(value, 'url', source),
    agentId: requiredString(value, 'agentId', source),
    targets: targets as HostId[],
    cliVersion: requiredString(value, 'cliVersion', source),
    runtimeVersion: requiredString(value, 'runtimeVersion', source),
    createdAt: requiredString(value, 'createdAt', source),
    updatedAt: requiredString(value, 'updatedAt', source),
  };
}

export function isObject(value: JsonValue | undefined): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}

function stringValue(value: JsonObject, keys: string[]): string {
  for (const key of keys) {
    const candidate = value[key];
    if (typeof candidate === 'string') return candidate;
  }
  return '';
}

function requiredString(value: JsonObject, key: string, source: string): string {
  const candidate = value[key];
  if (typeof candidate !== 'string' || candidate.trim() === '') {
    throw new Error(`${source} has invalid ${key}`);
  }
  return candidate;
}
