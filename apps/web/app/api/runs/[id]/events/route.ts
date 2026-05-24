import { NextResponse } from 'next/server';

import { errorResponse, forwardedQuery } from '@/app/api/_shared';
import { rustApiForAuthorizedWorkspace } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function GET(req: Request, context: RouteContext) {
  try {
    const { id } = await context.params;
    const url = new URL(req.url);
    const rustQuery = forwardedQuery(url.searchParams);
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
      `/v1/runs/${encodeURIComponent(id)}/events${rustQuery}`,
    );
    return NextResponse.json(data);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function POST(req: Request, context: RouteContext) {
  try {
    const { id } = await context.params;
    const body = await req.text();
    const data = await rustApiForAuthorizedWorkspace<unknown>(
      req,
      `/v1/runs/${encodeURIComponent(id)}/events`,
      {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body,
      },
    );
    return NextResponse.json(data, { status: 201 });
  } catch (err) {
    return errorResponse(err);
  }
}
