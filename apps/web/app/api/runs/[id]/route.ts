import { NextResponse } from 'next/server';

import { errorResponse } from '@/app/api/_shared';
import { rustApiForAuthorizedWorkspace } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  try {
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
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
    const body = await req.text();
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
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
