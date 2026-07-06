import type { ChatCompletionTool } from 'openai/resources/chat/completions';

import {
  executeRefundTool,
  formatMoney,
  prepareRefundTool,
  searchOrderTool,
  type RefundAgentClient,
} from './core';
import type { PrepareRefundInput, ToolTrace } from './types';

export interface ToolRunResult {
  trace: ToolTrace;
  toolResult: object;
  actionId?: string;
  receiptId?: string;
}

export const refundAgentTools: ChatCompletionTool[] = [
  {
    type: 'function',
    function: {
      name: 'search_order',
      description: 'Search the SQLite order backend before deciding whether a refund is eligible.',
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

export async function runRefundTool(
  name: string,
  rawArguments: string,
  client: RefundAgentClient,
): Promise<ToolRunResult> {
  if (name === 'search_order') return runSearchOrder(rawArguments);
  if (name === 'prepare_refund') return runPrepareRefund(rawArguments, client);
  if (name === 'execute_refund') return runExecuteRefund(rawArguments, client);
  return {
    trace: { tool: 'search_order', summary: `unknown tool ignored: ${name}` },
    toolResult: { error: `unknown tool: ${name}` },
  };
}

export function searchTrace(orderId: string): ToolTrace {
  const search = searchOrderTool({ orderId });
  return {
    tool: 'search_order',
    summary: search.found
      ? `found ${orderId} with ${formatMoney(search.order?.refundableBalanceMinor ?? 0)} refundable`
      : `did not find ${orderId}`,
  };
}

async function runSearchOrder(rawArguments: string): Promise<ToolRunResult> {
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

async function runPrepareRefund(
  rawArguments: string,
  client: RefundAgentClient,
): Promise<ToolRunResult> {
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

async function runExecuteRefund(
  rawArguments: string,
  client: RefundAgentClient,
): Promise<ToolRunResult> {
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
