// The "short TrustLoopGuard we bolt on" — the ONLY file in the demo that knows
// TrustLoopGuard exists. It adapts the agent's generic ActionGuard to the
// runtime guard: it submits the proposed refund tool call to /v1/events and maps
// the verdict to allow/deny. The DisputeAgent runs identically without this.
import type { Decision, GuardEvent, Source } from '@trustloopguard/sdk';

import type { ActionGuard, AgentAction, GuardOutcome } from './agent';
import { CONVERSATION_SOURCE } from './scenario';

/** Minimal slice of the SDK Client this guard needs. Depending on the interface
 *  (not the concrete Client) lets the self-check pass a typed fake. */
export interface GuardClient {
  submitEvent(event: GuardEvent, signal?: AbortSignal): Promise<Decision>;
}

/** Wrap the agent's action boundary with TrustLoopGuard. Money-moving tool calls
 *  are submitted with parameter provenance; plain replies are submitted as
 *  output checks so every guarded turn has a TrustLoopGuard decision/trace. */
export function trustloopGuard(client: GuardClient, agentId: string): ActionGuard {
  return async (action: AgentAction): Promise<GuardOutcome> => {
    const decision = await client.submitEvent(
      action.kind === 'issue_refund'
        ? buildRefundEvent(action, agentId)
        : buildOutputEvent(action, agentId),
    );

    if (decision.verdict === 'allow') return { allow: true, traceId: decision.trace_id };
    if (decision.verdict === 'rewrite') {
      return {
        allow: true,
        reason: decision.reason,
        traceId: decision.trace_id,
        ...(decision.safe_output !== null ? { safeReply: decision.safe_output } : {}),
      };
    }

    return {
      allow: false,
      reason: decision.reason,
      traceId: decision.trace_id,
      ...(decision.safe_output !== null ? { safeReply: decision.safe_output } : {}),
    };
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
