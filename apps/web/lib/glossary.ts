// Plain-language definitions for the domain terms the dashboard surfaces.
//
// One home for "what does this word mean?" so we never re-explain a term in two
// places with different wording. Consumed by `InfoHint` (hover help next to a
// label) and `VerdictLegend` (the allow/rewrite/block/escalate key). Keep each
// `short` to one friendly sentence a non-technical teammate would understand —
// no jargon inside the definition itself.

export interface GlossaryEntry {
  /** Human label for the term (used in aria text and legends). */
  label: string;
  /** One-sentence, plain-language meaning. */
  short: string;
}

export const GLOSSARY = {
  // Verdicts — the four decisions the guardrail can make about a request.
  allow: {
    label: 'Allow',
    short: 'Safe — the request passed through unchanged.',
  },
  rewrite: {
    label: 'Rewrite',
    short: 'The guardrail cleaned up the request, then let it through.',
  },
  block: {
    label: 'Block',
    short: 'Stopped — the request broke one of your rules.',
  },
  escalate: {
    label: 'Escalate',
    short: 'Held for a person to review before it continues.',
  },

  // Core nouns.
  guardrail: {
    label: 'Guardrail',
    short:
      'A safety check that runs on every request to your AI app, so risky ones get blocked, rewritten, or flagged automatically.',
  },
  verdict: {
    label: 'Verdict',
    short: 'The decision the guardrail made about a request: allow, rewrite, block, or escalate.',
  },
  policy: {
    label: 'Policy',
    short: 'A rule you set that tells the guardrail what to allow, rewrite, block, or escalate.',
  },
  policyKey: {
    label: 'Policy ID',
    short:
      'A short, lowercase id the engine uses to refer to this rule (for example, no-pii). It is not the friendly name — use the description for that.',
  },
  agent: {
    label: 'Agent',
    short: 'One of your AI assistants or apps that sends requests through the guardrail.',
  },
  knowledgeSource: {
    label: 'Knowledge source',
    short: 'A document or link you add so the guardrail knows your approved, trusted content.',
  },
  gateway: {
    label: 'Gateway',
    short:
      'A drop-in URL you send your AI service calls through, so every request is checked automatically — no code changes.',
  },
  provider: {
    label: 'Provider',
    short: 'An AI service you connect, like OpenAI or Anthropic, that the gateway forwards requests to.',
  },
  route: {
    label: 'Route',
    short:
      'A gateway address that connects one provider and agent; enabled policies apply automatically.',
  },
  run: {
    label: 'Run',
    short: 'A single request that went through the guardrail, with the decision it received.',
  },
  trace: {
    label: 'Trace',
    short: 'The step-by-step record of how one request was checked and decided.',
  },
  environment: {
    label: 'Environment',
    short:
      'A separate space for your work — like development, staging, or production — so testing never touches live traffic.',
  },
  workspace: {
    label: 'Workspace',
    short: 'A project space with its own policies, agents, and team. Keep separate products or clients apart.',
  },
  redteam: {
    label: 'Red-team test',
    short: 'A safe, simulated attack you run on your own agent to see how well your guardrails hold up.',
  },
  severity: {
    label: 'Severity',
    short: 'How serious a policy treats a violation: low, medium, high, or critical.',
  },
  latency: {
    label: 'Latency',
    short: 'How long the guardrail took to check a request, in milliseconds. Lower is faster.',
  },
  apiKey: {
    label: 'API key',
    short: 'A secret token your app uses to connect to the guardrail. Treat it like a password.',
  },
  scope: {
    label: 'Scope',
    short: 'Which agents or traffic a setting applies to.',
  },
  role: {
    label: 'Role',
    short:
      'What a teammate can do. Owners control everything, Admins manage rules and people, Editors change rules and agents, and Viewers can look but not change anything.',
  },
} as const;

export type GlossaryTerm = keyof typeof GLOSSARY;
