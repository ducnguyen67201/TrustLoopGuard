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
  return `import { guardAgent } from '@trustloopguard/sdk';

const agent = guardAgent(createAgent(), {
  agentId: '${opts.agentId}',
  baseUrl: process.env.TLG_URL ?? '${opts.baseUrl}',
  apiKey: process.env.TLG_API_KEY,
});

const reply = await agent.reply(userMessage);`;
}

export function buildPaymentSdkSnippet(opts: { baseUrl: string; agentId: string }): string {
  return `import { Client } from '@trustloopguard/sdk';

const client = new Client({
  baseUrl: process.env.TLG_URL ?? '${opts.baseUrl}',
  apiKey: process.env.TLG_API_KEY,
});

// 1. After verified user intent, create a reusable grant with the exact payment scope.
const grant = await client.createGrant({
  principal_id: '${opts.agentId}',
  domain: 'financial',
  capability: 'financial:x402_read_paid_resource',
  scope: {
    scope_type: 'financial',
    scope: {
      action_kinds: ['payment'],
      operation: 'x402_read_paid_resource',
      maximum_amount_minor: 500n,
      currency: 'USD',
      rail: 'x402',
      counterparties: ['0xmerchant'],
      x402_hosts: ['merchant.example'],
      x402_resources: ['/premium/article'],
      x402_networks: ['base-sepolia'],
      x402_assets: ['USDC'],
      x402_payees: ['0xmerchant'],
      required_preconditions: [],
    },
  },
  requirement_ids: ['financial:x402-agentic-payment-grant-required:grant_required'],
});

// 2. Your agent requests the resource and receives HTTP 402 from the merchant.
const requirement = paymentRequired.accepts[0];

// 3. TrustLoopGuard authorizes before the wallet signs or retries payment.
const auth = await client.authorizeAgenticPayment({
  idempotency_key: crypto.randomUUID(),
  principal_id: '${opts.agentId}',
  session_id: checkoutSessionId,
  operation: 'x402_read_paid_resource',
  authorization: { grant_id: grant.id, attempt_id: crypto.randomUUID() },
  payment_requirement: requirement,
  evidence: [],
  metadata: { order_id: checkoutSessionId },
});

if (!auth.signable) throw new Error('TrustLoopGuard denied payment: ' + auth.reason);

// 4. Sign/pay with your wallet or x402 client, then commit or rollback.
const settlement = await payWithWallet(requirement);
await client.commitAgenticPayment(auth.record.id, { proof: settlement });`;
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

1. Install the SDK: npm install @trustloopguard/sdk
2. Add two environment variables (I already have the values):
   TLG_URL=${opts.baseUrl}
   TLG_API_KEY=<the API key I just created — ask me to paste it into .env, do not hardcode it>
3. Find the agent object whose async agent.reply(...) method takes the user's message as its first argument and returns the final reply string. Keep the agent internals unchanged.
4. Import guardAgent from '@trustloopguard/sdk' and decorate the agent once where it is created: const agent = guardAgent(createAgent(), { agentId: '${opts.agentId}' }). Keep every existing agent.reply(...) call site unchanged. If the framework has no reply-style object, add one thin adapter at the single final-output boundary; do not scatter guard checks throughout the code.
5. Do not clone TrustLoopGuard, start a local TrustLoopGuard server, construct GuardEvent manually, or add run/monitoring setup.
6. Run the agent once end-to-end so a real request goes through the guard — I'm watching for the first event on my TrustLoopGuard dashboard.

Assistant workflow: ${assistantInstructions[opts.assistant]}`;
}

// The PreToolUse hook script installed by buildClaudeCodeHookPrompt. Kept
// free of backticks and ${…} so it can live inside a template literal.
const CLAUDE_HOOK_SCRIPT = `#!/usr/bin/env node
// TrustLoopGuard PreToolUse hook: ask the guard before every tool call.
// permit -> tool runs; deny -> denied (reason shown to the model);
// require_approval/defer/transform -> Claude Code asks the human. Guard unreachable -> fail open.
const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
let hook;
try {
  hook = JSON.parse(Buffer.concat(chunks).toString('utf8'));
} catch {
  process.exit(0);
}

const tool = hook.tool_name || 'unknown_tool';
const params = hook.tool_input || {};
const SIDE_EFFECTS = {
  Bash: 'shell_exec',
  Write: 'file_write',
  Edit: 'file_write',
  NotebookEdit: 'file_write',
  Read: 'read',
  Glob: 'read',
  Grep: 'read',
  WebFetch: 'network_call',
  WebSearch: 'network_call',
};
const source = {
  id: 'conversation',
  origin: 'user',
  labels: { trust: 'untrusted', confidentiality: 'unknown', integrity: 'unknown' },
};
const provenance = Object.fromEntries(Object.keys(params).map((key) => [key, [source.id]]));

const event = {
  kind: 'tool.call.proposed',
  principal: {
    workspace_id: '',
    environment_id: '',
    agent_id: process.env.TLG_AGENT_ID || 'claude-code',
  },
  action: { operation: tool, parameters: params, side_effect: SIDE_EFFECTS[tool] || 'api_mutation' },
  sources: [source],
  provenance,
  context: { channel: 'claude-code', session_id: hook.session_id || null },
};

try {
  const baseUrl = (process.env.TLG_URL || 'http://127.0.0.1:8080').replace(/\\/$/, '');
  const headers = { 'content-type': 'application/json' };
  if (process.env.TLG_API_KEY) headers.authorization = 'Bearer ' + process.env.TLG_API_KEY;
  const res = await fetch(baseUrl + '/v1/events', {
    method: 'POST',
    headers,
    body: JSON.stringify(event),
    signal: AbortSignal.timeout(3000),
  });
  if (!res.ok) process.exit(0);
  const decision = await res.json();
  if (decision.effect && decision.effect !== 'permit') {
    const reason = decision.reason || decision.violated_rule || 'workspace policy';
    process.stdout.write(
      JSON.stringify({
        hookSpecificOutput: {
          hookEventName: 'PreToolUse',
          permissionDecision: decision.effect === 'deny' ? 'deny' : 'ask',
          permissionDecisionReason:
            'TrustLoopGuard ' + decision.effect + ': ' + reason +
            ' (trace ' + (decision.trace_id || 'n/a') + ')',
        },
      }),
    );
  }
} catch {
  process.exit(0);
}`;

/**
 * Quick setup for when Claude Code IS the agent being guarded (not merely the
 * assistant doing an SDK integration): a self-contained prompt the user
 * pastes into Claude Code that installs a PreToolUse hook, so every tool call
 * is checked at POST /v1/events before it executes.
 */
export function buildClaudeCodeHookPrompt(opts: { baseUrl: string; agentId: string }): string {
  return `Guard this Claude Code session with TrustLoopGuard (agent id: ${opts.agentId}): install a PreToolUse hook so every tool call is checked by the guard BEFORE it runs.

1. Create .claude/hooks/tlg-guard.mjs with exactly this content:

${CLAUDE_HOOK_SCRIPT}

2. Merge this into .claude/settings.json (create the file if missing; preserve any existing keys):

{
  "env": {
    "TLG_URL": "${opts.baseUrl}",
    "TLG_AGENT_ID": "${opts.agentId}"
  },
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "*",
        "hooks": [
          { "type": "command", "command": "node \\"$CLAUDE_PROJECT_DIR/.claude/hooks/tlg-guard.mjs\\"" }
        ]
      }
    ]
  }
}

3. Never write my API key into any file. Remind me to run \`export TLG_API_KEY=<the key I just created>\` in the shell where I launch Claude Code, then restart Claude Code so the hook picks it up.
4. Verify: after the restart, run any harmless tool (list the project files). I'm watching for the first tool.call.proposed event on my TrustLoopGuard dashboard.`;
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
