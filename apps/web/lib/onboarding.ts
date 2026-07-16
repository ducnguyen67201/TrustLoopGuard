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

// The command hook installed by buildClaudeCodeHookPrompt. It contains only
// Node built-ins so onboarding never installs another runtime dependency.
export const CLAUDE_HOOK_SCRIPT = `#!/usr/bin/env node
import { createHash, randomUUID } from 'node:crypto';
import { chmod, lstat, mkdir, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const BRIDGE_VERSION = 'trustloopguard-claude-hook-v1';
const REQUEST_TIMEOUT_MS = 3000;
const APPROVAL_TIMEOUT_MS = positiveInt(process.env.TLG_APPROVAL_TIMEOUT_MS, 300000);
const APPROVAL_POLL_MS = positiveInt(process.env.TLG_APPROVAL_POLL_MS, 1000);
const READ_ONLY_TOOLS = new Set(['Read', 'Glob', 'Grep']);
const EVENT_KINDS = {
  Bash: 'shell.action.proposed',
  Write: 'file.action.proposed',
  Edit: 'file.action.proposed',
  NotebookEdit: 'file.action.proposed',
  WebFetch: 'network.request.proposed',
  WebSearch: 'network.request.proposed',
};
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

function positiveInt(value, fallback) {
  const parsed = Number.parseInt(value || '', 10);
  return Number.isSafeInteger(parsed) && parsed > 0 ? parsed : fallback;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function decisionOutput(decision, reason) {
  return {
    hookSpecificOutput: {
      hookEventName: 'PreToolUse',
      permissionDecision: decision,
      permissionDecisionReason: reason,
    },
  };
}

function emitDecision(decision, reason) {
  process.stdout.write(JSON.stringify(decisionOutput(decision, reason)));
}

function describeDecision(decision) {
  const effect = decision && decision.effect ? decision.effect : 'unexpected_response';
  const reason = decision && decision.reason ? decision.reason : 'the guard returned no reason';
  const trace = decision && decision.trace_id ? decision.trace_id : 'n/a';
  return 'TrustLoopGuard ' + effect + ': ' + reason + ' (trace ' + trace + ')';
}

function statePath(hook) {
  const userSuffix = typeof process.getuid === 'function' ? '-' + process.getuid() : '';
  const stateDir = process.env.TLG_HOOK_STATE_DIR || join(tmpdir(), 'trustloopguard-claude-hooks' + userSuffix);
  const key = sha256(String(hook.session_id || '') + '\\0' + String(hook.tool_use_id || ''));
  return { stateDir, file: join(stateDir, key + '.json') };
}

async function secureStateDirectory(stateDir) {
  let existing;
  try {
    existing = await lstat(stateDir);
  } catch (error) {
    if (!error || error.code !== 'ENOENT') throw error;
    try {
      await mkdir(stateDir, { recursive: false, mode: 0o700 });
    } catch (mkdirError) {
      if (!mkdirError || mkdirError.code !== 'EEXIST') throw mkdirError;
    }
    existing = await lstat(stateDir);
  }
  if (!existing.isDirectory() || existing.isSymbolicLink()) {
    throw new Error('hook state path is not a real directory');
  }
  if (typeof process.getuid === 'function' && existing.uid !== process.getuid()) {
    throw new Error('hook state directory is owned by another user');
  }
  await chmod(stateDir, 0o700);
}

async function storeLease(hook, leaseId) {
  const target = statePath(hook);
  await secureStateDirectory(target.stateDir);
  const temporary = target.file + '.' + randomUUID() + '.tmp';
  await writeFile(
    temporary,
    JSON.stringify({
      lease_id: leaseId,
      tool_use_id: hook.tool_use_id,
      session_id: hook.session_id || '',
    }),
    { encoding: 'utf8', mode: 0o600, flag: 'wx' },
  );
  await rename(temporary, target.file);
}

function requestHeaders() {
  const headers = { 'content-type': 'application/json' };
  if (process.env.TLG_API_KEY) headers.authorization = 'Bearer ' + process.env.TLG_API_KEY;
  return headers;
}

async function requestJson(pathname, init = {}) {
  const baseUrl = (process.env.TLG_URL || 'http://127.0.0.1:8080').replace(/\\/$/, '');
  const response = await fetch(baseUrl + pathname, {
    ...init,
    headers: { ...requestHeaders(), ...(init.headers || {}) },
    signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error('TrustLoopGuard returned HTTP ' + response.status);
  return response.json();
}

function buildEvent(hook) {
  const tool = String(hook.tool_name || 'unknown_tool');
  const rawParameters = hook.tool_input && typeof hook.tool_input === 'object' ? hook.tool_input : {};
  const kind = EVENT_KINDS[tool] || 'tool.call.proposed';
  const parameters = tool === 'Bash'
    ? {
        command: typeof rawParameters.command === 'string' ? rawParameters.command : '',
        shell: 'bash',
        cwd: hook.cwd || undefined,
        workspace_root: process.env.CLAUDE_PROJECT_DIR || hook.cwd || undefined,
        timeout_ms: Number.isSafeInteger(rawParameters.timeout) && rawParameters.timeout > 0
          ? rawParameters.timeout
          : undefined,
        run_in_background: rawParameters.run_in_background === true,
      }
    : rawParameters;
  const normalizedFields = tool === 'Bash'
    ? ['command', 'shell', 'cwd', 'workspace_root', 'timeout_ms', 'run_in_background']
    : Object.keys(parameters).sort();
  const source = {
    id: 'conversation',
    origin: 'user',
    labels: { trust: 'untrusted', confidentiality: 'unknown', integrity: 'unknown' },
  };
  return {
    kind,
    principal: {
      workspace_id: '',
      environment_id: '',
      agent_id: process.env.TLG_AGENT_ID || 'claude-code',
      session_id: hook.session_id || undefined,
    },
    action: {
      operation: tool,
      parameters,
      side_effect: SIDE_EFFECTS[tool] || 'api_mutation',
      invocation_id: hook.tool_use_id,
      tool_identity: {
        server_id: 'claude-code',
        tool_name: tool,
        schema_hash: 'sha256:' + sha256(JSON.stringify([BRIDGE_VERSION, tool, kind, normalizedFields])),
      },
    },
    sources: [source],
    provenance: Object.fromEntries(Object.keys(parameters).map((key) => [key, [source.id]])),
    context: { channel: 'claude-code' },
  };
}

async function submitEvent(event) {
  return requestJson('/v1/events', { method: 'POST', body: JSON.stringify(event) });
}

async function awaitApproval(summary) {
  const deadline = Date.now() + APPROVAL_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const approval = await requestJson(
      '/v1/authorization/approvals/' + encodeURIComponent(summary.id),
      { method: 'GET' },
    );
    if (approval.status === 'approved' && approval.grant_id) return approval.grant_id;
    if (['denied', 'canceled', 'expired'].includes(approval.status)) {
      throw new Error('approval ' + approval.status);
    }
    await new Promise((resolve) => setTimeout(resolve, APPROVAL_POLL_MS));
  }
  throw new Error('approval timed out');
}

async function handlePreToolUse(hook) {
  const tool = String(hook.tool_name || 'unknown_tool');
  const readOnly = READ_ONLY_TOOLS.has(tool);
  if (!hook.tool_use_id) {
    if (!readOnly) emitDecision('deny', 'TrustLoopGuard denied the tool because tool_use_id is missing.');
    return;
  }

  const event = buildEvent(hook);
  try {
    let decision = await submitEvent(event);
    if (decision.effect === 'require_approval' && decision.approval && decision.approval.id) {
      const grantId = await awaitApproval(decision.approval);
      event.action.authorization = { grant_id: grantId, attempt_id: randomUUID() };
      decision = await submitEvent(event);
      if (decision.effect !== 'permit' || !decision.lease || !decision.lease.id) {
        emitDecision('deny', describeDecision(decision) + '; approved actions require an execution lease.');
        return;
      }
    }
    if (decision.effect === 'permit') {
      if (decision.lease && decision.lease.id) await storeLease(hook, decision.lease.id);
      emitDecision('allow', describeDecision(decision));
      return;
    }
    emitDecision('deny', describeDecision(decision) + '; review the trace before retrying.');
  } catch (error) {
    if (!readOnly) {
      const reason = error instanceof Error ? error.message : 'guard unavailable';
      emitDecision('deny', 'TrustLoopGuard could not authorize this high-impact tool: ' + reason + '.');
    }
  }
}

async function handlePostToolUse(hook) {
  if (!hook.tool_use_id) return;
  const target = statePath(hook);
  let state;
  try {
    state = JSON.parse(await readFile(target.file, 'utf8'));
  } catch {
    return;
  }
  const status = hook.hook_event_name === 'PostToolUse' ? 'consumed' : 'canceled';
  let lastError;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    try {
      await requestJson('/v1/authorization/leases/' + encodeURIComponent(state.lease_id) + '/complete', {
        method: 'POST',
        body: JSON.stringify({ status, outcome: { hook_event_name: hook.hook_event_name } }),
      });
      await rm(target.file, { force: true });
      return;
    } catch (error) {
      lastError = error;
    }
  }
  const message = lastError instanceof Error ? lastError.message : 'unknown error';
  process.stderr.write('TrustLoopGuard lease completion failed; state retained: ' + message + '\\n');
}

const chunks = [];
for await (const chunk of process.stdin) chunks.push(chunk);
let hook;
try {
  hook = JSON.parse(Buffer.concat(chunks).toString('utf8'));
} catch {
  emitDecision('deny', 'TrustLoopGuard could not parse the Claude hook request.');
  process.exit(0);
}

if (hook.hook_event_name === 'PreToolUse') {
  await handlePreToolUse(hook);
} else if (hook.hook_event_name === 'PostToolUse' || hook.hook_event_name === 'PostToolUseFailure') {
  await handlePostToolUse(hook);
}`;

/**
 * Quick setup for when Claude Code IS the agent being guarded (not merely the
 * assistant doing an SDK integration): a self-contained prompt the user
 * pastes into Claude Code that installs a PreToolUse hook, so every tool call
 * is checked at POST /v1/events before it executes.
 */
export function buildClaudeCodeHookPrompt(opts: { baseUrl: string; agentId: string }): string {
  return `Guard this Claude Code session with TrustLoopGuard (agent id: ${opts.agentId}): install command hooks so every tool call is authorized before it runs and every execution lease is completed afterward.

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
          {
            "type": "command",
            "command": "node",
            "args": ["$CLAUDE_PROJECT_DIR/.claude/hooks/tlg-guard.mjs"],
            "timeout": 330
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "node",
            "args": ["$CLAUDE_PROJECT_DIR/.claude/hooks/tlg-guard.mjs"],
            "timeout": 330
          }
        ]
      }
    ],
    "PostToolUseFailure": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "node",
            "args": ["$CLAUDE_PROJECT_DIR/.claude/hooks/tlg-guard.mjs"],
            "timeout": 330
          }
        ]
      }
    ]
  }
}

3. Never write my API key into any file. Remind me to run \`export TLG_API_KEY=<the key I just created>\` in the shell where I launch Claude Code, then restart Claude Code so the hook picks it up.
4. Verify: after the restart, run any harmless tool (list the project files). I'm watching for the first executable event on my TrustLoopGuard dashboard.`;
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
