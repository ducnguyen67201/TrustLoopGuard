import { NextResponse } from 'next/server';

import { errorResponse } from '../../../_shared';
import { rustApiForAuthorizedWorkspace } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function GET(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  try {
    const url = new URL(req.url);
    const limit = url.searchParams.get('limit');
    const query = limit === null ? '' : `?limit=${encodeURIComponent(limit)}`;
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
      `/v1/runs/${encodeURIComponent(id)}/traces${query}`,
    );
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}
