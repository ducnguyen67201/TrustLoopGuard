import { NextResponse } from 'next/server';
import { tlClientForRequest, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

interface RouteContext {
  params: Promise<{ id: string }>;
}

/**
 * `POST /api/agents/{id}/redteam/plan` — derive tailored attack vectors from the
 * agent's own definition. Thin proxy: planning + workflow analysis are owned by
 * Rust.
 */
export async function POST(req: Request, context: RouteContext) {
  const { id } = await context.params;
  if (id.trim() === '') {
    return NextResponse.json({ error: 'agent id is required' }, { status: 400 });
  }

  let name: string | undefined;
  try {
    const body: unknown = await req.json();
    if (body !== null && typeof body === 'object' && 'name' in body) {
      const candidate = (body as { name?: unknown }).name;
      if (typeof candidate === 'string') name = candidate;
    }
  } catch {
    // No/invalid body — plan with a server-default name.
  }

  try {
    const result = await (
      await tlClientForRequest(req)
    ).planAttackVectors(id, name !== undefined ? { name } : {});
    return NextResponse.json(result);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
