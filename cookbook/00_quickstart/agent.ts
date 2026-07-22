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
  workspaceId?: string;
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
    fetchImpl: withLocalWorkspace(options.fetchImpl, options.workspaceId),
    retry: options.retry,
    log: options.log,
    failClosed: true,
    onBlock: DEFAULT_SAFE_REPLY,
    onError: DEFAULT_SAFE_REPLY,
  });

  return guardrail.wrap(draftSupportReply);
}

function withLocalWorkspace(
  fetchImpl: typeof fetch | undefined,
  workspaceId: string | undefined,
): typeof fetch | undefined {
  const resolvedWorkspaceId = workspaceId?.trim();
  if (!resolvedWorkspaceId) return fetchImpl;

  const delegate = fetchImpl ?? globalThis.fetch.bind(globalThis);
  return async (input, init) => {
    const headers = new Headers(init?.headers);
    headers.set('x-tlg-workspace-id', resolvedWorkspaceId);
    return delegate(input, { ...init, headers });
  };
}
