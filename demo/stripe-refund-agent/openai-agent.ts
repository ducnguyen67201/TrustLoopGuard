import OpenAI from 'openai';
import type {
  ChatCompletionMessage,
  ChatCompletionMessageParam,
} from 'openai/resources/chat/completions';

import { OPENAI_API_KEY, OPENAI_MODEL } from '../shared/env';
import type { RefundAgentClient } from './core';
import { refundAgentTools, runRefundTool } from './tool-runner';
import type { AgentRunOptions, AgentRunResult, ToolTrace } from './types';

const SYSTEM_PROMPT = [
  'You are a refund support agent.',
  'Use search_order before preparing a refund.',
  'Use prepare_refund before execute_refund.',
  'Only execute refunds through TrustLoopGuard.',
  'Never ask for or mention Stripe secret keys.',
].join(' ');

export async function runOpenAiRefundAgent(
  prompt: string,
  client: RefundAgentClient,
  options: AgentRunOptions = {},
): Promise<AgentRunResult> {
  const openai = new OpenAI({ apiKey: OPENAI_API_KEY });
  const state = new AgentState(prompt);
  const messages = initialMessages(prompt);

  options.logger?.log('openai_agent', 'starting OpenAI function-call loop');
  try {
    for (let step = 0; step < 6; step += 1) {
      const message = await nextAssistantMessage(openai, messages);
      if (message === undefined) break;
      messages.push(message);

      if (message.tool_calls === undefined || message.tool_calls.length === 0) {
        state.setFinalMessage(
          typeof message.content === 'string' ? message.content.trim() : undefined,
        );
        break;
      }

      for (const call of message.tool_calls) {
        if (call.type !== 'function') continue;
        const result = await runRefundTool(call.function.name, call.function.arguments, client, {
          logger: options.logger,
          requestId: options.requestId,
          dbPath: options.dbPath,
          refundGrantId: options.refundGrantId,
          allowGrantProvisioning: options.allowGrantProvisioning,
        });
        state.recordToolResult(result);
        messages.push({
          role: 'tool',
          tool_call_id: call.id,
          content: JSON.stringify(result.toolResult),
        });
      }
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (options.requireLiveAgent === true) {
      options.logger?.log('openai_agent', 'stopped safely after a tool error');
      throw error;
    }
    options.logger?.log('openai_agent', `stopped after tool work: ${message}`);
    if (!state.hasToolResults()) throw error;
    state.setFinalMessage(`The refund agent stopped after a tool error: ${message}`);
  }

  return state.toResult();
}

function initialMessages(prompt: string): ChatCompletionMessageParam[] {
  return [
    { role: 'system', content: SYSTEM_PROMPT },
    { role: 'user', content: prompt },
  ];
}

async function nextAssistantMessage(
  openai: OpenAI,
  messages: ChatCompletionMessageParam[],
): Promise<ChatCompletionMessage | undefined> {
  const response = await openai.chat.completions.create({
    model: OPENAI_MODEL,
    messages,
    tools: refundAgentTools,
    tool_choice: 'auto',
  });
  return response.choices[0]?.message;
}

class AgentState {
  private readonly traces: ToolTrace[] = [];
  private actionId: string | undefined;
  private receiptId: string | undefined;
  private finalMessage = '';

  constructor(private readonly prompt: string) {}

  recordToolResult(result: {
    trace: ToolTrace;
    actionId?: string;
    receiptId?: string;
  }): void {
    this.traces.push(result.trace);
    if (result.actionId !== undefined) this.actionId = result.actionId;
    if (result.receiptId !== undefined) this.receiptId = result.receiptId;
  }

  setFinalMessage(message: string | undefined): void {
    this.finalMessage = message ?? '';
  }

  hasToolResults(): boolean {
    return this.traces.length > 0;
  }

  toResult(): AgentRunResult {
    return {
      prompt: this.prompt,
      traces: this.traces,
      finalMessage: this.finalMessage || this.defaultFinalMessage(),
      actionId: this.actionId,
      receiptId: this.receiptId,
    };
  }

  private defaultFinalMessage(): string {
    const last = this.traces.at(-1)?.summary ?? 'no refund tool ran';
    if (this.receiptId !== undefined) return `Refund executed. Receipt ${this.receiptId} is ready.`;
    if (this.actionId !== undefined) return `Refund action ${this.actionId}: ${last}`;
    return last;
  }
}
