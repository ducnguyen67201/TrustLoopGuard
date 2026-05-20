import { NextResponse } from 'next/server';
import { rustApiForAuthorizedWorkspace, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string; version: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id, version } = await ctx.params;
  try {
    const result = await rustApiForAuthorizedWorkspace(
      req,
      `/v1/policies/${encodeURIComponent(id)}/versions/${encodeURIComponent(version)}`,
    );
    return NextResponse.json(result);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
