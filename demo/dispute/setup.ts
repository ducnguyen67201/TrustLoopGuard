// dispute:setup — install the dispute demo tool metadata and guard policies.
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

const policies = [
  {
    id: 'refund-promise',
    yaml: `id: refund-promise
description: Rewrites unsupported refund promises.
when:
  channels: [chat]
match:
  any:
    - regex: "(?i)\\\\b(refund|money[- ]back|guarantee)\\\\b"
    - literal: "I promise"
action: rewrite
rewrite: "I'll connect you with a teammate who can review that for you."
severity: high
`,
  },
  {
    id: 'sensitive-identity-verification',
    yaml: `id: sensitive-identity-verification
description: Escalates requests for high-risk identity documents or secrets.
when:
  channels: [chat]
match:
  any:
    - regex: "(?i)\\\\b(ssn|social security|date of birth|dob)\\\\b"
action: escalate
severity: high
`,
  },
];

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

  for (const policy of policies) {
    const policyRes = await fetch(`${SERVER_URL}/v1/policies`, {
      method: 'POST',
      headers: headers('application/yaml'),
      body: policy.yaml,
    });
    const policyText = await policyRes.text().catch(() => '');
    if (!policyRes.ok) {
      throw new Error(`POST /v1/policies ${policy.id} -> ${policyRes.status} ${policyText}`);
    }
  }

  process.stdout.write(
    `  ✓ tool metadata: issue_refund (account = authority-bearing, registry-only)\n` +
      `  ✓ policies: refund-promise rewrite, sensitive-identity escalation\n\n` +
      `Next: pnpm --filter @trustloopguard/demo dispute:serve, then attack :9202.\n`,
  );
}

main().catch((error) => {
  process.stderr.write(`dispute setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exit(1);
});
