import { type NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

interface RouteContext {
  params: Promise<{ token: string }>;
}

export async function POST(req: Request, context: RouteContext): Promise<NextResponse> {
  const { token } = await context.params;
  return proxyRustJson(req, `/v1/redteam/reports/${encodeURIComponent(token)}/revoke`, {
    method: 'POST',
  });
}
