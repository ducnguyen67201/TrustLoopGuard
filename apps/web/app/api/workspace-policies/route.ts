import { NextResponse } from 'next/server';
import { z } from 'zod';

import { draftToYaml, policyDraftSchema } from '@/lib/policy-draft';
import { getDashboardShell } from '@/lib/server/dashboard-data';
import { rustApiForUserWorkspace } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const requestSchema = z.object({
  workspace: z.string().trim().optional(),
  draft: policyDraftSchema,
  agentId: z.string().trim().nullable().optional(),
  enabled: z.boolean().optional(),
});

export async function POST(req: Request) {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'invalid JSON body' }, { status: 400 });
  }

  const parsed = requestSchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json(
      { error: parsed.error.issues[0]?.message ?? 'bad request' },
      { status: 400 },
    );
  }

  const shell = await getDashboardShell(parsed.data.workspace);
  const role = shell.activeWorkspace.role.toLowerCase();
  if (role !== 'owner' && role !== 'admin') {
    return NextResponse.json(
      { error: 'workspace owner or admin role is required to create policies' },
      { status: 403 },
    );
  }
  const { draft } = parsed.data;
  const agentId =
    parsed.data.agentId === undefined || parsed.data.agentId === null || parsed.data.agentId === ''
      ? null
      : parsed.data.agentId;
  const enabled = parsed.data.enabled ?? true;
  const sourceYaml = withOwnerAgent(draftToYaml(draft), agentId);
  await rustApiForUserWorkspace(shell.user, shell.activeWorkspace.id, '/v1/policies', {
    method: 'POST',
    headers: { 'content-type': 'application/yaml' },
    body: sourceYaml,
  });
  if (!enabled) {
    await rustApiForUserWorkspace(
      shell.user,
      shell.activeWorkspace.id,
      `/v1/policies/${encodeURIComponent(draft.id)}/enabled`,
      {
        method: 'PATCH',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ enabled: false }),
      },
    );
  }

  return NextResponse.json({ policyId: draft.id });
}

function withOwnerAgent(sourceYaml: string, ownerAgentId: string | null): string {
  if (!ownerAgentId) return sourceYaml;
  return `${sourceYaml.trimEnd()}\nowner_agent_id: ${ownerAgentId}\n`;
}
