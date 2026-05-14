import { NextResponse } from 'next/server';
import { z } from 'zod';
import { tlClient } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const requestSchema = z.object({
  prompt: z.string().trim().min(3, 'describe the policy in a few words'),
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

  try {
    const response = await tlClient().draftPolicy(parsed.data.prompt);
    return NextResponse.json({ draft: toCamelDraft(response.draft) });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}

/**
 * tl-server returns the draft with snake_case fields (Rust convention).
 * The UI's policyDraftSchema uses camelCase. Translate at the boundary
 * so the client zod schema is the source of truth for the dialog shape.
 */
function toCamelDraft(draft: {
  id: string;
  description: string;
  match_type: string;
  match_value: string;
  action: string;
  severity: string;
  rewrite?: string | null;
}) {
  return {
    id: draft.id,
    description: draft.description,
    matchType: draft.match_type,
    matchValue: draft.match_value,
    action: draft.action,
    severity: draft.severity,
    ...(draft.rewrite ? { rewrite: draft.rewrite } : {}),
  };
}
