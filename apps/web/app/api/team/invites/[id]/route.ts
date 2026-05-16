import { NextResponse } from 'next/server';

import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface RouteParams {
  params: Promise<{ id: string }>;
}

export async function DELETE(req: Request, { params }: RouteParams) {
  try {
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const { id } = await params;
    await rustApiForWorkspace<unknown>(workspaceId, `/v1/team/invites/${encodeURIComponent(id)}`, {
      method: 'DELETE',
    });
    return new NextResponse(null, { status: 204 });
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
