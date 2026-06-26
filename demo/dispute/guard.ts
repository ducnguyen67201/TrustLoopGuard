// The "short TrustLoopGuard we bolt on" — the ONLY file in the demo that knows
// TrustLoopGuard exists. It adapts the agent's generic ActionGuard to the
// runtime guard: it submits the proposed refund tool call to /v1/events and maps
// the verdict to allow/deny. The DisputeAgent runs identically without this.
import type { ActiveRun, Decision, GuardEvent, Source, WithRunOptions } from '@trustloopguard/sdk';

import type { ActionGuard, AgentAction, GuardOutcome } from './agent';
import { CONVERSATION_SOURCE } from './scenario';

/** Minimal slice of the SDK Client this guard needs. Depending on the interface
 *  (not the concrete Client) lets the self-check pass a typed fake. */
export interface GuardClient {
  submitEvent(event: GuardEvent, signal?: AbortSignal): Promise<Decision>;
  withRun<T>(opts: WithRunOptions, fn: (run: ActiveRun) => Promise<T>): Promise<T>;
}

export interface TrustloopGuardOptions {
  externalId?: string;
  inputSummary?: string;
  metadata?: Record<string, unknown>;
}

export interface RunEventInput {
  kind: 'user_turn' | 'assistant_turn' | 'tool_call' | 'workflow_step' | 'system_event' | 'other';
  label: string;
  input_summary?: string;
  output_summary?: string;
  metadata: Record<string, unknown>;
}

export interface TrustloopRunClient {
  submitEvent(event: GuardEvent, signal?: AbortSignal): Promise<Decision>;
  startRun(
    req: {
      agent_id: string;
      kind: 'chat_session';
      external_id?: string;
      metadata: Record<string, unknown>;
    },
    signal?: AbortSignal,
  ): Promise<{ id: string }>;
  createRunEvent(runId: string, req: RunEventInput, signal?: AbortSignal): Promise<{ id: string }>;
  finishRun(runId: string, status?: 'completed' | 'failed' | 'canceled'): Promise<unknown>;
}

export interface TrustloopGuardSession {
  id: string;
  event(req: RunEventInput): Promise<{ id: string }>;
  guard(inputSummary: string): ActionGuard;
  finish(status?: 'completed' | 'failed' | 'canceled'): Promise<void>;
}

/** Wrap the agent's action boundary with TrustLoopGuard. Money-moving tool calls
 *  are submitted with parameter provenance; plain replies are submitted as
 *  output checks so every guarded turn has a TrustLoopGuard decision/trace. */
export function trustloopGuard(
  client: GuardClient,
  agentId: string,
  options: TrustloopGuardOptions = {},
): ActionGuard {
  return async (action: AgentAction): Promise<GuardOutcome> => {
    return client.withRun(
      {
        agentId,
        kind: 'chat_session',
        externalId: options.externalId,
        inputSummary: options.inputSummary,
        metadata: { product: 'NorthPay Disputes', ...(options.metadata ?? {}) },
      },
      (run) =>
        run.withEvent(runEventFor(action, options.inputSummary), async () =>
          decisionToOutcome(
            await client.submitEvent(
              action.kind === 'issue_refund'
                ? buildRefundEvent(action, agentId)
                : buildOutputEvent(action, agentId),
            ),
          ),
        ),
    );
  };
}

export async function startTrustloopGuardSession(
  client: TrustloopRunClient,
  agentId: string,
  options: TrustloopGuardOptions = {},
): Promise<TrustloopGuardSession> {
  const run = await client.startRun({
    agent_id: agentId,
    kind: 'chat_session',
    external_id: options.externalId,
    metadata: { product: 'NorthPay Disputes', ...(options.metadata ?? {}) },
  });

  return {
    id: run.id,
    event(req) {
      return client.createRunEvent(run.id, req);
    },
    guard(inputSummary: string): ActionGuard {
      return async (action) => {
        const runEvent = await client.createRunEvent(run.id, runEventFor(action, inputSummary));
        const event =
          action.kind === 'issue_refund'
            ? buildRefundEvent(action, agentId)
            : buildOutputEvent(action, agentId);
        event.principal.run_id = run.id;
        event.principal.run_event_id = runEvent.id;
        return decisionToOutcome(await client.submitEvent(event));
      };
    },
    async finish(status = 'completed') {
      await client.finishRun(run.id, status);
    },
  };
}

function runEventFor(action: AgentAction, inputSummary?: string): RunEventInput {
  const input = inputSummary?.trim();
  if (action.kind === 'issue_refund') {
    return {
      kind: 'tool_call' as const,
      label: 'issue_refund',
      ...(input ? { input_summary: input } : {}),
      output_summary: action.message,
      metadata: {},
    };
  }
  return {
    kind: 'assistant_turn' as const,
    label: action.kind,
    ...(input ? { input_summary: input } : {}),
    output_summary: action.message,
    metadata: {},
  };
}

function decisionToOutcome(decision: Decision): GuardOutcome {
  if (decision.verdict === 'allow') {
    return { allow: true, verdict: decision.verdict, traceId: decision.trace_id };
  }
  if (decision.verdict === 'rewrite') {
    return {
      allow: true,
      verdict: decision.verdict,
      reason: decision.reason,
      traceId: decision.trace_id,
      ...(decision.safe_output !== null ? { safeReply: decision.safe_output } : {}),
    };
  }

  return {
    allow: false,
    verdict: decision.verdict,
    reason: decision.reason,
    traceId: decision.trace_id,
    ...(decision.safe_output !== null ? { safeReply: decision.safe_output } : {}),
  };
}

export function buildOutputEvent(
  action: Exclude<AgentAction, { kind: 'issue_refund' }>,
  agentId: string,
): GuardEvent {
  const sources: Source[] = [CONVERSATION_SOURCE];
  return {
    kind: 'output.proposed',
    principal: { workspace_id: '', environment_id: '', agent_id: agentId },
    action: {
      operation: 'output',
      parameters: { text: action.message, action: action.kind },
      side_effect: 'none',
    },
    sources,
    provenance: { text: [CONVERSATION_SOURCE.id] },
    context: {
      channel: 'chat',
      domain: 'customer_support',
      product: 'NorthPay Disputes',
      proposed_action: action.kind,
    },
  };
}

/** Build the tool-call event for a proposed refund. The `account` parameter's
 *  provenance points at the untrusted conversation source — what the engine's
 *  parameter-authorization checker blocks on, given `issue_refund`'s `account`
 *  is registered as authority-bearing (see `dispute:setup`). */
export function buildRefundEvent(
  action: Extract<AgentAction, { kind: 'issue_refund' }>,
  agentId: string,
): GuardEvent {
  const sources: Source[] = [CONVERSATION_SOURCE];
  return {
    kind: 'tool.call.proposed',
    principal: { workspace_id: '', environment_id: '', agent_id: agentId },
    action: {
      operation: 'issue_refund',
      parameters: { amount: action.amount, account: action.account },
      side_effect: 'api_mutation',
    },
    sources,
    provenance: { account: [CONVERSATION_SOURCE.id], amount: [CONVERSATION_SOURCE.id] },
    context: { channel: 'chat', domain: 'customer_support', product: 'NorthPay Disputes' },
  };
}
