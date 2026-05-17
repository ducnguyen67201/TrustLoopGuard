import { NextResponse } from 'next/server';

import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  try {
    const url = new URL(req.url);
    const workspaceId = workspaceIdFromSlug(url.searchParams.get('workspace'));
    const limit = url.searchParams.get('limit');
    const query = limit === null ? '' : `?limit=${encodeURIComponent(limit)}`;
    const data = await rustApiForWorkspace<unknown>(
      workspaceId,
      `/v1/runs/${encodeURIComponent(id)}/traces${query}`,
    );
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

function errorResponse(err: unknown) {
  const message = err instanceof Error ? err.message : 'unknown error';
  return NextResponse.json({ error: message }, { status: 502 });
}
