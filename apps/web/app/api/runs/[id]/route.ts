import { NextResponse } from 'next/server';

import { RustApiError, rustApiForWorkspace, workspaceIdFromSlug } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  try {
    const workspaceId = workspaceIdFromSlug(new URL(req.url).searchParams.get('workspace'));
    const data = await rustApiForWorkspace<unknown>(
      workspaceId,
      `/v1/runs/${encodeURIComponent(id)}`,
    );
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function PATCH(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  try {
    const workspaceId = workspaceIdFromSlug(new URL(req.url).searchParams.get('workspace'));
    const body = await req.text();
    const data = await rustApiForWorkspace<unknown>(
      workspaceId,
      `/v1/runs/${encodeURIComponent(id)}`,
      {
        method: 'PATCH',
        headers: { 'Content-Type': 'application/json' },
        body,
      },
    );
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

function errorResponse(err: unknown) {
  if (err instanceof RustApiError) {
    return upstreamErrorResponse(err);
  }
  const message = err instanceof Error ? err.message : 'unknown error';
  return NextResponse.json({ error: message }, { status: 502 });
}

function upstreamErrorResponse(err: RustApiError) {
  if (err.body.trim() !== '') {
    try {
      const body: unknown = JSON.parse(err.body);
      return NextResponse.json(body, { status: err.status });
    } catch {
      return new NextResponse(err.body, { status: err.status });
    }
  }
  return NextResponse.json({ error: err.message }, { status: err.status });
}
