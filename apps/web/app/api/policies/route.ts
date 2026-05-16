import { NextResponse } from 'next/server';
import { tlClientForRequest } from '@/lib/server/tl-client';

export const runtime = 'nodejs';

export async function GET(req: Request) {
  try {
    const agentId = new URL(req.url).searchParams.get('agentid')?.trim();
    const client = await tlClientForRequest(req);
    const result =
      agentId !== undefined && agentId !== ''
        ? await client.listGuardrails(agentId)
        : await client.listPolicies();
    return NextResponse.json(result);
  } catch (err) {
    return errorResponse(err);
  }
}

export async function POST(req: Request) {
  const yaml = await req.text();
  if (yaml.trim() === '') {
    return NextResponse.json({ error: 'empty body' }, { status: 400 });
  }
  try {
    const doc = await (await tlClientForRequest(req)).upsertPolicy(yaml);
    return NextResponse.json(doc);
  } catch (err) {
    return errorResponse(err);
  }
}

function errorResponse(err: unknown) {
  const message = err instanceof Error ? err.message : 'unknown error';
  return NextResponse.json({ error: message }, { status: 502 });
}
