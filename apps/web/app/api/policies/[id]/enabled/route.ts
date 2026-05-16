import { NextResponse } from 'next/server';
import { z } from 'zod';
import { tlClientForRequest } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

const bodySchema = z.object({ enabled: z.boolean() });

interface Ctx {
  params: Promise<{ id: string }>;
}

export async function PATCH(req: Request, ctx: Ctx) {
  const { id } = await ctx.params;
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return NextResponse.json({ error: 'invalid JSON' }, { status: 400 });
  }
  const parsed = bodySchema.safeParse(body);
  if (!parsed.success) {
    return NextResponse.json({ error: 'expected { enabled: boolean }' }, { status: 400 });
  }
  try {
    const doc = await (await tlClientForRequest(req)).setPolicyEnabled(id, parsed.data.enabled);
    return NextResponse.json(doc);
  } catch (err) {
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
