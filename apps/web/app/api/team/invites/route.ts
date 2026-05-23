import { NextResponse } from 'next/server';

import { rustApiForAuthorizedWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

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
    const data = await rustApiForAuthorizedWorkspace<InviteListResponse>(req, '/v1/team/invites');
    return NextResponse.json(data);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}

export async function POST(req: Request) {
  try {
    const body = await req.text();
    const data = await rustApiForAuthorizedWorkspace<CreateInviteResponse>(
      req,
      '/v1/team/invites',
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      },
    );
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
