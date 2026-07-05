import { API_KEY, SERVER_URL, WORKSPACE_ID } from '../shared/env';

const REFUND_CAP_CENTS = 50_000;

const tools = [
  {
    tool: 'ecommerce_issue_refund',
    side_effect: 'api_mutation',
    reversible: false,
    params: [
      {
        path: 'destination',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'tool', kind: 'order_registry' }],
      },
      {
        path: 'amount',
        role: 'content_bearing',
        limit: { min: 1, max: REFUND_CAP_CENTS, on_breach: 'block' },
      },
    ],
    enabled: true,
  },
  {
    tool: 'ecommerce_issue_store_credit',
    side_effect: 'api_mutation',
    reversible: false,
    params: [
      {
        path: 'destination',
        role: 'authority_bearing',
        allowed_sources: [{ origin: 'tool', kind: 'order_registry' }],
      },
      {
        path: 'amount',
        role: 'content_bearing',
      },
    ],
    approval: {
      required: true,
      approver_roles: ['support-lead'],
      reason: 'Store credit over pilot threshold needs human approval.',
    },
    enabled: true,
  },
];

function headers(): Record<string, string> {
  const result: Record<string, string> = { 'content-type': 'application/json' };
  if (API_KEY) result.authorization = `Bearer ${API_KEY}`;
  if (WORKSPACE_ID) result['x-tlg-workspace-id'] = WORKSPACE_ID;
  return result;
}

async function enforceModes(): Promise<void> {
  const userId = process.env.TL_USER_ID?.trim();
  if (userId === undefined || userId === '') {
    process.stderr.write(
      'TL_USER_ID not set - skipping checker-mode enforcement. The workspace must have\n' +
        'param_checker_mode and approval_checker_mode = "enforce", or every scenario may allow.\n',
    );
    return;
  }

  const res = await fetch(`${SERVER_URL}/v1/settings`, {
    method: 'PATCH',
    headers: { ...headers(), 'x-tlg-user-id': userId },
    body: JSON.stringify({ param_checker_mode: 'enforce', approval_checker_mode: 'enforce' }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => '');
    process.stderr.write(
      `could not set enforce modes (${res.status} ${text}). Enable param + approval checkers before running the pilot.\n`,
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
  process.stderr.write(`ecommerce setup failed: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
});
