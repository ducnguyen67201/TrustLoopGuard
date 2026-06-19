import { NextResponse } from 'next/server';
import { tlClientForRequest } from '@/lib/server/tl-client';
import { errorResponse } from '@/app/api/_shared';

export const runtime = 'nodejs';

interface RouteContext {
  params: Promise<{ id: string }>;
}

/**
 * `POST /api/agents/{id}/redteam/static-policies` — synthesize preventive
 * guardrails from the agent's workflow analyzer paths, attached `enabled=false`.
 * Thin proxy: synthesis is owned by Rust.
 */
export async function POST(req: Request, context: RouteContext) {
  const { id } = await context.params;
  if (id.trim() === '') {
    return NextResponse.json({ error: 'agent id is required' }, { status: 400 });
  }

  try {
    const result = await (await tlClientForRequest(req)).generateStaticPolicies(id);
    return NextResponse.json(result);
  } catch (err) {
    return errorResponse(err);
  }
}
