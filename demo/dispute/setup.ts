// dispute:setup — install the ACTION guard for the dispute agent: tool metadata
// for issue_refund, marking `account` authority-bearing and only trustworthy
// from a verified account registry. This is the one protection a system prompt
// can't express, so it's registered explicitly.
//
// The content guardrails (PII / scope / no guaranteed-refund) come from the
// agent's prompt and are set up manually in the dashboard:
//   • register the agent with its system_prompt
//   • POST /v1/agents/{id}/guardrails/generate → review → enable
//
// This script does only the action guard:  POST /v1/tool-metadata
//   pnpm --filter @trustloopguard/demo dispute:setup
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

function headers(contentType: string): Record<string, string> {
  const h: Record<string, string> = { 'content-type': contentType };
  if (API_KEY) h.authorization = `Bearer ${API_KEY}`;
  // Match createClient(): only scope by workspace when explicitly set, so this
  // and the runtime /v1/events calls resolve to the same (default) workspace.
  if (WORKSPACE_ID) h['x-tlg-workspace-id'] = WORKSPACE_ID;
  return h;
}

async function main(): Promise<void> {
  process.stdout.write(`Installing issue_refund action guard on ${SERVER_URL}\n`);

  const res = await fetch(`${SERVER_URL}/v1/tool-metadata`, {
    method: 'POST',
    headers: headers('application/json'),
    body: JSON.stringify(toolMetadata),
  });
  const text = await res.text().catch(() => '');
  if (!res.ok) throw new Error(`POST /v1/tool-metadata -> ${res.status} ${text}`);

  process.stdout.write(
    `  ✓ tool metadata: issue_refund (account = authority-bearing, registry-only)\n\n` +
      `Next: pnpm --filter @trustloopguard/demo dispute:serve, then attack :9202.\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`dispute setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});
