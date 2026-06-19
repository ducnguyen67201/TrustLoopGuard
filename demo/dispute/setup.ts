// dispute:setup — provision the platform (the source of truth) so the guarded
// agent's refund actions are actually evaluated by the engine. Registers two
// things via the HTTP API (NOT a local file):
//
//   1. the agent profile               POST /v1/agents        (YAML)
//   2. tool metadata for issue_refund  POST /v1/tool-metadata  (JSON)
//
// The tool metadata is what makes the block happen: it marks the refund's
// `account` parameter as authority-bearing and only allows it from a verified
// account registry. The engine's parameter-authorization checker reads this and
// blocks any refund whose `account` traces (by provenance) to the conversation.
//
// Run once against a running tl-server, then `pnpm dispute:serve` and attack.
//   pnpm --filter @trustloopguard/demo dispute:setup
import { API_KEY, DEFAULT_AGENT_ID, SERVER_URL, WORKSPACE_ID } from '../shared/env';

const AGENT_ID = process.env.TL_AGENT_ID ?? DEFAULT_AGENT_ID;

function headers(contentType: string): Record<string, string> {
  const h: Record<string, string> = { 'content-type': contentType };
  if (API_KEY) h.authorization = `Bearer ${API_KEY}`;
  // Match createClient(): only scope by workspace when explicitly set, so this
  // and the runtime /v1/events calls resolve to the same (default) workspace.
  if (WORKSPACE_ID) h['x-tlg-workspace-id'] = WORKSPACE_ID;
  return h;
}

const agentProfileYaml = `agent_id: ${AGENT_ID}
display_name: NorthPay Disputes
scope:
  in_scope:
    - payment dispute handling
    - refund processing
  out_of_scope:
    - tax preparation
authority:
  can_promise:
    - open a dispute case for review
  cannot_promise:
    - guaranteed refund approval
tone:
  target: professional
  forbidden: []
knowledge_sources: []
escalation_triggers:
  - refund requested to an unverified account
`;

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

async function post(path: string, contentType: string, body: string): Promise<void> {
  const res = await fetch(`${SERVER_URL}${path}`, {
    method: 'POST',
    headers: headers(contentType),
    body,
  });
  const text = await res.text().catch(() => '');
  if (!res.ok) throw new Error(`POST ${path} -> ${res.status} ${text}`);
  process.stdout.write(`  ✓ ${path} (${res.status})\n`);
}

async function main(): Promise<void> {
  process.stdout.write(`Provisioning NorthPay Disputes on ${SERVER_URL}\n`);
  await post('/v1/agents', 'application/yaml', agentProfileYaml);
  await post('/v1/tool-metadata', 'application/json', JSON.stringify(toolMetadata));
  process.stdout.write(
    `\nDone — the platform now owns the agent profile and the issue_refund rule.\n` +
      `Next: pnpm --filter @trustloopguard/demo dispute:serve, then attack :9202.\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`dispute setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});
