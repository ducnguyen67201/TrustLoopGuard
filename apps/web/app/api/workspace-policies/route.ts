import { NextResponse } from 'next/server';
import { z } from 'zod';

import { getDb } from '@/lib/db/client';
import { runtimePolicies, type RuntimePolicyDocument } from '@/lib/db/schema/workspace';
import { draftToYaml, policyDraftSchema } from '@/lib/policy-draft';
import { getDashboardShell } from '@/lib/server/dashboard-data';

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
  const { draft } = parsed.data;
  const agentId =
    parsed.data.agentId === undefined || parsed.data.agentId === null || parsed.data.agentId === ''
      ? null
      : parsed.data.agentId;
  const enabled = parsed.data.enabled ?? true;
  const now = new Date();
  const sourceYaml = draftToYaml(draft);
  const parsedPolicy = toRuntimePolicy(draft, agentId);
  const [policy] = await getDb()
    .insert(runtimePolicies)
    .values({
      workspaceId: shell.activeWorkspace.id,
      id: draft.id,
      ownerAgentId: agentId,
      enabled,
      policyYaml: sourceYaml,
      parsedPolicy,
      updatedAt: now,
      deletedAt: null,
    })
    .onConflictDoUpdate({
      target: [runtimePolicies.workspaceId, runtimePolicies.id],
      set: {
        ownerAgentId: agentId,
        enabled,
        policyYaml: sourceYaml,
        parsedPolicy,
        updatedAt: now,
        deletedAt: null,
      },
    })
    .returning({ id: runtimePolicies.id });

  return NextResponse.json({ policyId: policy?.id ?? draft.id });
}

function toRuntimePolicy(
  draft: z.infer<typeof policyDraftSchema>,
  ownerAgentId: string | null,
): RuntimePolicyDocument {
  return {
    id: draft.id,
    description: draft.description,
    match: { [draft.matchType]: draft.matchValue },
    action: draft.action,
    severity: draft.severity,
    ...(draft.rewrite ? { rewrite: draft.rewrite } : {}),
    ...(ownerAgentId ? { owner_agent_id: ownerAgentId } : {}),
  };
}
