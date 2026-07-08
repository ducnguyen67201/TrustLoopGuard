import { NextResponse } from 'next/server';

import { forwardedQuery, proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

/** `GET /api/redteam/regressions` — list promoted regression cases. */
export async function GET(req: Request): Promise<NextResponse> {
  const query = forwardedQuery(new URL(req.url).searchParams);
  return proxyRustJson(req, `/v1/redteam/regressions${query}`);
}
