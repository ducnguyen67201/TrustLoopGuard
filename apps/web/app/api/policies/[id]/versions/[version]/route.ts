import { NextResponse } from 'next/server';
import { rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string; version: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id, version } = await ctx.params;
  const workspaceSlug = new URL(req.url).searchParams.get('workspace')?.trim();
  try {
    const result = await rustApiForWorkspace(
      workspaceIdFromSlug(workspaceSlug),
      `/v1/policies/${encodeURIComponent(id)}/versions/${encodeURIComponent(version)}`,
    );
    return NextResponse.json(result);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
