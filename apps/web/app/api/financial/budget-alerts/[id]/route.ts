import type { NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

export async function PATCH(req: Request, context: RouteContext): Promise<NextResponse> {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/financial/budget-alerts/${encodeURIComponent(id)}`, {
    method: 'PATCH',
    headers: { 'Content-Type': 'application/json' },
    body: await req.text(),
  });
}

export async function DELETE(req: Request, context: RouteContext): Promise<NextResponse> {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/financial/budget-alerts/${encodeURIComponent(id)}`, {
    method: 'DELETE',
  });
}
