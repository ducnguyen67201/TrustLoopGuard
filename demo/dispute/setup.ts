// dispute:setup — provision the platform (the source of truth) so the guarded
// agent is protected. This mirrors the real onboarding: you paste the agent's
// system prompt, the platform AUTO-GENERATES content guardrails from it, and you
// register tool metadata for the one thing a prompt can't express — which
// parameters move money and where they may come from.
//
//   1. POST /v1/agents                      register the agent + its system_prompt
//   2. POST /v1/agents/{id}/guardrails/generate   LLM derives content policies
//      + PATCH /v1/policies/{id}/enabled     turn the generated ones on
//   3. POST /v1/tool-metadata               action guard for issue_refund
//
// Steps 1–2 = "paste prompt, get protection". Step 3 = the tool/action rule the
// param-auth checker reads (cannot be inferred from prose). Step 2 is best-effort:
// it needs an LLM configured on the server; if it can't run, the action guard
// (step 3) still installs.
//
//   pnpm --filter @trustloopguard/demo dispute:setup
import type { AgentProfile, GuardrailGenerateResponse } from '@trustloopguard/sdk';

import { API_KEY, DEFAULT_AGENT_ID, SERVER_URL, WORKSPACE_ID } from '../shared/env';

const AGENT_ID = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;

// The customer-facing persona you'd paste into the platform. The richer it is,
// the better the auto-generated content guardrails (PII, scope, no fake promises).
const SYSTEM_PROMPT = [
  'You are NorthPay Disputes, a payment-dispute support agent for a fintech.',
  'You help customers open and check chargeback/dispute cases and can issue refunds to a verified account on file.',
  'Stay strictly on payment disputes and refunds — decline anything else.',
  "Never reveal a customer's full card number, SSN, or any other customer's data.",
  'Never promise a guaranteed refund, a specific amount, or a timeline before review.',
  'Be concise and professional.',
].join(' ');

const agentProfile: AgentProfile = {
  agent_id: AGENT_ID,
  display_name: 'NorthPay Disputes',
  system_prompt: SYSTEM_PROMPT,
  scope: {
    in_scope: ['payment dispute handling', 'refund processing'],
    out_of_scope: ['tax preparation'],
  },
  authority: {
    can_promise: ['open a dispute case for review'],
    cannot_promise: ['guaranteed refund approval'],
  },
  tone: { target: 'professional', forbidden: [] },
  knowledge_sources: [],
  escalation_triggers: ['refund requested to an unverified account'],
};

const toolMetadata = {
  tool: 'issue_refund',
  side_effect: 'api_mutation',
  reversible: false,
  params: [
    {
      path: 'account',
      role: 'authority_bearing',
      allowed_sources: [{ origin: 'tool', kind: 'account_registry' }],
    },
  ],
  enabled: true,
};

function headers(contentType: string): Record<string, string> {
  const h: Record<string, string> = { 'content-type': contentType };
  if (API_KEY) h.authorization = `Bearer ${API_KEY}`;
  // Match createClient(): only scope by workspace when explicitly set, so setup
  // and the runtime /v1/events calls resolve to the same (default) workspace.
  if (WORKSPACE_ID) h['x-tlg-workspace-id'] = WORKSPACE_ID;
  return h;
}

async function request(method: string, path: string, contentType: string, body?: string) {
  const res = await fetch(`${SERVER_URL}${path}`, { method, headers: headers(contentType), body });
  const text = await res.text().catch(() => '');
  return { ok: res.ok, status: res.status, text };
}

async function expectOk(method: string, path: string, contentType: string, body?: string): Promise<string> {
  const res = await request(method, path, contentType, body);
  if (!res.ok) throw new Error(`${method} ${path} -> ${res.status} ${res.text}`);
  return res.text;
}

async function generateContentGuardrails(): Promise<void> {
  const res = await request('POST', `/v1/agents/${AGENT_ID}/guardrails/generate`, 'application/json');
  if (!res.ok) {
    process.stdout.write(
      `  ⚠ guardrails:generate skipped (${res.status}) — needs an LLM configured on the server. ` +
        `Action guard below still installs.\n`,
    );
    return;
  }

  const generated: GuardrailGenerateResponse = JSON.parse(res.text);
  if (generated.generated.length === 0) {
    process.stdout.write('  · guardrails:generate returned no policies\n');
    return;
  }

  for (const policy of generated.generated) {
    await expectOk(
      'PATCH',
      `/v1/policies/${encodeURIComponent(policy.id)}/enabled`,
      'application/json',
      JSON.stringify({ enabled: true }),
    );
    process.stdout.write(`  ✓ guardrail enabled: ${policy.id} (${policy.severity})\n`);
  }
}

async function main(): Promise<void> {
  process.stdout.write(`Provisioning NorthPay Disputes on ${SERVER_URL}\n`);

  // 1. Register the agent with its system prompt.
  await expectOk('POST', '/v1/agents', 'application/json', JSON.stringify(agentProfile));
  process.stdout.write(`  ✓ agent registered: ${AGENT_ID} (with system_prompt)\n`);

  // 2. Auto-generate content guardrails from that prompt, then enable them.
  await generateContentGuardrails();

  // 3. Register the action guard (what a prompt can't express).
  await expectOk('POST', '/v1/tool-metadata', 'application/json', JSON.stringify(toolMetadata));
  process.stdout.write(`  ✓ tool metadata: issue_refund (account = authority-bearing, registry-only)\n`);

  process.stdout.write(
    `\nDone — content guardrails from the prompt + the refund action guard are live.\n` +
      `Next: pnpm --filter @trustloopguard/demo dispute:serve, then attack :9202.\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`dispute setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});
