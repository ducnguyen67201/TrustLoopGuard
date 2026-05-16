import { NextResponse } from 'next/server';

import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface InviteListResponse {
  invites: unknown[];
}

interface CreateInviteResponse {
  kind: 'added' | 'invited';
  member?: unknown;
  invite?: unknown;
}

export async function GET(req: Request) {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const data = await rustApiForWorkspace<InviteListResponse>(workspaceId, '/v1/team/invites');
    return NextResponse.json(data);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}

export async function POST(req: Request) {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const body = await req.text();
    const data = await rustApiForWorkspace<CreateInviteResponse>(workspaceId, '/v1/team/invites', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body,
    });
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
