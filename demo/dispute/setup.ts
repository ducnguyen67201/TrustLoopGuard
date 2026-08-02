import { API_KEY, SERVER_URL, WORKSPACE_ID } from '../shared/env';

// Amounts are integer minor units (cents). $500 = 50_000; $50_000 = 5_000_000.
const REFUND_CAP_CENTS = 50_000;
const WIRE_ESCALATE_CENTS = 5_000_000;

// Two money tools, each foregrounding a different control:
//   issue_refund   -> value_limit (amount cap) + parameter_auth (account source)
//   wire_transfer  -> approval (human gate) + value_limit on_breach=escalate
const tools = [
  {
    tool: 'issue_refund',
    side_effect: 'api_mutation',
    reversible: false,
    params: [
      {
        // Destination must come from the account registry, never from chat.
        path: 'account',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'tool', kind: 'account_registry' }],
      },
      {
        // Amount is payload (content_bearing) so parameter_auth skips it; the
        // value-limit checker caps it. min/max in integer cents block a refund
        // over $500 AND a zero/negative one before any payment fires; a
        // non-integer amount escalates as unverifiable.
        path: 'amount',
        role: 'content_bearing',
        limit: { min: 1, max: REFUND_CAP_CENTS, on_breach: 'deny' },
      },
    ],
    enabled: true,
  },
  {
    tool: 'wire_transfer',
    side_effect: 'api_mutation',
    reversible: false,
    params: [
      {
        path: 'destination',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'tool', kind: 'account_registry' }],
      },
      {
        // Wires escalate over $50k for a human instead of hard-blocking.
        path: 'amount',
        role: 'content_bearing',
        limit: { min: 1, max: WIRE_ESCALATE_CENTS, on_breach: 'require_approval' },
      },
    ],
    // Every wire needs a human in the loop, regardless of amount.
    approval: { required: true, approver_roles: ['treasury'], reason: 'Irreversible money movement.' },
    enabled: true,
  },
];

function headers(): Record<string, string> {
  const result: Record<string, string> = { 'content-type': 'application/json' };
  if (API_KEY) result.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) result['x-featherlane-ai-workspace-id'] = WORKSPACE_ID;
  return result;
}

// Checkers default to `off` per workspace. The scenarios only block/escalate
// when param + approval checkers are in `enforce`. Settings writes require an
// Owner/Admin user (an agent must never weaken its own controls), so we
// forward TL_USER_ID. Best-effort: warn loudly on failure rather than letting
// the demo run with controls silently off.
async function enforceModes(): Promise<void> {
  const userId = process.env.TL_USER_ID?.trim();
  if (userId === undefined || userId === '') {
    process.stderr.write(
      '⚠ TL_USER_ID not set — skipping checker-mode enforcement. The demo workspace must have\n' +
        '  param_checker_mode and approval_checker_mode = "enforce" (dashboard Settings or seed-demo),\n' +
        '  or every scenario will (wrongly) allow.\n',
    );
    return;
  }
  const res = await fetch(`${SERVER_URL}/v1/settings`, {
    method: 'PATCH',
    headers: { ...headers(), 'x-featherlane-ai-user-id': userId },
    body: JSON.stringify({ param_checker_mode: 'enforce', approval_checker_mode: 'enforce' }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    process.stderr.write(
      `⚠ could not set enforce modes (${res.status} ${text}). Enable param + approval checkers\n` +
        '  for this workspace in Settings, or the scenarios will allow everything.\n',
    );
    return;
  }
  process.stdout.write('enforcement modes: param + approval = enforce\n');
}

async function main(): Promise<void> {
  for (const tool of tools) {
    const res = await fetch(`${SERVER_URL}/v1/tool-metadata`, {
      method: 'POST',
      headers: headers(),
      body: JSON.stringify(tool),
    });
    const text = await res.text().catch(() => '');
    if (!res.ok) throw new Error(`POST /v1/tool-metadata (${tool.tool}) -> ${res.status} ${text}`);
    process.stdout.write(`installed ${tool.tool} tool metadata\n`);
  }
  await enforceModes();
}

main().catch((error) => {
  process.stderr.write(`dispute setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
