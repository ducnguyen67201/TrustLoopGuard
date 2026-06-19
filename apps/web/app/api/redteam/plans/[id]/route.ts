import { NextResponse } from 'next/server';
import { tlClientForRequest } from '@/lib/server/tl-client';
import { errorResponse } from '@/app/api/_shared';

export const runtime = 'nodejs';

interface RouteContext {
  params: Promise<{ id: string }>;
}

/** `DELETE /api/redteam/plans/{id}` — delete a saved attack plan. */
export async function DELETE(req: Request, context: RouteContext) {
  const { id } = await context.params;
  if (id.trim() === '') {
    return NextResponse.json({ error: 'plan id is required' }, { status: 400 });
  }

  try {
    await (await tlClientForRequest(req)).deletePlan(id);
    return new NextResponse(null, { status: 204 });
  } catch (err) {
    return errorResponse(err);
  }
}
