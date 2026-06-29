import { API_KEY, SERVER_URL, WORKSPACE_ID } from '../shared/env';

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
    {
      // The refund amount is payload (content_bearing), so the
      // parameter-source checker ignores it; the value-limit checker caps
      // it. Bounds are in the demo's own unit — whole dollars, matching the
      // agent's `issue_refund(amount)`. `min`/`max` together block a refund
      // over $500 AND a zero/negative refund before any payment fires; a
      // non-integer amount (e.g. $49.99) escalates as unverifiable, by
      // design — a real integration should use integer minor units (cents).
      path: 'amount',
      role: 'content_bearing',
      limit: { min: 1, max: 500, on_breach: 'block' },
    },
  ],
  enabled: true,
};

function headers(): Record<string, string> {
  const result: Record<string, string> = { 'content-type': 'application/json' };
  if (API_KEY) result.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) result['x-tlg-workspace-id'] = WORKSPACE_ID;
  return result;
}

async function main(): Promise<void> {
  const res = await fetch(`${SERVER_URL}/v1/tool-metadata`, {
    method: 'POST',
    headers: headers(),
    body: JSON.stringify(toolMetadata),
  });
  const text = await res.text().catch(() => '');
  if (!res.ok) throw new Error(`POST /v1/tool-metadata -> ${res.status} ${text}`);
  process.stdout.write('installed issue_refund tool metadata\n');
}

main().catch((error) => {
  process.stderr.write(`dispute setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
