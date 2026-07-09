import type { NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ model: string[] }>;
};

export async function PUT(req: Request, context: RouteContext): Promise<NextResponse> {
  const { model } = await context.params;
  return proxyRustJson(req, `/v1/llm-pricing/${encodeURIComponent(model.join('/'))}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: await req.text(),
  });
}
