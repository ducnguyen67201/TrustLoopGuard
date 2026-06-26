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
