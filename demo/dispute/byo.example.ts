// Bring-your-own-agent: gate YOUR money tool on a TrustLoopGuard authorization effect.
//
// This is the whole integration: (1) register your tool's controls once, then
// (2) ask the guard before you execute and honor the effect. Copy this shape
// into your own agent. Run end-to-end against a local server with:
//   make server && pnpm --filter @trustloopguard/demo dispute:byo
//
// Prefer not to wrap each call yourself? Point your OpenAI/Anthropic client's
// baseURL at the gateway proxy instead — same effects, no per-call code.

import type { AuthorizationDecision, GuardEvent } from '@trustloopguard/sdk';

import { API_KEY, createClient, DEFAULT_AGENT_ID, SERVER_URL, WORKSPACE_ID } from '../shared/env';

function headers(): Record<string, string> {
  const result: Record<string, string> = { 'content-type': 'application/json' };
  if (API_KEY) result.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) result['x-tlg-workspace-id'] = WORKSPACE_ID;
  return result;
}

// (1) Register your tool's controls once: cap the amount (cents), require the
// destination to come from your registry, and add a human gate if you want one.
async function registerTool(): Promise<void> {
  const res = await fetch(`${SERVER_URL}/v1/tool-metadata`, {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify({
      tool: 'send_payout',
      side_effect: 'api_mutation',
      reversible: false,
      params: [
        { path: 'destination', role: 'authority_bearing', allowed_sources: [{ origin: 'tool', kind: 'account_registry' }] },
        { path: 'amount', role: 'content_bearing', limit: { min: 1, max: 50_000, on_breach: 'deny' } },
      ],
      enabled: true,
    }),
  });
  if (!res.ok) throw new Error(`register send_payout -> ${res.status} ${await res.text()}`);
}

// (2) Before executing your real payment, ask the guard and honor the effect.
async function guardedPayout(amountCents: number, destination: string): Promise<void> {
  const client = createClient();
  const event: GuardEvent = {
    kind: 'tool.call.proposed',
    principal: { workspace_id: '', environment_id: '', agent_id: DEFAULT_AGENT_ID },
    action: { operation: 'send_payout', parameters: { amount: amountCents, destination }, side_effect: 'api_mutation' },
    sources: [{ id: 'account_registry', origin: 'tool', kind: 'account_registry', labels: { trust: 'trusted', confidentiality: 'unknown', integrity: 'high' } }],
    provenance: { amount: ['account_registry'], destination: ['account_registry'] },
    context: { product: 'your agent' },
  };
  const decision: AuthorizationDecision = await client.submitEvent(event);
  if (decision.effect === 'permit') {
    // await yourRealPaymentApi(amountCents, destination);
    process.stdout.write(`permit   → would pay ${amountCents}¢ to ${destination}\n`);
  } else {
    process.stdout.write(`${decision.effect.padEnd(18)} → stopped: ${decision.reason}\n`);
  }
}

async function main(): Promise<void> {
  await registerTool();
  await guardedPayout(5_000, 'acct_registry_001'); // within cap -> permit
  await guardedPayout(900_000, 'acct_registry_001'); // over $500 cap -> deny
}

main().catch((error) => {
  process.stderr.write(`byo example failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
