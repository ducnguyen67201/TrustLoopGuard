import { NextResponse } from 'next/server';

import { proxyRustJson } from '@/app/api/_shared';

export const runtime = 'nodejs';

type RouteContext = {
  params: Promise<{ id: string }>;
};

/** `POST /api/redteam/jobs/{id}/cancel` — cooperatively cancel a job. */
export async function POST(req: Request, context: RouteContext): Promise<NextResponse> {
  const { id } = await context.params;
  return proxyRustJson(req, `/v1/redteam/jobs/${encodeURIComponent(id)}/cancel`, {
    method: 'POST',
  });
}
