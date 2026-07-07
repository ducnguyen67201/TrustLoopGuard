import { NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function GET(req: Request, context: RouteContext): Promise<NextResponse> {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/financial/receipts/${encodeURIComponent(id)}`);
}
