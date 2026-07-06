import OpenAI from 'openai';
import type { ChatCompletionMessageParam, ChatCompletionTool } from 'openai/resources/chat/completions';

import { OPENAI_API_KEY, OPENAI_MODEL } from '../shared/env';
import { executeRefundTool, formatMoney, prepareRefundTool, searchOrderTool, type RefundAgentClient } from './core';
import {
  DEMO_ORDER_ID,
  type AgentRunResult,
  type PrepareRefundInput,
  type ToolTrace,
} from './types';

const SYSTEM_PROMPT = [
  'You are a refund support agent.',
  'Use search_order before preparing a refund.',
  'Use prepare_refund before execute_refund.',
  'Only execute refunds through TrustLoopGuard.',
  'Never ask for or mention Stripe secret keys.',
].join(' ');

const tools: ChatCompletionTool[] = [
  {
    type: 'function',
    function: {
      name: 'search_order',
      description: 'Search for a customer order before deciding whether a refund is eligible.',
      parameters: {
        type: 'object',
        additionalProperties: false,
        properties: {
          orderId: { type: 'string' },
          email: { type: 'string' },
          last4: { type: 'string' },
        },
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'prepare_refund',
      description: 'Create a typed TrustLoopGuard financial authorization for a refund.',
      parameters: {
        type: 'object',
        additionalProperties: false,
        properties: {
          orderId: { type: 'string' },
          amountMinor: { type: 'integer' },
          reason: { type: 'string' },
        },
        required: ['orderId', 'amountMinor', 'reason'],
      },
    },
  },
  {
    type: 'function',
    function: {
      name: 'execute_refund',
      description: 'Execute an already-authorized TrustLoopGuard refund action.',
      parameters: {
        type: 'object',
        additionalProperties: false,
        properties: {
          actionId: { type: 'string' },
        },
        required: ['actionId'],
      },
    },
  },
];

export async function runRefundAgent(
  prompt: string,
  client: RefundAgentClient,
  options: { useOpenAI?: boolean } = {},
): Promise<AgentRunResult> {
  if (options.useOpenAI !== false && OPENAI_API_KEY !== undefined && OPENAI_API_KEY.trim() !== '') {
    try {
      return await runOpenAiAgent(prompt, client);
    } catch {
      return runOfflineAgent(prompt, client);
    }
  }
  return runOfflineAgent(prompt, client);
}

async function runOpenAiAgent(
  prompt: string,
  client: RefundAgentClient,
): Promise<AgentRunResult> {
  const openai = new OpenAI({ apiKey: OPENAI_API_KEY });
  const traces: ToolTrace[] = [];
  const messages: ChatCompletionMessageParam[] = [
    { role: 'system', content: SYSTEM_PROMPT },
    { role: 'user', content: prompt },
  ];
  let actionId: string | undefined;
  let receiptId: string | undefined;
  let finalMessage = '';

  for (let step = 0; step < 6; step += 1) {
    const response = await openai.chat.completions.create({
      model: OPENAI_MODEL,
      messages,
      tools,
      tool_choice: 'auto',
    });
    const message = response.choices[0]?.message;
    if (message === undefined) break;
    messages.push(message);

    if (message.tool_calls === undefined || message.tool_calls.length === 0) {
      finalMessage = message.content?.trim() || finalSummary(actionId, receiptId, traces);
      break;
    }

    for (const call of message.tool_calls) {
      if (call.type !== 'function') continue;
      const result = await runToolCall(call.function.name, call.function.arguments, client);
      traces.push(result.trace);
      if (result.actionId !== undefined) actionId = result.actionId;
      if (result.receiptId !== undefined) receiptId = result.receiptId;
      messages.push({
        role: 'tool',
        tool_call_id: call.id,
        content: JSON.stringify(result.toolResult),
      });
    }
  }

  return {
    prompt,
    traces,
    finalMessage: finalMessage || finalSummary(actionId, receiptId, traces),
    actionId,
    receiptId,
  };
}

async function runOfflineAgent(
  prompt: string,
  client: RefundAgentClient,
): Promise<AgentRunResult> {
  const input = refundInputFromPrompt(prompt);
  const traces: ToolTrace[] = [];

  const search = searchOrderTool({ orderId: input.orderId });
  traces.push({
    tool: 'search_order',
    summary: search.found
      ? `found ${input.orderId} with ${formatMoney(search.order?.refundableBalanceMinor ?? 0)} refundable`
      : `did not find ${input.orderId}`,
  });
  if (!search.found) {
    return {
      prompt,
      traces,
      finalMessage: `I could not find ${input.orderId}, so I did not prepare a refund.`,
    };
  }

  const prepared = await prepareRefundTool(input, client);
  traces.push({
    tool: 'prepare_refund',
    summary: `${prepared.status}: ${prepared.message}`,
  });

  if (prepared.status !== 'authorized' && prepared.status !== 'proposed') {
    return {
      prompt,
      traces,
      finalMessage: `TrustLoopGuard returned ${prepared.status}. I did not create a Stripe refund.`,
      actionId: prepared.action.id,
    };
  }

  const executed = await executeRefundTool(prepared.action.id, client);
  traces.push({
    tool: 'execute_refund',
    summary: `${executed.status}: ${executed.message}`,
  });

  return {
    prompt,
    traces,
    finalMessage:
      executed.status === 'executed'
        ? `Refund executed. Receipt ${executed.receipt?.id ?? executed.action.id} is ready.`
        : `Refund was not executed because the action is ${executed.status}.`,
    actionId: executed.action.id,
    receiptId: executed.receipt?.id,
  };
}

async function runToolCall(
  name: string,
  rawArguments: string,
  client: RefundAgentClient,
): Promise<{
  trace: ToolTrace;
  toolResult: object;
  actionId?: string;
  receiptId?: string;
}> {
  if (name === 'search_order') {
    const args = JSON.parse(rawArguments) as { orderId?: string; email?: string; last4?: string };
    const result = searchOrderTool(args);
    return {
      trace: {
        tool: 'search_order',
        summary: result.found ? `found ${result.order?.id}` : 'order not found',
      },
      toolResult: result,
    };
  }

  if (name === 'prepare_refund') {
    const args = JSON.parse(rawArguments) as PrepareRefundInput;
    const result = await prepareRefundTool(args, client);
    return {
      trace: {
        tool: 'prepare_refund',
        summary: `${result.status}: ${result.message}`,
      },
      toolResult: {
        action_id: result.action.id,
        status: result.status,
        message: result.message,
      },
      actionId: result.action.id,
    };
  }

  if (name === 'execute_refund') {
    const args = JSON.parse(rawArguments) as { actionId: string };
    const result = await executeRefundTool(args.actionId, client);
    return {
      trace: {
        tool: 'execute_refund',
        summary: `${result.status}: ${result.message}`,
      },
      toolResult: {
        action_id: result.action.id,
        status: result.status,
        receipt_id: result.receipt?.id,
        message: result.message,
      },
      actionId: result.action.id,
      receiptId: result.receipt?.id,
    };
  }

  return {
    trace: { tool: 'search_order', summary: `unknown tool ignored: ${name}` },
    toolResult: { error: `unknown tool: ${name}` },
  };
}

function refundInputFromPrompt(prompt: string): PrepareRefundInput {
  const orderId = prompt.match(/ord_[a-z0-9_]+/i)?.[0] ?? DEMO_ORDER_ID;
  const amountMinor = amountMinorFromPrompt(prompt);
  const reason = prompt.toLowerCase().includes('damaged') ? 'damaged_item' : 'customer_request';
  return { orderId, amountMinor, reason };
}

function amountMinorFromPrompt(prompt: string): number {
  const match = prompt.match(/\$\s*(\d+(?:\.\d{1,2})?)/) ?? prompt.match(/\bfor\s+(\d+(?:\.\d{1,2})?)\b/i);
  if (match?.[1] === undefined) return 7_500;
  return Math.round(Number.parseFloat(match[1]) * 100);
}

function finalSummary(
  actionId: string | undefined,
  receiptId: string | undefined,
  traces: ToolTrace[],
): string {
  const last = traces.at(-1)?.summary ?? 'no refund tool ran';
  if (receiptId !== undefined) return `Refund executed. Receipt ${receiptId} is ready.`;
  if (actionId !== undefined) return `Refund action ${actionId}: ${last}`;
  return last;
}
