import { z } from 'zod';

// The one home for first-run onboarding logic: step derivation and the
// copy-paste snippets shown on /onboarding/connect. The SDK snippet must stay
// in lockstep with sdks/typescript/README.md ("Quick start") — update both
// together.

export type OnboardingStep = 'workspace' | 'connect' | 'verify' | 'done';

/**
 * Derives where a user is in first-run onboarding from data the dashboard
 * shell already loads. Deliberately stateless: no durable "onboarding
 * completed" flag exists. Traces win over key count so revoking every key
 * later never drops an active account back into onboarding.
 */
export function deriveOnboardingStep(input: {
  workspaceCount: number;
  apiKeyCount: number;
  hasTraces: boolean;
}): OnboardingStep {
  if (input.workspaceCount === 0) return 'workspace';
  if (input.hasTraces) return 'done';
  if (input.apiKeyCount === 0) return 'connect';
  return 'verify';
}

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
export function buildAssistantPrompt(opts: { baseUrl: string; agentId: string }): string {
  return `Add TrustLoopGuard runtime guardrails to this project.

1. Install the SDK: npm install @trustloopguard/sdk
2. Add two environment variables (I already have the values):
   TLG_URL=${opts.baseUrl}
   TLG_API_KEY=<the API key I just created — ask me to paste it into .env, do not hardcode it>
3. Create a shared client: new Client({ baseUrl: process.env.TLG_URL, apiKey: process.env.TLG_API_KEY }) from '@trustloopguard/sdk'.
4. Wrap my agent's LLM call with guard() using agentId '${opts.agentId}': pass the user input as \`input\` and the model's draft reply as \`draft\`, and handle onBlock and onEscalate with safe fallback messages. Group calls with client.withRun({ agentId: '${opts.agentId}', kind: 'chat_session' }, ...).
5. Run the agent once end-to-end so a real request goes through the guard — I'm watching for the first event on my TrustLoopGuard dashboard.`;
}

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
