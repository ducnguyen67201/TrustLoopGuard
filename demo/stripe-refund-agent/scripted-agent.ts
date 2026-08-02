import { randomUUID } from 'node:crypto';

import { executeRefundTool, prepareRefundTool, searchOrderTool, type RefundAgentClient } from './core';
import { refundInputFromPrompt } from './prompt-parser';
import { searchTrace } from './tool-runner';
import type { AgentRunOptions, AgentRunResult, ToolTrace } from './types';

export async function runScriptedRefundAgent(
  prompt: string,
  client: RefundAgentClient,
  options: AgentRunOptions = {},
): Promise<AgentRunResult> {
  const input = {
    ...refundInputFromPrompt(prompt),
    requestId: options.requestId ?? randomUUID(),
  };
  const traces: ToolTrace[] = [];

  options.logger?.log(
    'parse_prompt',
    `order=${input.orderId} amount_minor=${input.amountMinor} reason=${input.reason}`,
  );
  const search = searchOrderTool({ orderId: input.orderId }, options.dbPath);
  traces.push(searchTrace(input.orderId, options.dbPath));
  options.logger?.log(
    'search_order',
    search.found
      ? `found ${input.orderId} with ${search.order?.refundableBalanceMinor ?? 0} refundable minor units`
      : `did not find ${input.orderId}`,
  );
  if (!search.found) {
    return {
      prompt,
      traces,
      finalMessage: `I could not find ${input.orderId}, so I did not prepare a refund.`,
    };
  }

  options.logger?.log('prepare_refund', 'submitting typed financial action to Featherlane AI');
  const prepared = await prepareRefundTool(input, client, options.dbPath, {
    grantId: options.refundGrantId,
    allowGrantProvisioning: options.allowGrantProvisioning,
  });
  traces.push({
    tool: 'prepare_refund',
    summary: `${prepared.status}: ${prepared.message}`,
  });
  options.logger?.log('prepare_refund', `${prepared.status}: ${prepared.action.id}`);
  if (prepared.status !== 'permit') {
    return {
      prompt,
      traces,
      finalMessage: `Featherlane AI returned ${prepared.status}. I did not create a Stripe refund.`,
      actionId: prepared.action.id,
    };
  }

  options.logger?.log('execute_refund', `executing action ${prepared.action.id}`);
  const executed = await executeRefundTool(prepared.action.id, client, options.dbPath, {
    grantId: options.refundGrantId,
    allowGrantProvisioning: options.allowGrantProvisioning,
  });
  traces.push({
    tool: 'execute_refund',
    summary: `${executed.status}: ${executed.message}`,
  });
  options.logger?.log('execute_refund', `${executed.status}: ${executed.action.id}`);

  return {
    prompt,
    traces,
    finalMessage:
      executed.status === 'succeeded'
        ? `Refund executed. Receipt ${executed.receipt?.id ?? executed.action.id} is ready.`
        : `Refund was not executed because the action is ${executed.status}.`,
    actionId: executed.action.id,
    receiptId: executed.receipt?.id,
  };
}
