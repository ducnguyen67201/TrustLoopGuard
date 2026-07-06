import { executeRefundTool, prepareRefundTool, searchOrderTool, type RefundAgentClient } from './core';
import { refundInputFromPrompt } from './prompt-parser';
import { searchTrace } from './tool-runner';
import type { AgentRunResult, ToolTrace } from './types';

export async function runScriptedRefundAgent(
  prompt: string,
  client: RefundAgentClient,
): Promise<AgentRunResult> {
  const input = refundInputFromPrompt(prompt);
  const traces: ToolTrace[] = [];

  const search = searchOrderTool({ orderId: input.orderId });
  traces.push(searchTrace(input.orderId));
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
