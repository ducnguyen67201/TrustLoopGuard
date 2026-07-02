import { z } from 'zod';

// The one home for first-run onboarding logic: the copy-paste snippets and
// wire schemas used by /onboarding/connect and /onboarding/verify. The SDK
// snippet must stay in lockstep with sdks/typescript/README.md ("Quick
// start") — update both together.

export const assistantOptions = [
  { id: 'claude', label: 'Claude Code' },
  { id: 'cursor', label: 'Cursor' },
  { id: 'hermes', label: 'Hermes' },
  { id: 'codex', label: 'Codex' },
] as const;

export type AssistantKind = (typeof assistantOptions)[number]['id'];

const assistantInstructions: Record<AssistantKind, string> = {
  claude:
    'Open Claude Code at the project root, paste this prompt, and let it edit the code. Ask before running deploys or changing external services.',
  cursor:
    'Open this project in Cursor, paste this into Chat with codebase context enabled, and apply the generated edits after review.',
  hermes:
    'Open Hermes in this project, paste this as an implementation task, and keep any secrets in environment variables only.',
  codex:
    'Open Codex at the project root, paste this prompt, and let it make the minimal code change plus one runnable check.',
};

/**
 * TypeScript quick-start shown on the connect step. The API key is referenced
 * only via the TLG_API_KEY env var — the plaintext secret is never baked into
 * snippet text.
 */
export function buildSdkSnippet(opts: { baseUrl: string; agentId: string }): string {
  return `import { Client, guard } from '@trustloopguard/sdk';

const client = new Client({
  baseUrl: process.env.TLG_URL ?? '${opts.baseUrl}',
  apiKey: process.env.TLG_API_KEY,
});

const reply = await client.withRun({ agentId: '${opts.agentId}', kind: 'chat_session' }, async (run) => {
  return run.withEvent({ kind: 'user_turn', metadata: {} }, () =>
    guard({
      client,
      agentId: '${opts.agentId}',
      input: userMessage,
      draft: agentDraft,
      onBlock: () => "I can't help with that.",
      onEscalate: () => 'A human will follow up.',
    }),
  );
});`;
}

/**
 * A self-contained prompt the user pastes into their AI coding assistant
 * (Claude Code, Cursor, …) to do the integration for them.
 */
export function buildAssistantPrompt(opts: {
  baseUrl: string;
  agentId: string;
  assistant: AssistantKind;
}): string {
  return `Add TrustLoopGuard runtime guardrails to this project.

Assistant workflow: ${assistantInstructions[opts.assistant]}

1. Install the SDK: npm install @trustloopguard/sdk
2. Add two environment variables (I already have the values):
   TLG_URL=${opts.baseUrl}
   TLG_API_KEY=<the API key I just created — ask me to paste it into .env, do not hardcode it>
3. Create a shared client: new Client({ baseUrl: process.env.TLG_URL, apiKey: process.env.TLG_API_KEY }) from '@trustloopguard/sdk'.
4. Wrap my agent's LLM call with guard() using agentId '${opts.agentId}': pass the user input as \`input\` and the model's draft reply as \`draft\`, and handle onBlock and onEscalate with safe fallback messages. Group calls with client.withRun({ agentId: '${opts.agentId}', kind: 'chat_session' }, ...).
5. Run the agent once end-to-end so a real request goes through the guard — I'm watching for the first event on my TrustLoopGuard dashboard.`;
}

/**
 * Keeps an agent id snippet-safe: anything outside [a-zA-Z0-9_-] becomes a
 * dash, so free-form input can never break out of the quoted '${agentId}'
 * slots in the generated code/prompt.
 */
export function sanitizeAgentId(value: string): string {
  return value.replace(/[^a-zA-Z0-9_-]+/g, '-').replace(/-{2,}/g, '-');
}

export function approvedWorkspaceLandingPath(opts: {
  workspaceSlug: string;
  agentCount: number;
  environmentId?: string | null;
}): string {
  const query = new URLSearchParams({ workspace: opts.workspaceSlug });
  if (opts.environmentId) query.set('environment', opts.environmentId);
  return `${opts.agentCount === 0 ? '/onboarding/connect' : '/'}?${query.toString()}`;
}

// Wire shape of POST /api/api-keys (proxied Rust POST /v1/api-keys), reduced
// to the fields the connect step renders.
export const createApiKeyResponseSchema = z.object({
  api_key: z.object({
    id: z.string(),
    name: z.string(),
    prefix: z.string(),
  }),
  plaintext_key: z.string(),
});

export type CreatedApiKey = z.infer<typeof createApiKeyResponseSchema>;

// Wire shape of GET /api/traces (proxied Rust GET /v1/traces), reduced to the
// fields the verify step renders. zod strips unrecognized keys, so the richer
// server payload passes through unchanged.
export const traceListSchema = z.object({
  traces: z.array(
    z.object({
      trace_id: z.string(),
      decision: z.string(),
      elapsed_ms: z.number(),
      created_at: z.string(),
    }),
  ),
});

export type FirstTrace = z.infer<typeof traceListSchema>['traces'][number];
