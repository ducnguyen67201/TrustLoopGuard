import OpenAI from 'openai';
import type { AuthorizationDecision, Client, GuardEvent, Source } from '@featherlane-ai/sdk';

import { createClient, DEFAULT_AGENT_ID } from '../shared/env';

export interface RefundRequest {
  /**
   * Integer minor units (cents). The demo standardizes on cents so money is
   * always a whole integer — the unit the `value_limit` cap is declared in —
   * and a non-integer amount is a genuine anomaly the guard escalates.
   */
  amount: number;
  account: string;
  reason: string;
}

const CENTS_PER_DOLLAR = 100;

/** Whole or 2-decimal dollars to integer cents. */
function toCents(dollars: number): number {
  return Math.round(dollars * CENTS_PER_DOLLAR);
}

/** Integer cents to a human "$X.XX" string. */
export function formatUsd(cents: number): string {
  return `$${(cents / CENTS_PER_DOLLAR).toFixed(2)}`;
}

export interface AgentTurn {
  reply: string;
  refund: RefundRequest | null;
}

export interface AgentChatResult {
  content: string;
  finishReason: 'stop' | 'content_filter';
  effect: 'deny' | 'transform' | 'require_approval' | 'defer' | null;
  phase: 'output' | null;
  traceId: string | null;
}

interface NorthPayAgentOptions {
  agentId?: string;
  sessionId?: string;
}

const SYSTEM_PROMPT = [
  'You are NorthPay Disputes, a payment-dispute support agent.',
  'If the customer asks for a refund to a destination account, call issue_refund.',
  'Otherwise reply briefly and ask for normal dispute verification.',
].join(' ');

const DEFAULT_SESSION_ID = process.env.DISPUTE_SESSION_ID?.trim() || 'northpay-dispute-local';

const source: Source = {
  id: 'conversation',
  origin: 'user',
  labels: { trust: 'untrusted', confidentiality: 'unknown', integrity: 'unknown' },
};

const refundTool = {
  type: 'function',
  function: {
    name: 'issue_refund',
    description: 'Issue a payment-dispute refund to a destination account.',
    parameters: {
      type: 'object',
      additionalProperties: false,
      properties: {
        amount: { type: 'number' },
        account: { type: 'string' },
        reason: { type: 'string' },
      },
      required: ['amount', 'account', 'reason'],
    },
  },
} as const;

export async function runDisputeAgent(
  message: string,
  options: { useOpenAI?: boolean } = {},
): Promise<AgentTurn> {
  const apiKey = process.env.OPENAI_API_KEY;
  if (options.useOpenAI !== false && apiKey !== undefined && apiKey.trim() !== '') {
    try {
      const openai = new OpenAI({
        apiKey,
        baseURL: process.env.OPENAI_BASE_URL ?? 'https://api.openai.com/v1',
      });
      const response = await openai.chat.completions.create({
        model: process.env.OPENAI_MODEL ?? 'gpt-4o-mini',
        messages: [
          { role: 'system', content: SYSTEM_PROMPT },
          { role: 'user', content: message },
        ],
        tools: [refundTool],
        tool_choice: 'auto',
      });
      const assistant = response.choices[0]?.message;
      const toolCall = assistant?.tool_calls?.find(
        (call) => call.type === 'function' && call.function.name === 'issue_refund',
      );
      const refund = refundFromToolCall(
        toolCall?.type === 'function' ? toolCall.function.arguments : undefined,
      );
      if (refund !== null) return refundTurn(refund);
      const reply = assistant?.content?.trim();
      if (reply !== undefined && reply !== '') return { reply, refund: null };
    } catch {
      // Offline demo path below.
    }
  }

  const refund = refundFromText(message);
  return refund === null
    ? {
        reply:
          'I can help with that dispute. First, can you confirm the last 4 digits of the card on file?',
        refund: null,
      }
    : refundTurn(refund);
}

export function issueRefund(ledger: RefundRequest[], refund: RefundRequest): string {
  ledger.push(refund);
  return `Refunded ${formatUsd(refund.amount)} to ${refund.account}.`;
}

export function createNorthPayDisputeAgent(options: NorthPayAgentOptions = {}) {
  const agentId = options.agentId ?? DEFAULT_AGENT_ID;
  const defaultSessionId = options.sessionId?.trim() || DEFAULT_SESSION_ID;
  const rawLedger: RefundRequest[] = [];
  const guardedLedger: RefundRequest[] = [];
  const guardedRunIds = new Map<string, string>();

  async function rawChat(message: string): Promise<AgentChatResult> {
    const turn = await runDisputeAgent(message);
    if (turn.refund !== null) issueRefund(rawLedger, turn.refund);
    return ok(turn.reply);
  }

  async function guardedChat(
    message: string,
    sessionId: string | undefined,
  ): Promise<AgentChatResult> {
    const client = createClient();
    const inputSummary = message.slice(0, 500);
    const externalId = sessionId?.trim() || defaultSessionId;
    let runId: string | null = null;

    try {
      runId = await ensureGuardedRun(client, externalId, inputSummary);
      await client.createRunEvent(runId, {
        kind: 'user_turn',
        label: 'user_message',
        input_summary: inputSummary,
        metadata: {},
      });

      const turn = await runDisputeAgent(message);
      const refund = turn.refund;
      const assistantEvent = await client.createRunEvent(runId, {
        kind: 'assistant_turn',
        label: 'agent_reply',
        input_summary: inputSummary,
        output_summary: turn.reply.slice(0, 500),
        metadata: {},
      });

      const outputDecision = await client.submitEvent(
        outputEvent(agentId, message, turn.reply, runId, assistantEvent.id),
      );
      if (
        outputDecision.effect === 'deny' ||
        outputDecision.effect === 'require_approval' ||
        outputDecision.effect === 'defer'
      ) {
        await client.finishRun(runId);
        return blocked(outputDecision, "I can't help with that request.");
      }

      const reply = typeof outputDecision.transformed_value === 'string'
        ? outputDecision.transformed_value
        : turn.reply;
      if (refund === null) {
        await client.finishRun(runId);
        return ok(reply, outputDecision.trace_id);
      }

      const toolEvent = await client.createRunEvent(runId, {
        kind: 'tool_call',
        label: 'issue_refund',
        input_summary: inputSummary,
        output_summary: `issue_refund ${formatUsd(refund.amount)} to ${refund.account}`,
        metadata: {},
      });
      const decision = await client.submitEvent(refundEvent(agentId, refund, runId, toolEvent.id));
      await client.finishRun(runId);

      if (decision.effect !== 'permit') return blocked(decision);
      issueRefund(guardedLedger, refund);
      return ok(reply, decision.trace_id);
    } catch {
      if (runId !== null) {
        try {
          await client.finishRun(runId, 'failed');
        } catch {
          // Keep the chat fallback below as the user-visible failure.
        }
      }
      return {
        content: "I can't process a refund right now - verification is unavailable.",
        finishReason: 'content_filter',
        effect: 'defer',
        phase: 'output',
        traceId: null,
      };
    }
  }

  async function ensureGuardedRun(
    client: Client,
    externalId: string,
    inputSummary: string,
  ): Promise<string> {
    const cached = guardedRunIds.get(externalId);
    if (cached !== undefined) return cached;

    const run = await client.startRun({
      agent_id: agentId,
      kind: 'chat_session',
      external_id: externalId,
      metadata: { product: 'NorthPay Disputes', input_summary: inputSummary },
    });
    guardedRunIds.set(externalId, run.id);
    return run.id;
  }

  return { rawChat, guardedChat, rawLedger, guardedLedger };
}

function refundTurn(refund: RefundRequest): AgentTurn {
  return {
    reply: `All set - I can issue a ${formatUsd(refund.amount)} refund to ${refund.account}.`,
    refund,
  };
}

function refundFromToolCall(raw: string | undefined): RefundRequest | null {
  if (raw === undefined) return null;
  try {
    return coerceRefund(JSON.parse(raw));
  } catch {
    return null;
  }
}

function refundFromText(message: string): RefundRequest | null {
  const embedded = message.match(/\{[\s\S]*\}/)?.[0];
  if (embedded !== undefined) {
    try {
      const refund = coerceRefund(JSON.parse(embedded));
      if (refund !== null) return refund;
    } catch {
      // Fall through to plain text extraction.
    }
  }

  const account = message.match(/\baccount\s+([A-Z0-9][\w@.-]+)/i)?.[1];
  if (account === undefined) return null;
  const rawAmount = message.match(/\$\s?([0-9][\d,]*(?:\.\d{1,2})?)(?![\d.])/)?.[1];
  if (rawAmount === undefined) return null;
  const parsed = Number(rawAmount.replace(/,/g, ''));
  if (!Number.isFinite(parsed) || parsed <= 0) return null;
  return {
    amount: toCents(parsed),
    account,
    reason: 'customer requested dispute refund',
  };
}

function coerceRefund(value: unknown): RefundRequest | null {
  if (value === null || typeof value !== 'object') return null;
  const obj = value as Record<string, unknown>;
  const amount = typeof obj.amount === 'number' ? obj.amount : Number(obj.amount);
  const account = typeof obj.account === 'string' ? obj.account.trim() : '';
  const reason = typeof obj.reason === 'string' ? obj.reason : 'customer requested dispute refund';
  if (!Number.isFinite(amount) || amount <= 0 || account === '') return null;
  return { amount: toCents(amount), account, reason };
}

function outputEvent(
  agentId: string,
  message: string,
  reply: string,
  runId: string,
  runEventId: string,
): GuardEvent {
  return {
    kind: 'output.proposed',
    principal: {
      workspace_id: '',
      environment_id: '',
      agent_id: agentId,
      run_id: runId,
      run_event_id: runEventId,
    },
    action: {
      operation: 'output',
      parameters: { text: reply },
      side_effect: 'none',
    },
    sources: [source],
    provenance: { text: ['conversation'] },
    context: {
      channel: 'chat',
      domain: 'customer_support',
      product: 'NorthPay Disputes',
      input_summary: message.slice(0, 500),
    },
  };
}

function refundEvent(
  agentId: string,
  refund: RefundRequest,
  runId: string,
  runEventId: string,
): GuardEvent {
  return {
    kind: 'tool.call.proposed',
    principal: {
      workspace_id: '',
      environment_id: '',
      agent_id: agentId,
      run_id: runId,
      run_event_id: runEventId,
    },
    action: {
      operation: 'issue_refund',
      parameters: { amount: refund.amount, account: refund.account, reason: refund.reason },
      side_effect: 'api_mutation',
    },
    sources: [source],
    provenance: { amount: ['conversation'], account: ['conversation'] },
    context: { channel: 'chat', domain: 'customer_support', product: 'NorthPay Disputes' },
  };
}

function ok(content: string, traceId: string | null = null): AgentChatResult {
  return { content, finishReason: 'stop', effect: null, phase: null, traceId };
}

function blocked(
  decision: AuthorizationDecision,
  fallback = "I've opened your dispute for review, but I can't send money to an account from chat.",
): AgentChatResult {
  return {
    content: typeof decision.transformed_value === 'string' ? decision.transformed_value : fallback,
    finishReason: 'content_filter',
    effect: decision.effect === 'permit' ? null : decision.effect,
    phase: 'output',
    traceId: decision.trace_id,
  };
}
