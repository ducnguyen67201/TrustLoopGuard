import { NextResponse } from 'next/server';

import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface MemberListResponse {
  members: unknown[];
}

export async function GET(req: Request) {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const data = await rustApiForWorkspace<MemberListResponse>(workspaceId, '/v1/team/members');
    return NextResponse.json(data);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
