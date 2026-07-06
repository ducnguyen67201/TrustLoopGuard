import { OPENAI_API_KEY } from '../shared/env';
import { runOpenAiRefundAgent } from './openai-agent';
import { runScriptedRefundAgent } from './scripted-agent';
import type { RefundAgentClient } from './core';
import type { AgentRunResult } from './types';

export async function runRefundAgent(
  prompt: string,
  client: RefundAgentClient,
  options: { useOpenAI?: boolean } = {},
): Promise<AgentRunResult> {
  if (!shouldUseOpenAI(options.useOpenAI)) {
    return runScriptedRefundAgent(prompt, client);
  }

  try {
    return await runOpenAiRefundAgent(prompt, client);
  } catch {
    return runScriptedRefundAgent(prompt, client);
  }
}

function shouldUseOpenAI(useOpenAI: boolean | undefined): boolean {
  return useOpenAI !== false && OPENAI_API_KEY !== undefined && OPENAI_API_KEY.trim() !== '';
}
