import { NextResponse } from 'next/server';

import { RustApiError, rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

export async function PATCH(
  req: Request,
  { params }: { params: Promise<{ id: string }> },
) {
  try {
    const { id } = await params;
    const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
    const workspaceId = workspaceIdFromSlug(workspaceSlug);
    const data = await rustApiForWorkspace<unknown>(
      workspaceId,
      `/v1/gateway/routes/${encodeURIComponent(id)}`,
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body: await req.text(),
      },
    );
    return NextResponse.json(data);
  } catch (err) {
    if (err instanceof RustApiError) {
      const status = err.status >= 500 ? 502 : err.status;
      try {
        return NextResponse.json(JSON.parse(err.body), { status });
      } catch {
        return NextResponse.json({ error: 'upstream error' }, { status });
      }
    }
    return NextResponse.json({ error: 'upstream error' }, { status: 502 });
  }
}
