import { NextResponse } from 'next/server';

import { rustApiForAuthorizedWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface RouteParams {
  params: Promise<{ id: string }>;
}

export async function DELETE(req: Request, { params }: RouteParams) {
  try {
    const { id } = await params;
    await rustApiForAuthorizedWorkspace<unknown>(
      req,
      `/v1/team/invites/${encodeURIComponent(id)}`,
      {
        method: 'DELETE',
      },
    );
    return new NextResponse(null, { status: 204 });
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
