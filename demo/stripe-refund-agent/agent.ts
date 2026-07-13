import { OPENAI_API_KEY } from '../shared/env';
import { runOpenAiRefundAgent } from './openai-agent';
import { runScriptedRefundAgent } from './scripted-agent';
import type { RefundAgentClient } from './core';
import type { AgentRunOptions, AgentRunResult } from './types';

export async function runRefundAgent(
  prompt: string,
  client: RefundAgentClient,
  options: AgentRunOptions = {},
): Promise<AgentRunResult> {
  if (!shouldUseOpenAI(options.useOpenAI)) {
    if (options.requireLiveAgent === true) {
      throw new Error('live agent mode requires OPENAI_API_KEY and OpenAI tool calling');
    }
    return runScriptedRefundAgent(prompt, client, options);
  }

  try {
    return await runOpenAiRefundAgent(prompt, client, options);
  } catch (error) {
    if (options.requireLiveAgent === true) {
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`live agent failed: ${message}`, { cause: error });
    }
    options.logger?.log('openai_fallback', 'OpenAI agent failed, using scripted refund flow');
    return runScriptedRefundAgent(prompt, client, options);
  }
}

function shouldUseOpenAI(useOpenAI: boolean | undefined): boolean {
  return useOpenAI !== false && OPENAI_API_KEY !== undefined && OPENAI_API_KEY.trim() !== '';
}
