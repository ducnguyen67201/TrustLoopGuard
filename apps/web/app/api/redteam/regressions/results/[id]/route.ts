import { NextResponse } from 'next/server';

import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

/** `GET /api/redteam/regressions/results/{id}` — summarize one regression job. */
export async function GET(req: Request, context: RouteContext): Promise<NextResponse> {
  const { id } = await context.params;
  const query = forwardedQuery(new URL(req.url).searchParams);
  return proxyRustJson(req, `/v1/redteam/regressions/results/${encodeURIComponent(id)}${query}`);
}
