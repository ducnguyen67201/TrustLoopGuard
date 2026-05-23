import { NextResponse } from 'next/server';
import { tlClientForRequest, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface RouteContext {
  params: Promise<{ id: string }>;
}

export async function POST(req: Request, context: RouteContext) {
  const { id } = await context.params;
  if (id.trim() === '') {
    return NextResponse.json({ error: 'agent id is required' }, { status: 400 });
  }

  try {
    const result = await (await tlClientForRequest(req)).generateGuardrails(id);
    return NextResponse.json(result);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
