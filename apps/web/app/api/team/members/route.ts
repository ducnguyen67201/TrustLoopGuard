import { NextResponse } from 'next/server';

import { rustApiForAuthorizedWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface MemberListResponse {
  members: unknown[];
}

export async function GET(req: Request) {
  try {
    const data = await rustApiForAuthorizedWorkspace<MemberListResponse>(req, '/v1/team/members');
    return NextResponse.json(data);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
