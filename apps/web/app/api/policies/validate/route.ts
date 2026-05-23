import { NextResponse } from 'next/server';
import { tlClientForRequest, WorkspaceAccessError } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

export async function POST(req: Request) {
  const yaml = await req.text();
  if (yaml.trim() === '') {
    return NextResponse.json({ error: 'empty body' }, { status: 400 });
  }
  try {
    const result = await (await tlClientForRequest(req)).validatePolicy(yaml);
    return NextResponse.json(result);
  } catch (err) {
    if (err instanceof WorkspaceAccessError) {
      return NextResponse.json({ error: err.message }, { status: err.status });
    }
    const message = err instanceof Error ? err.message : 'unknown error';
    return NextResponse.json({ error: message }, { status: 502 });
  }
}
