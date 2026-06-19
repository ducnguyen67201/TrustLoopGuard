import { NextResponse } from 'next/server';
import { tlClientForRequest } from '@/lib/server/tl-client';
import { errorResponse } from '@/app/api/_shared';

export const runtime = 'nodejs';

interface RouteContext {
  params: Promise<{ id: string }>;
}

/** `GET /api/agents/{id}/redteam/plans` — the agent's saved attack plans. */
export async function GET(req: Request, context: RouteContext) {
  const { id } = await context.params;
  if (id.trim() === '') {
    return NextResponse.json({ error: 'agent id is required' }, { status: 400 });
  }

  try {
    const result = await (await tlClientForRequest(req)).listPlans(id);
    return NextResponse.json(result);
  } catch (err) {
    return errorResponse(err);
  }
}
