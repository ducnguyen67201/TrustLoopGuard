import { NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

/** `POST /api/redteam/regressions/run` — dispatch promoted cases as a job. */
export async function POST(req: Request): Promise<NextResponse> {
  return proxyRustJson(req, '/v1/redteam/regressions/run', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: await req.text(),
  });
}
