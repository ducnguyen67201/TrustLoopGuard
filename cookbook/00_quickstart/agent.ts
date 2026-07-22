import {
  guard,
  type GuardLogEvent,
  type RetryConfig,
} from '@trustloopguard/sdk';

export const DEFAULT_AGENT_ID = 'cookbook-support-agent';
export const DEFAULT_SAFE_REPLY =
  'I cannot share sensitive account information. A human teammate can help.';

export interface CookbookAgentOptions {
  agentId?: string;
  baseUrl?: string;
  apiKey?: string;
  fetchImpl?: typeof fetch;
  retry?: RetryConfig;
  log?: (event: GuardLogEvent) => void;
}

export function draftSupportReply(input: string): string {
  if (input.toLowerCase().includes('ssn')) {
    return "The customer's SSN is 123-45-6789.";
  }
  return 'Support is available from 9:00 to 17:00 UTC.';
}

export function createGuardedSupportAgent(
  options: CookbookAgentOptions = {},
): (input: string) => Promise<string> {
  const guardrail = guard({
    agentId: options.agentId ?? DEFAULT_AGENT_ID,
    baseUrl: options.baseUrl,
    apiKey: options.apiKey,
    fetchImpl: options.fetchImpl,
    retry: options.retry,
    log: options.log,
    failClosed: true,
    onBlock: DEFAULT_SAFE_REPLY,
    onError: DEFAULT_SAFE_REPLY,
  });

  return guardrail.wrap(draftSupportReply);
}
